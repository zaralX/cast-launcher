import { path } from "@tauri-apps/api";
import {exists, mkdir, readTextFile, remove, rename, writeTextFile} from "@tauri-apps/plugin-fs";
import {VanillaInstaller} from "~/lib/installers/VanillaInstaller";
import type {DownloadTask, InstallPhase, MojangLibraryArtifact} from "~/types/instance";
import {getMavenLibraryPath, getMavenUrl} from "~/utils/mavenUtils";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {LauncherError} from "~/types/error";

export class ForgeInstaller extends VanillaInstaller {

    protected override definePhases(): InstallPhase[] {
        return [
            {key: "libraries", label: "Библиотеки", weight: 18},
            {key: "assets", label: "Ресурсы", weight: 45},
            {key: "forge-installer", label: "Установщик Forge", weight: 12},
            {key: "natives", label: "Нативные библиотеки", weight: 5},
            {key: "forge-install", label: "Установка Forge", weight: 20}
        ]
    }

    private async cleanup(targets: string[]) {
        for (const target of targets) {
            try {
                if (await exists(target)) await remove(target, {recursive: true})
            } catch (e) {
                console.warn("Failed to cleanup", target, e)
            }
        }
    }

    protected override async download(): Promise<void> {
        this.emit({ stage: "download", message: "Начало загрузки" })

        await this.downloadLibraries()
        await this.downloadAssets()

        let forgeLoaderVersion = this.instance.loaderVersion

        // Download Forge Installer
        this.beginPhase("forge-installer", "Установщик Forge")

        const forgeInstallerDir = await path.join(this.cacheDir!, "forge", `${forgeLoaderVersion}`)
        try {
            if (!(await exists(forgeInstallerDir))) await mkdir(forgeInstallerDir, { recursive: true })
        } catch (e) {
            throw this.wrap(e, { path: forgeInstallerDir })
        }

        const forgeInstallerFile = await path.join(forgeInstallerDir, "installer.jar")
        if (!(await exists(forgeInstallerFile))) {
            const installerUrl = `https://maven.minecraftforge.net/net/minecraftforge/forge/${forgeLoaderVersion}/forge-${forgeLoaderVersion}-installer.jar`
            await this.downloader.downloadSingle({
                url: installerUrl,
                destination: forgeInstallerFile
            }, (file) => {
                this.emitFile(file, "Установщик Forge")
                this.reportPhase(file.percent)
            })
        }

        this.reportPhase(1)
    }

    protected override async installFiles(): Promise<void> {
        await super.installFiles();
        let forgeLoaderVersion = this.instance.loaderVersion

        const forgeInstallerDir = await path.join(this.cacheDir!, "forge", `${forgeLoaderVersion}`)
        const forgeInstallerFile = await path.join(forgeInstallerDir, "installer.jar")

        const forgeInstalledFile = await path.join(forgeInstallerDir, "client.jar")
        const forgeInstalledJsonFile = await path.join(forgeInstallerDir, "client.json")
        if (!(await exists(forgeInstalledFile) && await exists(forgeInstalledJsonFile))) {
            this.emit({stage: "install", message: "Установка Forge"})
            this.beginPhase("forge-install", "Установщик Forge работает")

            // Forge installer defence fix
            const launcherProfilesFile = await path.join(this.launcherDir, "launcher_profiles.json")
            try {
                await writeTextFile(launcherProfilesFile, JSON.stringify({
                    "profiles": {},
                    "clientToken": "00000000-0000-0000-0000-000000000000",
                    "launcherVersion": {
                        "name": "custom",
                        "format": 21
                    }
                }))
            } catch (e) {
                throw this.wrap(e, { path: launcherProfilesFile })
            }

            const unsubscribeLog = await listen<string>("forgeinstaller-log", (l) => console.log(l.payload))
            const unsubscribeError = await listen<string>("forgeinstaller-error", (e) => console.error(e.payload))

            try {
                await invoke("install_forge", {
                    javaPath: this.javaPath,
                    installerPath: forgeInstallerFile,
                    minecraftDir: this.launcherDir
                })
            } catch (e) {
                throw this.wrap(e, { javaPath: this.javaPath, path: forgeInstallerFile })
            } finally {
                unsubscribeLog()
                unsubscribeError()
            }

            const versionsDir = await path.join(this.launcherDir, "versions")

            // Move installed files
            const _forgeClientJar = await path.join(versionsDir, this.instance.minecraftVersion, `${this.instance.minecraftVersion}.jar`)
            const _forgeClientJson = await path.join(versionsDir, `${this.instance.minecraftVersion}-forge-${forgeLoaderVersion?.split('-')?.[1]}`, `${this.instance.minecraftVersion}-forge-${forgeLoaderVersion?.split('-')?.[1]}.json`)

            for (const [from, to] of [[_forgeClientJar, forgeInstalledFile], [_forgeClientJson, forgeInstalledJsonFile]] as const) {
                if (!(await exists(from))) {
                    throw new LauncherError("FORGE_INSTALL_FAILED", {
                        message: "Установщик Forge не создал ожидаемые файлы",
                        details: `Не найден: ${from}`,
                        context: this.errorContext({ path: from })
                    })
                }
                await rename(from, to).catch(e => {
                    throw this.wrap(e, { path: from })
                })
            }

            // Cleanup
            await this.cleanup([
                versionsDir,
                await path.join(this.launcherDir, "installer.jar.log"),
                await path.join(this.launcherDir, "launcher_profiles.json")
            ])
        }
    }
}
