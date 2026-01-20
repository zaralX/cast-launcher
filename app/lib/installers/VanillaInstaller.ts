import type {
    DownloadTask,
    MojangAssetIndexObject,
    MojangLibraryArtifact,
    MojangLibraryObject,
    MojangObject
} from "~/types/instance"
import {InstallerBase} from "./InstallerBase"
import {$fetch} from "ofetch";
import {path} from "@tauri-apps/api";
import {arch, platform} from "@tauri-apps/plugin-os";
import {exists, mkdir, readFile, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";

export class VanillaInstaller extends InstallerBase {
    private tasks: DownloadTask[] = []
    private versionPackage?: any
    private libs?: MojangLibraryObject[]

    protected override async prepare() {
        await super.prepare()
        this.emit({ stage: "prepare", message: "Подготовка Vanilla" })

        let versionPackage = null

        // Cached version package
        const versionPackageDir = await path.join(this.cacheDir!, "versions", `${this.instance.minecraftVersion}-vanilla`)
        if (!(await exists(versionPackageDir))) await mkdir(versionPackageDir, { recursive: true })
        const versionPackageFile = await path.join(versionPackageDir, "package.json")
        if (await exists(versionPackageFile)) {
            versionPackage = JSON.parse(await readTextFile(versionPackageFile))
        }

        if (!versionPackage) {
            const versionsManifest = await $fetch("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            const versionObject = versionsManifest.versions.find((v: any) => v.id == this.instance.minecraftVersion)
            const versionPackage = await $fetch(versionObject.url)
            await writeTextFile(versionPackageFile, JSON.stringify(versionPackage))
        }

        this.versionPackage = versionPackage
    }

    protected async download() {
        this.emit({ stage: "download", message: "Начало загрузки" })

        // Client.jar
        this.emit({ stage: "download", message: "Проверка client.jar" })
        const clientObject: MojangObject = this.versionPackage?.downloads?.client

        const clientTask: DownloadTask = {
            url: clientObject.url,
            destination: await path.join(this.minecraftDir!, "client.jar"),
            size: clientObject.size,
            verificationType: "sha1",
            hash: clientObject.sha1
        }

        await this.downloader.download([clientTask], (progress) => {
            this.emit({
                stage: "download",
                message: "Загрузка client.jar",
                progress: progress.percent,
            })
        })

        // Libraries
        this.emit({ stage: "download", message: "Проверка libraries" })
        this.libs = await this.getLibraries(this.versionPackage?.libraries)
        console.log(this.librariesDir!)
        console.log(this.libs)
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

        await this.downloader.download(librariesTasks, (progress) => {
            this.emit({
                stage: "download",
                type: 'single',
                message: "Загрузка библиотеки " + progress.name,
                progress: progress.percent,
            })
        }, (progress) => {
            this.emit({
                stage: "download",
                type: 'global',
                message: "Загрузка библиотек",
                progress: progress,
            })
        })

        // Assets
        this.emit({ stage: "download", message: "Проверка assets" })
        const assetIndex: MojangAssetIndexObject = this.versionPackage.assetIndex
        const assetIndexesDir = await path.join(this.assetsDir!, "indexes")
        if (!(await exists(assetIndexesDir))) await mkdir(assetIndexesDir)

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
        const assetIndexData = JSON.parse(await readTextFile(assetIndexFilePath))
        const assets = assetIndexData.objects

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

        await this.downloader.download(assetsTasks, (progress) => {
            this.emit({
                stage: "download",
                type: 'single',
                message: "Загрузка ассета " + progress.name,
                progress: progress.percent,
            })
        }, (progress) => {
            this.emit({
                stage: "download",
                type: 'global',
                message: "Загрузка ассетов",
                progress: progress,
            })
        })
    }

    private async getLibraries(rawLibraries: any[]): Promise<MojangLibraryObject[]> {
        const libs: MojangLibraryObject[] = []

        const os = platform();
        const architecture = arch();

        for (const lib of rawLibraries) {
            const rules = lib.rules

            if (!checkRules(rules, os.toLowerCase(), architecture.toLowerCase()))
                continue

            const nativeId =  lib?.natives?.[os]
            const native = lib?.downloads?.classifiers?.[nativeId]

            libs.push({
                ...lib.downloads.artifact,
                native: native
            } as MojangLibraryObject)
        }

        return libs
    }

    protected async installFiles() {
        this.emit({ stage: "install", message: "Установка Vanilla" })

        // Installing natives
        for (const lib of this.libs!.filter(lib => lib.native)) {
            console.log("installing native", lib.native)
            await this.installNative(lib.native!, this.nativesDir!)
            console.log("Native installed")
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
