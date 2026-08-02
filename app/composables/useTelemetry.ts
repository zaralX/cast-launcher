import {invoke} from "@tauri-apps/api/core"

export type TelemetryProps = Record<string, string | number>

const MAX_VALUE_LENGTH = 200

let enabled = false

export function setTelemetryEnabled(value: boolean) {
    enabled = value
}

export function isTelemetryEnabled() {
    return enabled
}

export function trackEvent(name: string, props: TelemetryProps = {}) {
    if (!enabled) return

    invoke("plugin:aptabase|track_event", {name, props: clean(props)})
        .catch(error => console.warn(`[telemetry] ${name}`, error))
}

function clean(props: TelemetryProps): TelemetryProps {
    const cleaned: TelemetryProps = {}

    for (const [key, value] of Object.entries(props)) {
        if (typeof value === "number") {
            if (Number.isFinite(value)) cleaned[key] = value
            continue
        }

        const text = String(value ?? "").trim()
        if (text) cleaned[key] = text.slice(0, MAX_VALUE_LENGTH)
    }

    return cleaned
}
