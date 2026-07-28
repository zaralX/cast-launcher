import {arch, platform, version} from "@tauri-apps/plugin-os"
import type {MojangLibraryArtifact} from "~/types/instance"

export type MojangOs = "windows" | "osx" | "linux" | "unknown"

export interface RuntimeContext {
    os: MojangOs
    arch: string
    bits: "32" | "64"
    version: string
}

export type RuleAction = "allow" | "disallow"

export interface Rule {
    action: RuleAction
    os?: {
        name?: string
        arch?: string
        version?: string
    }
    features?: Record<string, boolean>
}

export interface ResolvedLibrary {
    name?: string
    artifact?: MojangLibraryArtifact
    native?: MojangLibraryArtifact
}

const MOJANG_OS: Record<string, MojangOs> = {
    windows: "windows",
    macos: "osx",
    ios: "osx",
    linux: "linux",
    android: "linux"
}

const MOJANG_ARCH: Record<string, string> = {
    x86: "x86",
    x86_64: "x86_64",
    arm: "arm32",
    aarch64: "arm64"
}

let runtimeContext: RuntimeContext | null = null

export function getRuntimeContext(): RuntimeContext {
    if (runtimeContext) return runtimeContext

    const mappedArch = MOJANG_ARCH[arch()] ?? arch()

    runtimeContext = {
        os: MOJANG_OS[platform()] ?? "unknown",
        arch: mappedArch,
        bits: mappedArch === "x86" || mappedArch === "arm32" ? "32" : "64",
        version: version()
    }

    return runtimeContext
}

export function checkRules(
    rules: Rule[] | undefined,
    ctx: RuntimeContext = getRuntimeContext(),
    features: Record<string, boolean> = {}
): boolean {
    if (!rules || rules.length === 0) {
        return true
    }

    let allowed = false

    for (const rule of rules) {
        if (!ruleMatches(rule, ctx, features)) continue
        allowed = rule.action === "allow"
    }

    return allowed
}

function ruleMatches(rule: Rule, ctx: RuntimeContext, features: Record<string, boolean>): boolean {
    if (rule.os) {
        if (rule.os.name && !osNameMatches(rule.os.name, ctx)) return false
        if (rule.os.arch && rule.os.arch !== ctx.arch) return false
        if (rule.os.version && !osVersionMatches(rule.os.version, ctx.version)) return false
    }

    if (rule.features) {
        for (const [key, value] of Object.entries(rule.features)) {
            if ((features[key] ?? false) !== value) return false
        }
    }

    return true
}

function osNameMatches(name: string, ctx: RuntimeContext): boolean {
    return name === ctx.os || name === `${ctx.os}-${ctx.arch}`
}

function osVersionMatches(pattern: string, current: string): boolean {
    try {
        return new RegExp(pattern).test(current)
    } catch {
        return false
    }
}

export function resolveLibraries(
    rawLibraries: any[] | undefined,
    ctx: RuntimeContext = getRuntimeContext()
): ResolvedLibrary[] {
    if (!Array.isArray(rawLibraries)) return []

    const libs: ResolvedLibrary[] = []

    for (const lib of rawLibraries) {
        if (!checkRules(lib?.rules, ctx)) continue

        const artifact: MojangLibraryArtifact | undefined = lib?.downloads?.artifact
        const native = resolveNativeArtifact(lib, ctx)

        if (!artifact?.path && !native) continue

        libs.push({
            name: lib?.name,
            artifact: artifact?.path ? artifact : undefined,
            native
        })
    }

    return libs
}

function resolveNativeArtifact(lib: any, ctx: RuntimeContext): MojangLibraryArtifact | undefined {
    const classifier = lib?.natives?.[ctx.os]
    if (typeof classifier !== "string") return undefined

    const resolved = classifier.replace(/\$\{arch}/g, ctx.bits)
    const native: MojangLibraryArtifact | undefined = lib?.downloads?.classifiers?.[resolved]

    return native?.path ? native : undefined
}
