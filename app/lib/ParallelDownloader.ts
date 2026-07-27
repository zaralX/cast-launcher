import { dirname } from "@tauri-apps/api/path"
import { exists, mkdir, readFile, writeFile } from "@tauri-apps/plugin-fs"
import { sha1 } from "hash-wasm"
import type { DownloadFileProgress, DownloadTask } from "~/types/instance"
import { fetch } from '@tauri-apps/plugin-http';
import { LauncherError, toLauncherError } from "~/types/error"

export type FileProgress = DownloadFileProgress

/** Порог изменения, ниже которого прогресс не пересылается наружу (гасит дрожание UI). */
const GLOBAL_PROGRESS_EPS = 0.002
const FILE_PROGRESS_EPS = 0.01

export class ParallelDownloader {
    private concurrency: number

    constructor(concurrency = 3) {
        this.concurrency = concurrency
    }

    async downloadSingle(task: DownloadTask, onFileProgress?: (p: FileProgress) => void) {
        await this.downloadFile(task, undefined, onFileProgress)
    }

    async download(
        tasks: DownloadTask[],
        onFileProgress?: (p: FileProgress) => void,
        onGlobalProgress?: (percent: number) => void
    ) {
        if (!tasks.length) {
            onGlobalProgress?.(1)
            return
        }

        const useBytes = tasks.every(t => (t.size ?? 0) > 0)
        const weights = tasks.map(t => (useBytes ? (t.size as number) : 1))
        const totalWeight = weights.reduce((a, w) => a + w, 0) || 1

        const fractions = new Array<number>(tasks.length).fill(0)
        let accumulated = 0
        let lastReported = -1

        const setFraction = (index: number, value: number) => {
            const clamped = Math.min(1, Math.max(fractions[index]!, value))
            accumulated += (clamped - fractions[index]!) * weights[index]!
            fractions[index] = clamped
        }

        const reportGlobal = (force = false) => {
            if (!onGlobalProgress) return
            const percent = Math.min(1, Math.max(0, accumulated / totalWeight))
            if (!force && percent - lastReported < GLOBAL_PROGRESS_EPS) return
            lastReported = percent
            onGlobalProgress(percent)
        }

        reportGlobal(true)

        const queue = tasks.map((task, index) => ({ task, index }))

        const workers = Array.from({ length: Math.min(this.concurrency, queue.length) }).map(async () => {
            while (queue.length) {
                const item = queue.shift()
                if (!item) return

                await this.downloadFile(item.task, (loaded, total) => {
                    if (total > 0) {
                        setFraction(item.index, loaded / total)
                        reportGlobal()
                    }
                }, onFileProgress)

                setFraction(item.index, 1)
                reportGlobal(true)
            }
        })

        await Promise.all(workers)

        reportGlobal(true)
    }

    private async downloadFile(
        task: DownloadTask,
        onChunk?: (loaded: number, total: number) => void,
        onFileProgress?: (p: FileProgress) => void
    ) {
        const context = { url: task.url, path: task.destination }
        const name = task.destination.split(/[\\/]/).pop() ?? "файл"

        const reportFile = (loaded: number, total: number, done: boolean) => {
            onFileProgress?.({
                url: task.url,
                name,
                destination: task.destination,
                loaded,
                total,
                percent: total ? Math.min(1, loaded / total) : (done ? 1 : 0),
                done
            })
        }

        if (task.verificationType === "sha1" && task.hash) {
            try {
                if (await exists(task.destination)) {
                    const data = await readFile(task.destination)
                    if (await sha1(data) === task.hash) {
                        reportFile(data.length, task.size ?? data.length, true)
                        return 0
                    }
                }
            } catch (e) {
                console.warn("Failed to verify cached file, redownloading", task.destination, e)
            }
        }

        try {
            await mkdir(await dirname(task.destination), { recursive: true })
        } catch (e) {
            throw toLauncherError(e, "FS_ERROR", context)
        }

        let response: Response
        try {
            response = await fetch(task.url, {
                method: "GET",
            })
        } catch (e) {
            throw toLauncherError(e, "NETWORK", context)
        }

        if (!response.ok || !response.body) {
            throw new LauncherError("DOWNLOAD_FAILED", {
                message: `Сервер ответил HTTP ${response.status} на ${task.url}`,
                context
            })
        }

        const headerLength = Number(response.headers.get("content-length"))
        const total = task.size ?? (Number.isFinite(headerLength) && headerLength > 0 ? headerLength : 0)

        const reader = response.body.getReader()
        let received = 0
        let lastFilePercent = -1
        const chunks: Uint8Array[] = []

        reportFile(0, total, false)

        while (true) {
            let done: boolean
            let value: Uint8Array | undefined

            try {
                ({ done, value } = await reader.read())
            } catch (e) {
                throw toLauncherError(e, "DOWNLOAD_FAILED", context)
            }

            if (done || !value) break

            received += value.length
            chunks.push(value)

            onChunk?.(received, total)

            const percent = total ? Math.min(1, received / total) : 0
            if (percent - lastFilePercent >= FILE_PROGRESS_EPS) {
                lastFilePercent = percent
                reportFile(received, total, false)
            }
        }

        const buffer = new Uint8Array(received)
        let offset = 0
        for (const chunk of chunks) {
            buffer.set(chunk, offset)
            offset += chunk.length
        }

        if (task.verificationType === "sha1" && task.hash) {
            const actual = await sha1(buffer)
            if (actual !== task.hash) {
                throw new LauncherError("HASH_MISMATCH", {
                    message: `Контрольная сумма не совпала: ${task.url}`,
                    details: `Ожидалось: ${task.hash}\nПолучено:  ${actual}`,
                    context
                })
            }
        }

        try {
            await writeFile(task.destination, buffer)
        } catch (e) {
            throw toLauncherError(e, "FS_ERROR", context)
        }

        reportFile(received, total || received, true)

        return received
    }
}
