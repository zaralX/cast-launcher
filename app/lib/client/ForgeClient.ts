import {ClientBase} from "~/lib/client/ClientBase";
import {path} from "@tauri-apps/api";
import {readTextFile} from "@tauri-apps/plugin-fs";
import type { Account } from "~/types/account";
import {v4} from "uuid";
import {VanillaClient} from "~/lib/client/VanillaClient";
import {getMavenLibraryPath} from "~/utils/mavenUtils";
import type {LivingInstance, MojangLibraryArtifact} from "~/types/instance";

export class ForgeClient extends VanillaClient {
    private loaderPackage: any

    constructor(launcherDir: string, instance: LivingInstance) {
        super(launcherDir, instance);
        this.versionType = "Forge"
    }

    override async prepare(): Promise<void> {
        await super.prepare()
        const loaderVersionPackageFile = await path.join(this.launcherDir, "cache", "forge", `${this.instance.loaderVersion}`, 'client.json')
        this.loaderPackage = JSON.parse(await readTextFile(loaderVersionPackageFile));
    }

    protected override async generateArgs(placeholders: Record<string, any> = {}): Promise<string[]> {
        const argumentsObject = this.versionPackage.arguments
        const forgeArgumentsObject = this.loaderPackage.arguments
        const args: string[] = []

        // Legacy versions doesnt have arguments object
        if (argumentsObject) {
            const gameArgs = [...ClientBase.getMojangRuleFilteredArgs(argumentsObject.game), ...(forgeArgumentsObject?.game ?? [])]
            const jvmArgs = [...ClientBase.getMojangRuleFilteredArgs(argumentsObject.jvm), ...(forgeArgumentsObject?.jvm ?? [])]

            args.push(...ClientBase.replaceArgPlaceholders(jvmArgs, placeholders))
            args.push(this.loaderPackage.mainClass)
            args.push(...ClientBase.replaceArgPlaceholders(gameArgs, placeholders))
        } else if (this.loaderPackage.minecraftArguments) {
            // Forge legacy versions have readyToGo args
            const gameArgs = this.loaderPackage.minecraftArguments.split(" ") as string[]
            const jvmArgs = ["-Djava.library.path=${natives_directory}", "-cp", "${classpath}"]

            args.push(...ClientBase.replaceArgPlaceholders(jvmArgs, placeholders))
            args.push(this.loaderPackage.mainClass)
            args.push(...ClientBase.replaceArgPlaceholders(gameArgs, placeholders))
        }

        return args
    }

    protected override async generateCP(libraries: any[]): Promise<string[]> {
        const forgeCp: string[] = []

        const forgeLibs: MojangLibraryArtifact[] = this.loaderPackage.libraries.map((lib: any) => lib.downloads.artifact)

        for (const library of forgeLibs) {
            forgeCp.push(await path.join(this.librariesDir!, library.path))
        }

        const vanillaCp = await super.generateCP(libraries)
        vanillaCp.pop() // remove vanilla client.jar

        const patchedForgeJar = await path.join(this.launcherDir, "cache", "forge", `${this.instance.loaderVersion}`, 'client.jar')

        return [...forgeCp, ...vanillaCp, patchedForgeJar];
    }
}
