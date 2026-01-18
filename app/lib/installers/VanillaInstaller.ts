import type {DownloadTask, MojangLibraryObject, MojangObject} from "~/types/instance"
import { InstallerBase } from "./InstallerBase"
import {$fetch} from "ofetch";
import { path } from "@tauri-apps/api";
import { platform, arch } from "@tauri-apps/plugin-os";

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
        // TODO я сделал получение package дальше писать установку по аналогии со старым
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
    }

    private async getLibraries(rawLibraries: any[]): Promise<MojangLibraryObject[]> {
        const libs: MojangLibraryObject[] = []

        const os = platform();
        const architecture = arch();

        for (const lib of rawLibraries) {
            const rules = lib.rules
            if (rules) {
                let skip = false
                for (const rule of rules) {
                    const verify = (!rule?.os || rule?.os?.name == os) && (!rule?.arch && rule?.arch?.name == architecture)
                    if (rule.action == "allow" && !verify) {
                        skip = true
                    }
                    if (rule.action == "disallow" && verify) {
                        skip = true
                    }
                }
                if (skip) continue
            }
            libs.push(lib.downloads.artifact as MojangLibraryObject)
        }

        return libs
    }

    protected async installFiles() {
        this.emit({ stage: "install", message: "Установка Vanilla" })
        // распаковка, json, libraries, assets
    }

    private async resolveVanillaAssets(): Promise<DownloadTask[]> {
        // version_manifest.json → version.json → libraries/assets
        return []
    }
}
