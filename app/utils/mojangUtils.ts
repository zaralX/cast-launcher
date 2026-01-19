type Rule = {
    action: "allow" | "disallow"
    os?: {
        name?: string
        arch?: string
    }
    arch?: {
        name?: string
    }
    features?: Record<string, boolean>
}

export function checkRules(
    rules: Rule[] | undefined,
    os: string,
    architecture: string,
    features: Record<string, boolean> = {}
): boolean {
    if (!rules || rules.length === 0) {
        return true
    }

    let allowed = false

    for (const rule of rules) {
        let match = true

        // OS
        if (rule.os?.name && rule.os.name !== os) {
            match = false
        }

        // ARCH
        if (rule.arch?.name && rule.arch.name !== architecture) {
            match = false
        }

        // FEATURES
        if (rule.features) {
            for (const [key, value] of Object.entries(rule.features)) {
                // если feature отсутствует — НЕ матч
                if (!(key in features)) {
                    match = false
                    break
                }

                // если значение не совпадает — НЕ матч
                if (features[key] !== value) {
                    match = false
                    break
                }
            }
        }

        // если правило совпало — оно влияет
        if (match) {
            allowed = rule.action === "allow"
        }
    }

    return allowed
}
