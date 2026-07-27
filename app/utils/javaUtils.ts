import type {JavaRuntime} from "~/types/app";

export interface JavaRequirement {
    major: number
    atLeast?: boolean
}

export function javaRequirementForMinecraft(minecraftVersion: string): JavaRequirement | null {
    const match = /^(\d+)\.(\d+)(?:\.(\d+))?/.exec(minecraftVersion.trim())
    if (!match) return null

    const major = Number(match[1])
    const minor = Number(match[2])
    const patch = Number(match[3] ?? 0)

    if (major >= 20) return {major: 21, atLeast: true}
    if (major !== 1) return null

    if (minor >= 21) return {major: 21}
    if (minor === 20) return {major: patch >= 5 ? 21 : 17}
    if (minor >= 18) return {major: 17}
    if (minor === 17) return {major: 16}

    return {major: 8}
}

export function javaRequirementFromPackage(versionPackage: any, minecraftVersion: string): JavaRequirement | null {
    const major = versionPackage?.javaVersion?.majorVersion
    return typeof major === "number" ? {major} : javaRequirementForMinecraft(minecraftVersion)
}

function maxCompatibleMajor(required: number): number {
    return required <= 8 ? 11 : Number.MAX_SAFE_INTEGER
}

function best(runtimes: JavaRuntime[]): JavaRuntime | null {
    return runtimes.find(runtime => runtime.is_64bit) ?? runtimes[0] ?? null
}

export function pickJavaRuntime(runtimes: JavaRuntime[], requirement: JavaRequirement | null): JavaRuntime | null {
    if (!runtimes.length || !requirement) return null

    if (requirement.atLeast) {
        return best(runtimes.filter(runtime => runtime.major >= requirement.major))
    }

    const exact = best(runtimes.filter(runtime => runtime.major === requirement.major))
    if (exact) return exact

    const higher = runtimes
        .filter(runtime => runtime.major > requirement.major && runtime.major <= maxCompatibleMajor(requirement.major))
        .sort((a, b) => a.major - b.major)

    return higher.length ? best(higher.filter(runtime => runtime.major === higher[0]!.major)) : null
}

export function pickSystemJavaRuntime(runtimes: JavaRuntime[]): JavaRuntime | null {
    const configured = runtimes.find(runtime => runtime.source === "path" || runtime.source === "java_home")
    return configured ?? runtimes[0] ?? null
}
