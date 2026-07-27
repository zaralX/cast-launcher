import { path } from "@tauri-apps/api";
import { exists, mkdir, writeTextFile } from "@tauri-apps/plugin-fs";
import {VanillaInstaller} from "~/lib/installers/VanillaInstaller";
import type {DownloadTask} from "~/types/instance";
import {getMavenLibraryPath, getMavenUrl} from "~/utils/mavenUtils";
import {LauncherError} from "~/types/error";

const FABRIC_MAVEN = "https://maven.fabricmc.net/"

export class FabricInstaller extends VanillaInstaller {

    protected override async download(): Promise<void> {
        await super.download(); // Vanilla client.jar, libs and assets

        let loaderData
        let fabricLoaderVersion = this.instance.loaderVersion
        if (fabricLoaderVersion == 'latest') {
            const fabricLoaders = await this.fetchJson<any[]>("https://meta.fabricmc.net/v2/versions/loader/" + this.instance.minecraftVersion)
            loaderData = fabricLoaders?.[0]
        } else {
            // Cached loader version
            const versionPackageDir = await path.join(this.cacheDir!, "fabric_loaders", `${fabricLoaderVersion}`)
            const versionPackageFile = await path.join(versionPackageDir, "package.json")

            loaderData = await this.readCachedJson(versionPackageFile)

            if (!loaderData) {
                loaderData = await this.fetchJson(`https://meta.fabricmc.net/v2/versions/loader/${this.instance.minecraftVersion}/${fabricLoaderVersion}`)

                try {
                    if (!(await exists(versionPackageDir))) await mkdir(versionPackageDir, { recursive: true })
                    await writeTextFile(versionPackageFile, JSON.stringify(loaderData))
                } catch (e) {
                    throw this.wrap(e, { path: versionPackageFile })
                }
            }
        }

        if (!loaderData?.loader?.maven || !loaderData?.intermediary?.maven || !loaderData?.launcherMeta?.libraries) {
            throw new LauncherError("VERSION_NOT_FOUND", {
                message: `Fabric ${fabricLoaderVersion} недоступен для Minecraft ${this.instance.minecraftVersion}`,
                context: this.errorContext()
            })
        }

        // Download Fabric libs
        this.emit({
            stage: "download",
            message: "Загрузка Fabric Libraries",
            type: "global"
        })

        const librariesTasks: DownloadTask[] = []

        // Loader
        librariesTasks.push({
            url: getMavenUrl(loaderData.loader.maven, FABRIC_MAVEN),
            destination: await path.join(this.librariesDir!, getMavenLibraryPath(loaderData.loader.maven)),
        })

        // Intermediary
        librariesTasks.push({
            url: getMavenUrl(loaderData.intermediary.maven, FABRIC_MAVEN),
            destination: await path.join(this.librariesDir!, getMavenLibraryPath(loaderData.intermediary.maven)),
        })

        // Other libs
        for (const lib of [
            ...(loaderData.launcherMeta.libraries.common ?? []),
            ...(loaderData.launcherMeta.libraries.client ?? [])
        ]) {
            librariesTasks.push({
                url: getMavenUrl(lib.name, lib.url ?? FABRIC_MAVEN),
                destination: await path.join(this.librariesDir!, getMavenLibraryPath(lib.name)),
                size: lib.size,
                verificationType: 'sha1',
                hash: lib.sha1,
            })
        }

        await this.downloader.download(librariesTasks, (progress) => {
            this.emit({
                stage: "download",
                type: 'single',
                message: "Загрузка Fabric библиотеки " + progress.name,
                progress: progress.percent,
            })
        }, (progress) => {
            this.emit({
                stage: "download",
                type: 'global',
                message: "Загрузка Fabric библиотек",
                progress: progress,
            })
        })
    }
}
