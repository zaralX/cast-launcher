import {invoke} from "@tauri-apps/api/core"
import {listen} from "@tauri-apps/api/event"
import {v4 as uuidv4} from "uuid"
import type {DownloadFileProgress, DownloadTask} from "~/types/instance"
import {toLauncherError} from "~/types/error"

export type FileProgress = DownloadFileProgress

export interface DownloadOptions {
    largeThreshold?: number
    deepVerify?: boolean
}

type JobState = "running" | "finished" | "cancelled" | "failed"

interface JobSnapshot {
    jobId: string
    status: { state: JobState, error?: { code: string, message: string, details?: string } }
    progress: number
    totalFiles: number
    doneFiles: number
    skippedFiles: number
    downloadedBytes: number
    totalBytes: number
    files: FileProgress[]
}

const PROGRESS_EVENT = "download:progress"

export async function listActiveDownloadJobs(): Promise<string[]> {
    try {
        const jobs = await invoke<JobSnapshot[]>("list_downloads")
        return jobs.filter(job => job.status.state === "running").map(job => job.jobId)
    } catch (e) {
        console.warn("Failed to list active downloads", e)
        return []
    }
}

export class ParallelDownloader {
    private options: DownloadOptions
    private jobKey: string | null = null
    private activeJobId: string | null = null
    private aborted = false

    constructor(options: DownloadOptions = {}) {
        this.options = options
    }

    /** Детерминированный ключ джобы — по нему происходит переподключение после перезагрузки. */
    setJob(key: string) {
        this.jobKey = key
    }

    abort() {
        this.aborted = true
        if (this.activeJobId) invoke("cancel_download", {jobId: this.activeJobId}).catch(() => {})
    }

    async downloadSingle(task: DownloadTask, onFileProgress?: (p: FileProgress) => void) {
        await this.download([task], onFileProgress)
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

        const jobId = this.jobKey ?? uuidv4()
        this.activeJobId = jobId

        const known = new Map<string, FileProgress>()

        const apply = (snapshot: JobSnapshot) => {
            if (snapshot.jobId !== jobId) return

            onGlobalProgress?.(snapshot.progress)

            if (!onFileProgress) return

            const seen = new Set<string>()

            for (const file of snapshot.files) {
                seen.add(file.url)
                known.set(file.url, file)
                onFileProgress(file)
            }

            for (const [url, file] of known) {
                if (seen.has(url)) continue
                known.delete(url)
                onFileProgress({...file, done: true})
            }
        }

        const unlisten = await listen<JobSnapshot>(PROGRESS_EVENT, e => apply(e.payload))

        try {
            const initial = await invoke<JobSnapshot>("start_download", {
                jobId,
                tasks,
                options: this.options
            })

            apply(initial)

            if (this.aborted) {
                await invoke("cancel_download", {jobId}).catch(() => {})
            }

            await invoke("await_download", {jobId})

            onGlobalProgress?.(1)
        } catch (e) {
            throw toLauncherError(e, "DOWNLOAD_FAILED", {})
        } finally {
            unlisten()
            this.activeJobId = null

            if (onFileProgress) {
                for (const [, file] of known) onFileProgress({...file, done: true})
                known.clear()
            }
        }
    }
}
