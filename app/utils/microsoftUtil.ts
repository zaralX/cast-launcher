import type {MicrosoftTokens, MinecraftAccount, MinecraftProfile, XboxLiveResponse} from "~/types/account";
import {invoke} from "@tauri-apps/api/core";
import {$fetch} from "ofetch";

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
    return await $fetch("https://user.auth.xboxlive.com/user/authenticate", {
        method: "POST",
        body: {
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": "d=" + microsoftAccessToken
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }
    })
}

export async function xstsAuthorize(
    xboxLiveToken: string,
): Promise<XboxLiveResponse> {
    return await $fetch("https://xsts.auth.xboxlive.com/xsts/authorize", {
        method: "POST",
        body: {
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [ xboxLiveToken ]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }
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
