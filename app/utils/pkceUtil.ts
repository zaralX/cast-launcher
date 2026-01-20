export class PKCE {
    // Генерация случайного code_verifier (длина 43–128 символов)
    static generateCodeVerifier(length = 128): string {
        const array = new Uint8Array(length);
        crypto.getRandomValues(array);
        // Преобразуем байты в символы URL-safe
        return Array.from(array)
            .map((b) => ('0' + (b % 256).toString(16)).slice(-2))
            .join('')
            .slice(0, length)
            .replace(/\+/g, '-')
            .replace(/\//g, '_')
            .replace(/=/g, '');
    }

    // Генерация code_challenge из code_verifier
    static async generateCodeChallenge(codeVerifier: string): Promise<string> {
        const encoder = new TextEncoder();
        const data = encoder.encode(codeVerifier);
        const hashBuffer = await crypto.subtle.digest('SHA-256', data);
        const hashArray = Array.from(new Uint8Array(hashBuffer));
        const base64 = btoa(String.fromCharCode(...hashArray));
        // Преобразуем в Base64 URL-safe
        return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
    }

    // Удобный метод для получения сразу пары
    static async createPKCEPair(): Promise<{ codeVerifier: string; codeChallenge: string }> {
        const codeVerifier = PKCE.generateCodeVerifier();
        const codeChallenge = await PKCE.generateCodeChallenge(codeVerifier);
        return { codeVerifier, codeChallenge };
    }
}