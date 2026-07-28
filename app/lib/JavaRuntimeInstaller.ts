import {path} from "@tauri-apps/api";
import {invoke} from "@tauri-apps/api/core";
import {arch, platform} from "@tauri-apps/plugin-os";
import {$fetch} from "ofetch";
import {ParallelDownloader} from "~/lib/ParallelDownloader";
import type {DownloadFileProgress, DownloadTask} from "~/types/instance";
import {toLauncherError} from "~/types/error";

export const JAVA_RUNTIME_META_URL =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json"

interface RuntimeEntry {
    version?: { name?: string }
    manifest?: { url?: string }
}

interface ManifestEntry {
    type: "file" | "directory" | "link"
    executable?: boolean
    target?: string
    downloads?: { raw?: { url: string, sha1: string, size: number } }
}

export interface JavaRuntimeInstallOptions {
    onFile?: (file: DownloadFileProgress) => void
    onProgress?: (percent: number) => void
    downloader?: ParallelDownloader
}

export interface JavaResolveOptions extends JavaRuntimeInstallOptions {
    component?: string
}

function mojangOsKey(): string | null {
    const cpu = arch()

    switch (platform()) {
        case "windows":
            if (cpu === "aarch64") return "windows-arm64"
            if (cpu === "x86") return "windows-x86"
            return "windows-x64"
        case "macos":
            return cpu === "aarch64" ? "mac-os-arm64" : "mac-os"
        case "linux":
            if (cpu === "x86") return "linux-i386"
            return cpu === "x86_64" ? "linux" : null
        default:
            return null
    }
}

async function fetchJson<T>(url: string): Promise<T> {
    try {
        return await $fetch<T>(url)
    } catch (e) {
        throw toLauncherError(e, "NETWORK", {url})
    }
}

export async function installJavaRuntime(
    component: string,
    targetDir: string,
    options: JavaRuntimeInstallOptions = {}
): Promise<string | null> {
    const osKey = mojangOsKey()
    if (!osKey) return null

    const all = await fetchJson<Record<string, Record<string, RuntimeEntry[]>>>(JAVA_RUNTIME_META_URL)

    const entry = all?.[osKey]?.[component]?.[0]
    if (!entry?.manifest?.url) return null

    const manifest = await fetchJson<{ files?: Record<string, ManifestEntry> }>(entry.manifest.url)

    const tasks: DownloadTask[] = []
    const executables: string[] = []
    const links: { path: string, target: string }[] = []

    for (const [relative, file] of Object.entries(manifest.files ?? {})) {
        if (file.type === "link") {
            if (file.target) links.push({path: relative, target: file.target})
            continue
        }

        if (file.type !== "file") continue

        const raw = file.downloads?.raw
        if (!raw?.url) continue

        tasks.push({
            url: raw.url,
            destination: await path.join(targetDir, ...relative.split("/")),
            size: raw.size,
            verificationType: "sha1",
            hash: raw.sha1
        })

        if (file.executable) executables.push(relative)
    }

    const downloader = options.downloader ?? new ParallelDownloader()
    await downloader.download(tasks, options.onFile, options.onProgress)

    await invoke("finalize_java_runtime", {root: targetDir, executables, links})

    return entry.version?.name ?? component
}
