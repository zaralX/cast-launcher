import type {LauncherError} from "~/types/error";
import {type ReportOptions, useErrorStore} from "~/stores/error";

export type Attempt<T> =
    | { ok: true, value: T }
    | { ok: false, error: LauncherError }

export const useErrorCenterOpen = () => useState<boolean>("error-center-open", () => false)

export function captureError(raw: unknown, options: ReportOptions = {}): LauncherError {
    return useErrorStore().report(raw, options)
}

export async function safeRun<T>(
    operation: () => Promise<T>,
    options: ReportOptions = {}
): Promise<T | undefined> {
    try {
        return await operation()
    } catch (e) {
        captureError(e, options)
        return undefined
    }
}

export async function attempt<T>(
    operation: () => Promise<T>,
    options: ReportOptions = {}
): Promise<Attempt<T>> {
    try {
        return {ok: true, value: await operation()}
    } catch (e) {
        return {ok: false, error: captureError(e, options)}
    }
}
