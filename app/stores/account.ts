import {defineStore} from 'pinia'
import {invoke} from "@tauri-apps/api/core";
import {appConfigDir, dirname} from "@tauri-apps/api/path";
import {path} from "@tauri-apps/api";
import {exists, mkdir, readTextFile, writeTextFile} from "@tauri-apps/plugin-fs";
import type {
    AccountConfig,
    MicrosoftTokens,
    MinecraftAccount,
    MinecraftProfile,
    XboxLiveResponse
} from "~/types/account";
import {open} from "@tauri-apps/plugin-shell";
import {listen} from "@tauri-apps/api/event";
import {$fetch} from "ofetch";

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

        async getMicrosoftLogin() {
            function base64Url(buffer: ArrayBuffer) {
                return btoa(String.fromCharCode(...new Uint8Array(buffer)))
                    .replace(/\+/g, '-')
                    .replace(/\//g, '_')
                    .replace(/=+$/, '')
            }

            async function generatePKCE() {
                const verifier = crypto.randomUUID().replace(/-/g, '')
                const hash = await crypto.subtle.digest(
                    'SHA-256',
                    new TextEncoder().encode(verifier)
                )

                return {
                    verifier,
                    challenge: base64Url(hash)
                }
            }

            const { verifier, challenge } = await generatePKCE()

            const url =
                'https://login.live.com/oauth20_authorize.srf' +
                '?client_id=' + this.microsoftClientId +
                '&response_type=code' +
                '&redirect_uri=http://localhost:55325/' +
                '&scope=XboxLive.SignIn%20XboxLive.offline_access' +
                '&code_challenge=' + challenge +
                '&code_challenge_method=S256'

            return { url, verifier, challenge }
        },

        async microsoftLogin() {
            const { codeVerifier, codeChallenge } = await PKCE.createPKCEPair();

            const unlisten = await listen<string>('microsoft-oauth-code', async (event) => {
                const code = event.payload
                console.log('OAuth code:', code)

                const microsoftTokens: MicrosoftTokens = await this.exchangeMicrosoftCode(
                    code,
                    codeVerifier
                )

                if (microsoftTokens?.error) {
                    throw new Error("Failed to fetch microsoft tokens " + microsoftTokens.error)
                }

                console.log('Microsoft tokens:', microsoftTokens)

                const xboxLive: XboxLiveResponse = await $fetch("https://user.auth.xboxlive.com/user/authenticate", {
                    method: "POST",
                    body: {
                        "Properties": {
                            "AuthMethod": "RPS",
                            "SiteName": "user.auth.xboxlive.com",
                            "RpsTicket": "d=" + microsoftTokens.access_token
                        },
                        "RelyingParty": "http://auth.xboxlive.com",
                        "TokenType": "JWT"
                    }
                })

                console.log("xboxLive", xboxLive)

                const xstsAuth: XboxLiveResponse = await $fetch("https://xsts.auth.xboxlive.com/xsts/authorize", {
                    method: "POST",
                    body: {
                        "Properties": {
                            "SandboxId": "RETAIL",
                            "UserTokens": [ xboxLive.Token ]
                        },
                        "RelyingParty": "rp://api.minecraftservices.com/",
                        "TokenType": "JWT"
                    }
                })

                console.log("xstsAuth", xstsAuth)

                const minecraftAccount: MinecraftAccount = await invoke("minecraft_services_request", {
                    url: "https://api.minecraftservices.com/authentication/login_with_xbox",
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json"
                    },
                    body: {
                        identityToken: `XBL3.0 x=${xboxLive.DisplayClaims.xui[0]!.uhs};${xstsAuth.Token}`
                    }
                })

                console.log("minecraftAccount", minecraftAccount)

                // const createdMinecraftProfile = await invoke("minecraft_services_request", {
                //     url: "https://api.minecraftservices.com/minecraft/profile",
                //     method: "POST",
                //     headers: {
                //         Authorization: `Bearer ${minecraftAccount.access_token}`,
                //         "Content-Type": "application/json"
                //     },
                //     body: {
                //         profileName: "_zaralX_"
                //     }
                // })

                // console.log("createdMinecraftProfile", createdMinecraftProfile)

                const minecraftProfile: MinecraftProfile = await invoke("minecraft_services_request", {
                    url: 'https://api.minecraftservices.com/minecraft/profile',
                    headers: {
                        Authorization: `Bearer ${minecraftAccount.access_token}`
                    },
                })

                console.log("minecraftProfile", minecraftProfile)

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

        async exchangeMicrosoftCode(
            code: string,
            verifier: string
        ): Promise<MicrosoftTokens> {
            return await invoke("exchange_microsoft_code", {
                code,
                codeVerifier: verifier,
                clientId: this.microsoftClientId
            }) as MicrosoftTokens
        }

    }
})
