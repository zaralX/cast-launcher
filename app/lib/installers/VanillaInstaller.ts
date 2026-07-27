import type {
    DownloadTask,
    InstallPhase,
    MojangAssetIndexObject,
    MojangLibraryArtifact,
    MojangLibraryObject,
    MojangObject
} from "~/types/instance"
import {InstallerBase} from "./InstallerBase"
import {path} from "@tauri-apps/api";
import {arch, platform} from "@tauri-apps/plugin-os";
import {exists, mkdir, readFile, writeTextFile} from "@tauri-apps/plugin-fs";
import {LauncherError} from "~/types/error";

export class VanillaInstaller extends InstallerBase {
    private tasks: DownloadTask[] = []
    protected versionPackage?: any
    private libs?: MojangLibraryObject[]

    protected override definePhases(): InstallPhase[] {
        return [
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
        this.libs = await this.getLibraries(this.versionPackage?.libraries)

        const librariesTasks: DownloadTask[] = await Promise.all(this.libs
            .filter(lib => lib.path)
            .map(async lib => ({
                url: lib.url,
                destination: await path.join(this.librariesDir!, lib.path),
                size: lib.size,
                verificationType: 'sha1',
                hash: lib.sha1
            } as DownloadTask)));

        // add natives tasks
        for (const lib of this.libs!.filter(lib => lib.native)) {
            console.log("Appended native " + lib.native?.path + " to download tasks")
            librariesTasks.push({
                url: lib.native!.url,
                destination: await path.join(this.librariesDir!, lib.native!.path),
                size: lib.native!.size,
                verificationType: 'sha1',
                hash: lib.native!.sha1,
            })
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

    private async getLibraries(rawLibraries: any[] | undefined): Promise<MojangLibraryObject[]> {
        if (!Array.isArray(rawLibraries)) {
            throw new LauncherError("MANIFEST_INVALID", {
                message: "В манифесте версии нет списка библиотек",
                context: this.errorContext()
            })
        }

        const libs: MojangLibraryObject[] = []

        const os = platform();
        const architecture = arch();

        for (const lib of rawLibraries) {
            const rules = lib?.rules

            if (!checkRules(rules, os.toLowerCase(), architecture.toLowerCase()))
                continue

            const nativeId = lib?.natives?.[os]
            const native = lib?.downloads?.classifiers?.[nativeId]

            libs.push({
                ...lib?.downloads?.artifact,
                native: native
            } as MojangLibraryObject)
        }

        return libs
    }

    protected async installFiles() {
        this.emit({ stage: "install", message: "Установка Vanilla" })
        this.beginPhase("natives", "Нативные библиотеки")

        // Installing natives
        const natives = this.libs!.filter(lib => lib.native)

        for (const [index, lib] of natives.entries()) {
            await this.installNative(lib.native!, this.nativesDir!)
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
