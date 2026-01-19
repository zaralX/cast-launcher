import {ClientBase} from "~/lib/client/ClientBase";
import {path} from "@tauri-apps/api";
import {readTextFile} from "@tauri-apps/plugin-fs";
import type { Account } from "~/types/account";

export class VanillaClient extends ClientBase {
    private versionPackage: any

    public override async prepare(): Promise<void> {
        await super.prepare();
        const versionPackageFile = await path.join(this.launcherDir, "cache", "versions", `${this.instance.minecraftVersion}-vanilla`, 'package.json')
        this.versionPackage = JSON.parse(await readTextFile(versionPackageFile))
    }

    protected override async generateArgs(placeholders: Record<string, any> = {}): Promise<string[]> {
        const argumentsObject = this.versionPackage.arguments
        const args: string[] = []

        const gameArgs = ClientBase.getMojangRuleFilteredArgs(argumentsObject.game)
        const jvmArgs = ClientBase.getMojangRuleFilteredArgs(argumentsObject.jvm)

        args.push(...ClientBase.replaceArgPlaceholders(jvmArgs, placeholders))
        args.push(this.versionPackage.mainClass)
        args.push(...ClientBase.replaceArgPlaceholders(gameArgs, placeholders))

        return args
    }


    protected override async getFullArgs(account: Account): Promise<string[]> {
        const cp = await this.generateCP(this.versionPackage.libraries)
        return this.generateArgs({
            auth_player_name: account.name,
            version_name: this.versionPackage.id,
            game_directory: this.minecraftDir,
            assets_root: this.assetsDir!,
            assets_index_name: this.versionPackage.assets,
            uuid: undefined,
            auth_access_token: "null",
            clientid: undefined,
            auth_xuid: undefined,
            version_type: "Vanilla",
            natives_directory: this.nativesDir!,
            launcher_name: "Cast Launcher",
            launcher_version: "1.0",
            classpath: cp.join(path.delimiter())
        });
    }
}
