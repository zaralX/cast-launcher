import {ClientBase} from "~/lib/client/ClientBase";
import {path} from "@tauri-apps/api";
import {readTextFile} from "@tauri-apps/plugin-fs";
import type { Account } from "~/types/account";
import {v4} from "uuid";
import {VanillaClient} from "~/lib/client/VanillaClient";
import {getMavenLibraryPath} from "~/utils/mavenUtils";
import type {LivingInstance} from "~/types/instance";

export class FabricClient extends VanillaClient {
    private loaderPackage: any


    constructor(launcherDir: string, instance: LivingInstance) {
        super(launcherDir, instance);
        this.versionType = "Fabric"
    }

    override async prepare(): Promise<void> {
        await super.prepare()
        const loaderVersionPackageFile = await path.join(this.launcherDir, "cache", "fabric_loaders", `${this.instance.loaderVersion}`, 'package.json')
        this.loaderPackage = JSON.parse(await readTextFile(loaderVersionPackageFile));
    }

    protected override async generateArgs(placeholders: Record<string, any> = {}): Promise<string[]> {
        const argumentsObject = this.versionPackage.arguments
        const args: string[] = []

        const gameArgs = ClientBase.getMojangRuleFilteredArgs(argumentsObject.game)
        const jvmArgs = ClientBase.getMojangRuleFilteredArgs(argumentsObject.jvm)

        args.push(...ClientBase.replaceArgPlaceholders(jvmArgs, placeholders))
        args.push(this.loaderPackage.launcherMeta.mainClass.client)
        args.push(...ClientBase.replaceArgPlaceholders(gameArgs, placeholders))

        return args
    }

    protected override async generateCP(libraries: any[]): Promise<string[]> {
        const fabricCp: string[] = []
        fabricCp.push(await path.join(this.librariesDir!, getMavenLibraryPath(this.loaderPackage.loader.maven))) // Loader
        fabricCp.push(await path.join(this.librariesDir!, getMavenLibraryPath(this.loaderPackage.intermediary.maven))) // Intermediary
        for (const library of [...this.loaderPackage.launcherMeta.libraries.common, ...this.loaderPackage.launcherMeta.libraries.client]) {
            fabricCp.push(await path.join(this.librariesDir!, getMavenLibraryPath(library.name)))
        }

        return [...fabricCp, ...await super.generateCP(libraries)];
    }
}
