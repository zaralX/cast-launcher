import type {MicrosoftTokens, MinecraftAccount, MinecraftProfile, XboxLiveResponse} from "~/types/account";
import {invoke} from "@tauri-apps/api/core";
import {$fetch} from "ofetch";
import {LauncherError} from "~/types/error";

const XERR_HINTS: Record<string, string> = {
    "2148916233": "К аккаунту Microsoft не привязан профиль Xbox. Создайте его на xbox.com и повторите вход.",
    "2148916235": "Xbox Live недоступен в стране этого аккаунта.",
    "2148916236": "Аккаунт требует подтверждения возраста.",
    "2148916237": "Аккаунт требует подтверждения возраста.",
    "2148916238": "Детский аккаунт должен быть добавлен в семейную группу Microsoft."
}

async function xboxRequest<T>(url: string, body: Record<string, any>): Promise<T> {
    try {
        return await $fetch<T>(url, {method: "POST", body})
    } catch (e) {
        const data = (e as { data?: { XErr?: number | string } })?.data
        const xErr = data?.XErr !== undefined ? String(data.XErr) : undefined

        throw new LauncherError("AUTH_FAILED", {
            message: (xErr && XERR_HINTS[xErr])
                ?? (xErr ? `Xbox Live отклонил вход (XErr ${xErr})` : "Xbox Live отклонил вход"),
            details: data ? JSON.stringify(data, null, 2) : (e instanceof Error ? e.message : String(e)),
            context: {url},
            cause: e
        })
    }
}

export async function exchangeMicrosoftCode(
    code: string,
    verifier: string,
    microsoftClientId: string
): Promise<MicrosoftTokens> {
    return await invoke("exchange_microsoft_code", {
        code,
        codeVerifier: verifier,
        clientId: microsoftClientId
    }) as MicrosoftTokens
}

export async function xboxLiveAuthenticate(
    microsoftAccessToken: string,
): Promise<XboxLiveResponse> {
    return await xboxRequest("https://user.auth.xboxlive.com/user/authenticate", {
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": "d=" + microsoftAccessToken
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    })
}

export async function xstsAuthorize(
    xboxLiveToken: string,
): Promise<XboxLiveResponse> {
    return await xboxRequest("https://xsts.auth.xboxlive.com/xsts/authorize", {
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [ xboxLiveToken ]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    })
}

export async function minecraftXboxLogin(
    xboxLiveUHSToken: string,
    xstsAuthToken: string,
): Promise<MinecraftAccount> {
    return await invoke("minecraft_services_request", {
        url: "https://api.minecraftservices.com/authentication/login_with_xbox",
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: {
            identityToken: `XBL3.0 x=${xboxLiveUHSToken};${xstsAuthToken}`
        }
    })
}

export async function createMinecraftProfile(
    minecraftAccessToken: string,
    profileName: string,
): Promise<any> {
    return await invoke("minecraft_services_request", {
        url: "https://api.minecraftservices.com/minecraft/profile",
        method: "POST",
        headers: {
            Authorization: `Bearer ${minecraftAccessToken}`,
            "Content-Type": "application/json"
        },
        body: {
            profileName: profileName
        }
    })
}

export async function getMinecraftProfile(
    minecraftAccessToken: string,
): Promise<MinecraftProfile> {
    return await invoke("minecraft_services_request", {
        url: 'https://api.minecraftservices.com/minecraft/profile',
        headers: {
            Authorization: `Bearer ${minecraftAccessToken}`
        },
    })
}

export async function refreshMicrosoftToken(
    refreshToken: string,
    clientId: string
): Promise<MicrosoftTokens> {
    return await invoke("refresh_microsoft", {
        refreshToken,
        clientId: clientId
    }) as MicrosoftTokens
}
