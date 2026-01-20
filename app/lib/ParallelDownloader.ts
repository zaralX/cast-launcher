import { dirname } from "@tauri-apps/api/path"
import { exists, mkdir, readFile, writeFile } from "@tauri-apps/plugin-fs"
import { sha1 } from "hash-wasm"
import type { DownloadTask } from "~/types/instance"
import { fetch } from '@tauri-apps/plugin-http';

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
        // Проверка sha1
        if (task.verificationType === "sha1" && task.hash) {
            if (await exists(task.destination)) {
                const data = await readFile(task.destination)
                if (await sha1(data) === task.hash) return 0
            }
        }

        await mkdir(await dirname(task.destination), { recursive: true })

        const response = await fetch(task.url, {
            method: "GET",
        })
        if (!response.ok || !response.body) {
            throw new Error(`DOWNLOAD_FAILED: ${task.url}`)
        }

        const total =
            task.size ??
            Number(response.headers.get("content-length")) ??
            0

        const reader = response.body.getReader()
        let received = 0
        const chunks: Uint8Array[] = []

        while (true) {
            const { done, value } = await reader.read()
            if (done) break

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
            if (await sha1(buffer) !== task.hash) {
                throw new Error(`HASH_MISMATCH: ${task.url}`)
            }
        }

        await writeFile(task.destination, buffer)

        return received
    }
}
