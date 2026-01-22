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
    minecraftXboxLogin, refreshMicrosoftToken,
    xboxLiveAuthenticate,
    xstsAuthorize
} from "~/utils/microsoftUtil";

export const useAccountStore = defineStore('account', {
    state: () => ({
        accountConfig: null as null | AccountConfig,
        microsoftClientId: "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb"
    }),
    getters: {
        hasConfig: (state) => !!state.accountConfig,
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
                this.accountConfig = {
                    accounts: []
                }
                await writeTextFile(configPath, JSON.stringify(this.accountConfig))
            } else {
                this.accountConfig = JSON.parse(await readTextFile(configPath))
            }

            console.log("Loaded account config ", this.accountConfig)

            return this.accountConfig
        },

        async updateConfig(config: AccountConfig) {
            const configPath = await this.getConfigPath()
            if (!(await exists(configPath))) {
                await mkdir(await dirname(configPath), {recursive: true})
            }
            await writeTextFile(configPath, JSON.stringify(config))
            this.accountConfig = config
        },

        async selectAccount(id: number) {
            this.accountConfig!.selected = id < this.accountConfig!.accounts.length ? id : 0;
            await this.updateConfig(this.accountConfig!)
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

                    this.accountConfig!.accounts.push(savedAccount)
                    await this.updateConfig(this.accountConfig!)
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

        async refreshMicrosoftAccount(uuid: string) {
            const account = this.accountConfig!.accounts.find(a => a.uuid == uuid)

            if (!account) {
                throw new Error("Account not found")
            }

            const microsoftTokens: MicrosoftTokens = await refreshMicrosoftToken(account.refreshToken!, this.microsoftClientId)

            if (microsoftTokens?.error) {
                console.error("Failed to fetch microsoft tokens ", microsoftTokens.error)
                return
            }

            const xboxLive: XboxLiveResponse = await xboxLiveAuthenticate(microsoftTokens.access_token)

            const xstsAuth: XboxLiveResponse = await xstsAuthorize(xboxLive.Token)

            const minecraftAccount: MinecraftAccount = await minecraftXboxLogin(xboxLive.DisplayClaims.xui[0]!.uhs, xstsAuth.Token)

            const minecraftProfile: MinecraftProfile = await getMinecraftProfile(minecraftAccount.access_token)

            account.skins = minecraftProfile.skins
            account.capes = minecraftProfile.capes
            account.accessToken = minecraftAccount.access_token
            account.xblHash = xboxLive.DisplayClaims.xui[0]!.uhs
            account.expiresAt = Math.floor(Date.now() / 1000) + minecraftAccount.expires_in

            await this.updateConfig(this.accountConfig!)
        }
    }
})
