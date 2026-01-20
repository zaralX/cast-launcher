import { path } from "@tauri-apps/api";
import {exists, mkdir, readTextFile, remove, rename, writeTextFile} from "@tauri-apps/plugin-fs";
import {VanillaInstaller} from "~/lib/installers/VanillaInstaller";
import type {DownloadTask, MojangLibraryArtifact} from "~/types/instance";
import {getMavenLibraryPath, getMavenUrl} from "~/utils/mavenUtils";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";

export class ForgeInstaller extends VanillaInstaller {

    protected override async download(): Promise<void> {
        this.emit({ stage: "download", message: "Начало загрузки" })

        await this.downloadLibraries()
        await this.downloadAssets()

        let forgeLoaderVersion = this.instance.loaderVersion

        // Download Forge Installer
        this.emit({
            stage: "download",
            message: "Загрузка Forge Installer",
            type: "global"
        })

        const forgeInstallerDir = await path.join(this.cacheDir!, "forge", `${forgeLoaderVersion}`)
        if (!(await exists(forgeInstallerDir))) await mkdir(forgeInstallerDir, { recursive: true })

        const forgeInstallerFile = await path.join(forgeInstallerDir, "installer.jar")
        if (!(await exists(forgeInstallerFile))) {
            const installerUrl = `https://maven.minecraftforge.net/net/minecraftforge/forge/${forgeLoaderVersion}/forge-${forgeLoaderVersion}-installer.jar`
            await this.downloader.downloadSingle({
                url: installerUrl,
                destination: forgeInstallerFile
            })
        }
    }

    protected override async installFiles(): Promise<void> {
        await super.installFiles();
        let forgeLoaderVersion = this.instance.loaderVersion

        const forgeInstallerDir = await path.join(this.cacheDir!, "forge", `${forgeLoaderVersion}`)
        const forgeInstallerFile = await path.join(forgeInstallerDir, "installer.jar")

        const forgeInstalledFile = await path.join(forgeInstallerDir, "client.jar")
        const forgeInstalledJsonFile = await path.join(forgeInstallerDir, "client.json")
        if (!(await exists(forgeInstalledFile) && await exists(forgeInstalledJsonFile))) {
            this.emit({
                stage: "install",
                message: "Установка Forge",
                type: "global"
            })

            await writeTextFile(await path.join(this.launcherDir, "launcher_profiles.json"), JSON.stringify({
                    "profiles": {},
                    "clientToken": "00000000-0000-0000-0000-000000000000",
                    "launcherVersion": {
                        "name": "custom",
                        "format": 21
                    }
                }
            )) // Forge installer defence fix

            const unsubscribeLog = await listen<string>("forgeinstaller-log", (l) => console.log(l.payload))
            const unsubscribeError = await listen<string>("forgeinstaller-error", (e) => console.error(e.payload))

            await invoke("install_forge", {
                javaPath: "C:/Users/admin/AppData/Roaming/PrismLauncher/java/java-runtime-delta/bin/javaw.exe",
                installerPath: forgeInstallerFile,
                minecraftDir: this.launcherDir
            }).catch(() => {
                unsubscribeLog()
                unsubscribeError()
            })

            unsubscribeLog()
            unsubscribeError()

            const versionsDir = await path.join(this.launcherDir, "versions")

            // Move installed files
            const _forgeClientJar = await path.join(versionsDir, this.instance.minecraftVersion, `${this.instance.minecraftVersion}.jar`)
            const _forgeClientJson = await path.join(versionsDir, `${this.instance.minecraftVersion}-forge-${forgeLoaderVersion?.split('-')?.[1]}`, `${this.instance.minecraftVersion}-forge-${forgeLoaderVersion?.split('-')?.[1]}.json`)

            await rename(_forgeClientJar, forgeInstalledFile);
            await rename(_forgeClientJson, forgeInstalledJsonFile);

            // Cleanup
            await remove(versionsDir, {recursive: true})
            await remove(await path.join(this.launcherDir, "installer.jar.log"))
            await remove(await path.join(this.launcherDir, "launcher_profiles.json"))
        }
    }
}
