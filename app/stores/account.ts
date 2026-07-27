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
import {LauncherError, toLauncherError} from "~/types/error";
import {captureError} from "~/composables/useErrorHandler";

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
                this.accountConfig = { accounts: [] }
                await this.updateConfig(this.accountConfig)
                return this.accountConfig
            }

            try {
                this.accountConfig = JSON.parse(await readTextFile(configPath))
            } catch (e) {
                captureError(e, {code: "CONFIG_ERROR", context: {path: configPath}})
                this.accountConfig = { accounts: [] }
            }

            console.log("Loaded account config ", this.accountConfig)

            return this.accountConfig
        },

        async updateConfig(config: AccountConfig) {
            const configPath = await this.getConfigPath()

            try {
                if (!(await exists(configPath))) {
                    await mkdir(await dirname(configPath), {recursive: true})
                }
                await writeTextFile(configPath, JSON.stringify(config))
            } catch (e) {
                throw toLauncherError(e, "FS_ERROR", {path: configPath})
            }

            this.accountConfig = config
        },

        async selectAccount(id: number) {
            this.accountConfig!.selected = id < this.accountConfig!.accounts.length ? id : 0;
            await this.updateConfig(this.accountConfig!)
        },

        async completeMicrosoftLogin(microsoftTokens: MicrosoftTokens) {
            const xboxLive: XboxLiveResponse = await xboxLiveAuthenticate(microsoftTokens.access_token)

            const uhs = xboxLive?.DisplayClaims?.xui?.[0]?.uhs
            if (!uhs || !xboxLive?.Token) {
                throw new LauncherError("AUTH_FAILED", {
                    message: "Xbox Live вернул ответ без токена",
                    details: JSON.stringify(xboxLive, null, 2)
                })
            }

            const xstsAuth: XboxLiveResponse = await xstsAuthorize(xboxLive.Token)
            const minecraftAccount: MinecraftAccount = await minecraftXboxLogin(uhs, xstsAuth.Token)
            const minecraftProfile: MinecraftProfile = await getMinecraftProfile(minecraftAccount.access_token)

            if (!minecraftProfile?.id) {
                throw new LauncherError("AUTH_FAILED", {
                    message: "Minecraft не вернул профиль для этого аккаунта",
                    details: JSON.stringify(minecraftProfile, null, 2)
                })
            }

            return {uhs, minecraftAccount, minecraftProfile}
        },

        async microsoftLogin() {
            const { codeVerifier, codeChallenge } = await PKCE.createPKCEPair();

            let unlistenCode: (() => void) | undefined
            let unlistenError: (() => void) | undefined
            let timeout: ReturnType<typeof setTimeout> | undefined

            const stop = () => {
                unlistenCode?.()
                unlistenError?.()
                if (timeout) clearTimeout(timeout)
                unlistenCode = unlistenError = undefined
                timeout = undefined
            }

            unlistenCode = await listen<string>('microsoft-oauth-code', async (event) => {
                try {
                    const microsoftTokens: MicrosoftTokens = await exchangeMicrosoftCode(
                        event.payload,
                        codeVerifier,
                        this.microsoftClientId
                    )

                    const {uhs, minecraftAccount, minecraftProfile} = await this.completeMicrosoftLogin(microsoftTokens)

                    const savedAccount: Account = {
                        type: "microsoft",
                        name: minecraftProfile.name,
                        uuid: minecraftProfile.id,
                        accessToken: minecraftAccount.access_token,
                        expiresAt: Math.floor(Date.now() / 1000) + minecraftAccount.expires_in,
                        xblHash: uhs,
                        refreshToken: microsoftTokens.refresh_token,
                        skins: minecraftProfile.skins,
                        capes: minecraftProfile.capes,
                    }

                    this.accountConfig!.accounts.push(savedAccount)
                    await this.updateConfig(this.accountConfig!)
                } catch (e) {
                    captureError(e, {code: "AUTH_FAILED"})
                } finally {
                    stop()
                }
            })

            unlistenError = await listen<string>('microsoft-oauth-error', (event) => {
                captureError(new LauncherError("AUTH_FAILED", {message: event.payload}))
                stop()
            })

            timeout = setTimeout(stop, 5 * 60 * 1000)

            try {
                await invoke('auth_microsoft')
            } catch (e) {
                stop()
                throw toLauncherError(e, "AUTH_FAILED", {})
            }

            const url =
                'https://login.live.com/oauth20_authorize.srf' +
                '?client_id=' + this.microsoftClientId +
                '&response_type=code' +
                '&redirect_uri=http://localhost:55325/' +
                '&scope=XboxLive.SignIn%20XboxLive.offline_access' +
                '&code_challenge=' + codeChallenge +
                '&code_challenge_method=S256'

            try {
                await open(url)
            } catch (e) {
                stop()
                throw new LauncherError("AUTH_FAILED", {
                    message: "Не удалось открыть браузер для входа",
                    cause: e
                })
            }
        },

        async refreshMicrosoftAccount(uuid: string) {
            const account = this.accountConfig!.accounts.find(a => a.uuid == uuid)

            if (!account) {
                throw new LauncherError("NO_ACCOUNT", {
                    message: "Аккаунт не найден в конфигурации",
                    context: {uuid}
                })
            }

            if (!account.refreshToken) {
                throw new LauncherError("AUTH_EXPIRED", {
                    message: `Для аккаунта ${account.name} нет refresh-токена. Войдите заново.`,
                    context: {uuid}
                })
            }

            const microsoftTokens: MicrosoftTokens = await refreshMicrosoftToken(account.refreshToken, this.microsoftClientId)

            const {uhs, minecraftAccount, minecraftProfile} = await this.completeMicrosoftLogin(microsoftTokens)

            account.skins = minecraftProfile.skins
            account.capes = minecraftProfile.capes
            account.accessToken = minecraftAccount.access_token
            account.xblHash = uhs
            account.expiresAt = Math.floor(Date.now() / 1000) + minecraftAccount.expires_in
            if (microsoftTokens.refresh_token) account.refreshToken = microsoftTokens.refresh_token

            await this.updateConfig(this.accountConfig!)
        }
    }
})
