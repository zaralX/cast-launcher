import {defineStore} from 'pinia'
import {invoke} from "@tauri-apps/api/core";
import {appConfigDir, dirname} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import type {
    Account,
    AccountConfig,
    MicrosoftTokens,
    MinecraftAccount,
    MinecraftProfile,
    XboxLiveResponse
} from "~/types/account";
import {open} from "@tauri-apps/plugin-shell";
import {listen} from "@tauri-apps/api/event";
import {$fetch} from "ofetch";
import {
    exchangeMicrosoftCode,
    getMinecraftProfile,
    minecraftXboxLogin,
    xboxLiveAuthenticate,
    xstsAuthorize
} from "~/utils/microsoftUtil";

export const useAccountStore = defineStore('account', {
    state: () => ({
        config: null as null | AccountConfig,
        microsoftClientId: "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb"
    }),
    getters: {
        hasConfig: (state) => !!state.config,
    },
    actions: {
        async getConfigPath() {
            return await path.join(
                await appConfigDir(),
                "accounts.json"
            )
        },

        async loadConfig() {
            const configPath = await this.getConfigPath()
            if (!(await exists(configPath))) {
                await mkdir(await dirname(configPath), {recursive: true})
                this.config = {
                    accounts: []
                }
                await writeTextFile(configPath, JSON.stringify(this.config))
            } else {
                this.config = JSON.parse(await readTextFile(configPath))
            }

            return this.config
        },

        async updateConfig(config: AccountConfig) {
            const configPath = await this.getConfigPath()
            if (!(await exists(configPath))) {
                await mkdir(await dirname(configPath), {recursive: true})
            }
            await writeTextFile(configPath, JSON.stringify(config))
            this.config = config
        },

        async microsoftLogin() {
            const { codeVerifier, codeChallenge } = await PKCE.createPKCEPair();

            const unlisten = await listen<string>('microsoft-oauth-code', async (event) => {
                const code = event.payload
                console.log('OAuth code:', code)
                try {
                    const microsoftTokens: MicrosoftTokens = await exchangeMicrosoftCode(
                        code,
                        codeVerifier,
                        this.microsoftClientId
                    )

                    if (microsoftTokens?.error) {
                        console.error("Failed to fetch microsoft tokens ", microsoftTokens.error)
                        return
                    }

                    const xboxLive: XboxLiveResponse = await xboxLiveAuthenticate(microsoftTokens.access_token)

                    const xstsAuth: XboxLiveResponse = await xstsAuthorize(xboxLive.Token)

                    const minecraftAccount: MinecraftAccount = await minecraftXboxLogin(xboxLive.DisplayClaims.xui[0]!.uhs, xstsAuth.Token)

                    const minecraftProfile: MinecraftProfile = await getMinecraftProfile(minecraftAccount.access_token)

                    const savedAccount: Account = {
                        type: "microsoft",
                        name: minecraftProfile.name,
                        uuid: minecraftProfile.id,
                        accessToken: minecraftAccount.access_token,
                        expiresAt: Math.floor(Date.now() / 1000) + minecraftAccount.expires_in,
                        xblHash: xboxLive.DisplayClaims.xui[0]!.uhs,
                        refreshToken: microsoftTokens.refresh_token,
                        skins: minecraftProfile.skins,
                        capes: minecraftProfile.capes,
                    }

                    console.log("savedAccount", savedAccount)

                    this.config!.accounts.push(savedAccount)
                    await this.updateConfig(this.config!)
                } catch (e) {
                    console.error(e)
                }

                unlisten()
            })

            await invoke('auth_microsoft')

            const url =
                'https://login.live.com/oauth20_authorize.srf' +
                '?client_id=' + this.microsoftClientId +
                '&response_type=code' +
                '&redirect_uri=http://localhost:55325/' +
                '&scope=XboxLive.SignIn%20XboxLive.offline_access' +
                '&code_challenge=' + codeChallenge +
                '&code_challenge_method=S256'

            await open(url)
        },
    }
})
