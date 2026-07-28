import type {
    DownloadTask,
    InstallPhase,
    MojangAssetIndexObject,
    MojangObject
} from "~/types/instance"
import {InstallerBase} from "./InstallerBase"
import {path} from "@tauri-apps/api";
import {exists, mkdir, readFile, writeTextFile} from "@tauri-apps/plugin-fs";
import {LauncherError} from "~/types/error";
import {javaRequirementFromPackage} from "~/utils/javaUtils";
import {resolveLibraries, type ResolvedLibrary} from "~/utils/mojangUtils";

export class VanillaInstaller extends InstallerBase {
    private tasks: DownloadTask[] = []
    protected versionPackage?: any
    private libs?: ResolvedLibrary[]

    protected override definePhases(): InstallPhase[] {
        return [
            {key: "java", label: "Java", weight: 8},
            {key: "client", label: "Клиент", weight: 12},
            {key: "libraries", label: "Библиотеки", weight: 20},
            {key: "assets", label: "Ресурсы", weight: 58},
            {key: "natives", label: "Нативные библиотеки", weight: 10}
        ]
    }

    protected override async prepare() {
        await super.prepare()
        this.emit({ stage: "prepare", message: "Подготовка Vanilla" })

        // Cached version package
        const versionPackageDir = await path.join(this.cacheDir!, "versions", `${this.instance.minecraftVersion}-vanilla`)
        const versionPackageFile = await path.join(versionPackageDir, "package.json")

        let versionPackage = await this.readCachedJson(versionPackageFile)

        if (!versionPackage) {
            const versionsManifest = await this.fetchJson("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            const versionObject = versionsManifest?.versions?.find((v: any) => v.id == this.instance.minecraftVersion)

            if (!versionObject?.url) {
                throw new LauncherError("VERSION_NOT_FOUND", {
                    message: `Версия ${this.instance.minecraftVersion} отсутствует в манифесте Mojang`,
                    context: this.errorContext()
                })
            }

            versionPackage = await this.fetchJson(versionObject.url)

            try {
                if (!(await exists(versionPackageDir))) await mkdir(versionPackageDir, { recursive: true })
                await writeTextFile(versionPackageFile, JSON.stringify(versionPackage))
            } catch (e) {
                throw this.wrap(e, { path: versionPackageFile })
            }
        }

        this.versionPackage = versionPackage
    }

    protected override async ensureJava(): Promise<void> {
        this.beginPhase("java", "Проверка Java")

        await this.resolveJava(
            javaRequirementFromPackage(this.versionPackage, this.instance.minecraftVersion),
            {
                component: this.versionPackage?.javaVersion?.component,
                downloader: this.downloader,
                onFile: (file) => this.emitFile(file, "Java"),
                onProgress: (progress) => this.reportPhase(progress, "Загрузка Java")
            }
        )

        this.reportPhase(1)
    }

    protected async download() {
        this.emit({ stage: "download", message: "Начало загрузки" })

        await this.downloadClientJar()
        await this.downloadLibraries()
        await this.downloadAssets()
    }

    protected async downloadClientJar() {
        this.beginPhase("client", "Клиент Minecraft")
        const clientObject: MojangObject | undefined = this.versionPackage?.downloads?.client

        if (!clientObject?.url) {
            throw new LauncherError("MANIFEST_INVALID", {
                message: "В манифесте версии нет ссылки на client.jar",
                context: this.errorContext()
            })
        }

        const clientTask: DownloadTask = {
            url: clientObject.url,
            destination: await path.join(this.minecraftDir!, "client.jar"),
            size: clientObject.size,
            verificationType: "sha1",
            hash: clientObject.sha1
        }

        await this.downloader.download(
            [clientTask],
            (file) => this.emitFile(file, "Клиент"),
            (progress) => this.reportPhase(progress)
        )
    }

    protected async downloadLibraries() {
        this.beginPhase("libraries", "Библиотеки")
        this.libs = this.getLibraries(this.versionPackage?.libraries)

        const librariesTasks: DownloadTask[] = []

        for (const lib of this.libs) {
            for (const object of [lib.artifact, lib.native]) {
                if (!object) continue

                librariesTasks.push({
                    url: object.url,
                    destination: await path.join(this.librariesDir!, object.path),
                    size: object.size,
                    verificationType: "sha1",
                    hash: object.sha1
                })
            }
        }

        await this.downloader.download(
            librariesTasks,
            (file) => this.emitFile(file, "Библиотека"),
            (progress) => this.reportPhase(progress)
        )
    }

    protected async downloadAssets() {
        this.beginPhase("assets", "Ресурсы игры")
        const assetIndex: MojangAssetIndexObject | undefined = this.versionPackage?.assetIndex

        if (!assetIndex?.url || !assetIndex?.id) {
            throw new LauncherError("MANIFEST_INVALID", {
                message: "В манифесте версии нет индекса ассетов",
                context: this.errorContext()
            })
        }

        const assetIndexesDir = await path.join(this.assetsDir!, "indexes")

        try {
            if (!(await exists(assetIndexesDir))) await mkdir(assetIndexesDir, { recursive: true })
        } catch (e) {
            throw this.wrap(e, { path: assetIndexesDir })
        }

        const assetIndexFilePath = await path.join(assetIndexesDir, `${assetIndex.id}.json`)
        if (await exists(assetIndexFilePath)) {
            const fileData = await readFile(assetIndexFilePath);
            const fileHash = Array.from(new Uint8Array(
                await crypto.subtle.digest("SHA-1", fileData)
            ))
                .map(b => b.toString(16).padStart(2, "0"))
                .join("");
            if (fileHash != assetIndex.sha1) {
                await this.downloadJson(assetIndex.url, assetIndexFilePath)
            }
        } else { await this.downloadJson(assetIndex.url, assetIndexFilePath) }

        const assetIndexData = await this.readCachedJson(assetIndexFilePath)
        const assets = assetIndexData?.objects

        if (!assets) {
            throw new LauncherError("MANIFEST_INVALID", {
                message: "Индекс ассетов повреждён",
                context: this.errorContext({ path: assetIndexFilePath })
            })
        }

        const assetsTasks: DownloadTask[] = await Promise.all(Object.values(assets).map(async (asset: any) => {
            const folder = asset.hash.slice(0, 2)
            return {
                url: `https://resources.download.minecraft.net/${folder}/${asset.hash}`,
                destination: await path.join(this.assetsDir!, "objects", folder, asset.hash),
                size: asset.size,
                verificationType: "sha1",
                hash: asset.hash
            } as DownloadTask
        }))

        await this.downloader.download(
            assetsTasks,
            (file) => this.emitFile(file, "Ресурс"),
            (progress) => this.reportPhase(progress)
        )
    }

    private getLibraries(rawLibraries: any[] | undefined): ResolvedLibrary[] {
        if (!Array.isArray(rawLibraries)) {
            throw new LauncherError("MANIFEST_INVALID", {
                message: "В манифесте версии нет списка библиотек",
                context: this.errorContext()
            })
        }

        return resolveLibraries(rawLibraries)
    }

    protected async installFiles() {
        this.emit({ stage: "install", message: "Установка Vanilla" })
        this.beginPhase("natives", "Нативные библиотеки")

        // Installing natives
        const natives = this.libs!.flatMap(lib => lib.native ? [lib.native] : [])

        if (!natives.length) {
            this.reportPhase(1)
            return
        }

        for (const [index, native] of natives.entries()) {
            await this.installNative(native, this.nativesDir!)
            this.reportPhase((index + 1) / natives.length)
        }
    }

    protected override async finalize(): Promise<void> {
        await super.finalize();
    }

    private async resolveVanillaAssets(): Promise<DownloadTask[]> {
        // version_manifest.json → version.json → libraries/assets
        return []
    }
}
