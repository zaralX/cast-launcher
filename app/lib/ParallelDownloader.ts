import { dirname } from "@tauri-apps/api/path"
import { exists, mkdir, readFile, writeFile } from "@tauri-apps/plugin-fs"
import { sha1 } from "hash-wasm"
import type { DownloadTask } from "~/types/instance"
import { fetch } from '@tauri-apps/plugin-http';
import { LauncherError, toLauncherError } from "~/types/error"

type FileProgress = {
    url: string
    name: string
    destination: string
    loaded: number
    total: number
    percent: number
}

export class ParallelDownloader {
    private concurrency: number

    constructor(concurrency = 3) {
        this.concurrency = concurrency
    }

    async downloadSingle(task: DownloadTask) {
        await this.downloadFile(task)
    }

    async download(
        tasks: DownloadTask[],
        onFileProgress?: (p: FileProgress) => void,
        onGlobalProgress?: (percent: number) => void
    ) {
        let globalLoaded = 0
        const globalTotal = tasks.reduce((a, t) => a + (t.size ?? 0), 0)

        const queue = [...tasks]

        const workers = Array.from({ length: this.concurrency }).map(async () => {
            while (queue.length) {
                const task = queue.shift()
                if (!task) return

                const loaded = await this.downloadFile(task, (chunk, total) => {
                    globalLoaded += chunk
                    onGlobalProgress?.(
                        globalTotal
                            ? globalLoaded / globalTotal
                            : 0
                    )
                }, onFileProgress)

                globalLoaded += loaded
            }
        })

        await Promise.all(workers)
    }

    private async downloadFile(
        task: DownloadTask,
        onChunk?: (chunkSize: number, total: number) => void,
        onFileProgress?: (p: FileProgress) => void
    ) {
        const context = { url: task.url, path: task.destination }

        // Проверка sha1
        if (task.verificationType === "sha1" && task.hash) {
            try {
                if (await exists(task.destination)) {
                    const data = await readFile(task.destination)
                    if (await sha1(data) === task.hash) return 0
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

        const total =
            task.size ??
            Number(response.headers.get("content-length")) ??
            0

        const reader = response.body.getReader()
        let received = 0
        const chunks: Uint8Array[] = []

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

            onChunk?.(value.length, total)

            onFileProgress?.({
                url: task.url,
                name: task.destination.split(/[\\/]/).pop() ?? "файл",
                destination: task.destination,
                loaded: received,
                total,
                percent: total ? received / total : 0
            })
        }

        // склеиваем
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

        return received
    }
}
