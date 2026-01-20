import { path } from "@tauri-apps/api";
import { exists, mkdir, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import {VanillaInstaller} from "~/lib/installers/VanillaInstaller";
import type {DownloadTask} from "~/types/instance";
import {getMavenLibraryPath, getMavenUrl} from "~/utils/mavenUtils";

export class FabricInstaller extends VanillaInstaller {

    protected override async download(): Promise<void> {
        await super.download(); // Vanilla client.jar, libs and assets

        let loaderData;
        let fabricLoaderVersion = this.instance.loaderVersion
        if (fabricLoaderVersion == 'latest') {
            const fabricLoaders: any[] = await $fetch("https://meta.fabricmc.net/v2/versions/loader/" + this.instance.minecraftVersion)
            loaderData = fabricLoaders[0]
        } else {
            // Cached loader version
            const versionPackageDir = await path.join(this.cacheDir!, "fabric_loaders", `${fabricLoaderVersion}`)
            if (!(await exists(versionPackageDir))) await mkdir(versionPackageDir, { recursive: true })
            const versionPackageFile = await path.join(versionPackageDir, "package.json")
            if (await exists(versionPackageFile)) {
                loaderData = JSON.parse(await readTextFile(versionPackageFile))
            }

            if (!loaderData) {
                loaderData = await $fetch(`https://meta.fabricmc.net/v2/versions/loader/${this.instance.minecraftVersion}/${fabricLoaderVersion}`)
                await writeTextFile(versionPackageFile, JSON.stringify(loaderData))
            }
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
            url: getMavenUrl(loaderData.loader.maven, "https://maven.fabricmc.net/"),
            destination: await path.join(this.librariesDir!, getMavenLibraryPath(loaderData.loader.maven)),
        })

        // Intermediary
        librariesTasks.push({
            url: getMavenUrl(loaderData.intermediary.maven, "https://maven.fabricmc.net/"),
            destination: await path.join(this.librariesDir!, getMavenLibraryPath(loaderData.intermediary.maven)),
        })

        // Other libs
        for (const lib of [...loaderData.launcherMeta.libraries.common, ...loaderData.launcherMeta.libraries.client]) {
            librariesTasks.push({
                url: getMavenUrl(lib.name, lib.url),
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
