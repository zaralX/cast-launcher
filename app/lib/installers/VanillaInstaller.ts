import type {DownloadTask, MojangAssetIndexObject, MojangLibraryObject, MojangObject} from "~/types/instance"
import {InstallerBase} from "./InstallerBase"
import {$fetch} from "ofetch";
import {path} from "@tauri-apps/api";
import {arch, platform} from "@tauri-apps/plugin-os";
import {exists, mkdir, readFile, readTextFile} from "@tauri-apps/plugin-fs";
import { invoke } from "@tauri-apps/api/core";

export class VanillaInstaller extends InstallerBase {
    private tasks: DownloadTask[] = []
    private versionPackage?: any

    protected override async prepare() {
        await super.prepare()
        this.emit({ stage: "prepare", message: "Подготовка Vanilla" })

        const versionsManifest = await $fetch("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        const versionObject = versionsManifest.versions.find((v: any) => v.id == this.instance.minecraftVersion)

        const versionPackage = await $fetch(versionObject.url)
        this.versionPackage = versionPackage
    }

    protected async download() {
        this.emit({ stage: "download", message: "Начало загрузки" })

        // Client.jar
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
        const libraries = await this.getLibraries(this.versionPackage?.libraries)
        const librariesTasks: DownloadTask[] = await Promise.all(libraries.map(async lib => ({
            url: lib.url,
            destination: await path.join(this.librariesDir!, lib.path),
            size: lib.size,
            verificationType: 'sha1',
            hash: lib.sha1
        } as DownloadTask)));

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

    protected generateArgs(placeholders: Record<string, any> = {}): string[] {
        const argumentsObject = this.versionPackage!.arguments
        const args: string[] = []

        const gameArgs = this.getFilteredArgs(argumentsObject.game)
        const jvmArgs = this.getFilteredArgs(argumentsObject.jvm)

        args.push(...this.replaceArgPlaceholders(jvmArgs, placeholders))
        args.push(this.versionPackage!.mainClass)
        args.push(...this.replaceArgPlaceholders(gameArgs, placeholders))

        return args
    }

    private getFilteredArgs(args: any[]): string[] {
        const filteredArgs: string[] = []

        const os = platform();
        const architecture = arch();

        for (const arg of args as any[]) {
            if (typeof arg == 'string') {
                filteredArgs.push(arg)
            } else {
                if (arg.rules) {
                    const allow = checkRules(arg.rules, os, architecture)
                    if (allow) {
                        if (typeof arg.value == 'string') {
                            filteredArgs.push(arg.value)
                        } else if (Array.isArray(arg.value)) {
                            filteredArgs.push(...arg.value)
                        }
                    }
                }
            }
        }

        return filteredArgs
    }

    private replaceArgPlaceholders(args: string[], placeholders: Record<string, any>): string[] {
        return args.map(str =>
            str.replace(/\$\{(\w+)}/g, (match, key) => placeholders[key] ?? match)
        )
    }

    private async getLibraries(rawLibraries: any[]): Promise<MojangLibraryObject[]> {
        const libs: MojangLibraryObject[] = []

        const os = platform();
        const architecture = arch();

        for (const lib of rawLibraries) {
            const rules = lib.rules

            if (!checkRules(rules, os.toLowerCase(), architecture.toLowerCase()))
                continue

            libs.push(lib.downloads.artifact as MojangLibraryObject)
        }

        return libs
    }

    protected async installFiles() {
        this.emit({ stage: "install", message: "Установка Vanilla" })
        // распаковка, json, libraries, assets
    }

    protected override async finalize(): Promise<void> {
        await super.finalize();

        const nativesDir = await path.join(this.minecraftDir!, "natives")
        if (!(await exists(nativesDir))) await mkdir(nativesDir)

        const cp: string[] = []
        for (const library of this.versionPackage.libraries) {
            cp.push(await path.join(this.librariesDir!, library.downloads.artifact.path))
        }
        cp.push(await path.join(this.minecraftDir!, "client.jar"))

        const args = this.generateArgs({
            auth_player_name: "_zaralX_test",
            version: this.versionPackage.id,
            game_directory: this.minecraftDir,
            assets_root: this.assetsDir,
            assets_index_name: this.versionPackage.assets,
            uuid: undefined,
            auth_access_token: "null",
            clientid: undefined,
            auth_xuid: undefined,
            version_type: "Vanilla",
            natives_directory: nativesDir,
            launcher_name: "Cast Launcher",
            launcher_version: "1.0",
            classpath: cp.join(path.delimiter())
        })

        console.log(args)


        await invoke("launch_minecraft", {
            javaPath: "C:/Users/admin/AppData/Roaming/PrismLauncher/java/java-runtime-delta/bin/javaw.exe",
            args: args
        });
    }

    private async resolveVanillaAssets(): Promise<DownloadTask[]> {
        // version_manifest.json → version.json → libraries/assets
        return []
    }
}
