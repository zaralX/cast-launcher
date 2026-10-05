# CLAUDE.md

Guidance for Claude Code when working in this repository.

## What this is

Cast Launcher — a Minecraft launcher, a Tauri 2 desktop app. Rust does all the work
(version metadata, downloads, installs, Java, accounts, launching); the Nuxt 4 frontend
in `app/` is a client-only SPA (`ssr: false`) that renders Rust state and calls Rust
commands. The webview never touches the filesystem, the network or processes: the
capabilities in [src-tauri/capabilities/](src-tauri/capabilities/) grant window operations,
telemetry and the self-updater, nothing else.

Frontend style follows `vilbux-panel` (a Nuxt site), adapted for the desktop: there is no
SSR and no HTTP API — the backend is the Rust process behind IPC.

## Commands

Package manager is **npm** (`package-lock.json`).

| Command                     | Result                                                       |
|-----------------------------|--------------------------------------------------------------|
| `npm run tauri:dev`         | the app with hot reload (`nuxt dev` + the Rust side)         |
| `npm run generate`          | static frontend into `.output/public`, linked as `dist/`     |
| `npm run build`             | `nuxt build` — quickest check that every SFC compiles        |
| `npm run lint` / `lint:fix` | ESLint (`@nuxt/eslint`, stylistic)                           |
| `npm run typecheck`         | `nuxt typecheck` (vue-tsc)                                   |
| `cargo test -p cast-core`   | Rust tests, run from `src-tauri/`                            |

`nuxt dev` alone in a browser hangs on start: `bootstrap` has no Rust to answer it. The
frontend has no tests — `lint` + `typecheck` is its gate, and CI runs both (`frontend` job).

New logic goes to `cast-core` together with its tests: on windows-gnu a test binary that
links Tauri doesn't start (no Common-Controls v6 manifest), so the app crate has
`test = false` and stays a thin layer.

On a `*-windows-gnu` toolchain cargo needs a working MinGW gcc for bundled SQLite; under
Git Bash it fails, so run cargo from PowerShell.

## Layout

```
src-tauri/
├─ core/            # cast-core: all launcher logic + unit tests, no Tauri dependency
├─ src/             # cast-launcher: thin #[tauri::command]s, events, AppState, orchestration
└─ capabilities/    # what the webview may do
app/
├─ app.vue          # UApp, appearance + language, error toasts
├─ app.config.ts    # Nuxt UI theme overrides (square corners, mono labels)
├─ assets/css/      # tailwind.css: @theme tokens · main.css: --cast-* palette, :root dark / html.light
├─ components/
│  ├─ app/          # shell and shared pieces: Button, ErrorCenter, LoadingScreen, UnsavedChangesModal
│  ├─ instance/     # instance page tabs, cards, icon, create modal, blocked files
│  ├─ import/       # launcher import wizard, modpack file import
│  └─ …             # castpack/, onboarding/, search/, settings/, skin/
├─ composables/     # use* only: useLauncherEvents, useInstanceActions, useFileDrop, …
├─ utils/           # functions + their private constants (auto-imported): backend, error, telemetry, …
├─ types/           # one file per entity: types mirroring Rust structs + constants
├─ stores/          # Pinia option stores mirroring Rust state
└─ layouts/ · pages/ · middleware/ · plugins/
i18n/locales/       # ru.json, en.json
```

Components are path-prefixed: `components/instance/Card.vue` is `<InstanceCard>`.
`checkUnknownComponents` is on, so `typecheck` catches a stale tag after a move.
`utils/`, `composables/` and `stores/` are auto-imported — don't import them by hand.
`types/` is not scanned: import its types and constants explicitly.

## The one rule that matters: Rust is the backend

**Commands.** Every Rust command has a twin in `Commands`
([app/types/backend.ts](app/types/backend.ts)): `name: Command<Args, Result>`, `void` for
none. Call it as `call('name', args)` ([app/utils/backend.ts](app/utils/backend.ts)) — typed
by that map, rejects with `LauncherError`. A new command is a `#[tauri::command]` in
`src-tauri/src`, an entry in `generate_handler!` in `src-tauri/src/lib.rs` and a line in
`Commands`. Argument keys are camelCase on the JS side (`instanceId`), as Tauri expects.

**Events.** Rust emits exactly one event, `launcher://event`, with a `LauncherEvent` tagged
by `type` ([src-tauri/core/src/events.rs](src-tauri/core/src/events.rs)). The frontend
listens once, in `useLauncherEvents()`, which also runs `bootstrap` and feeds the stores.
A new event kind needs the Rust variant, the TS union member and a case in that switch.

ESLint rejects raw `invoke`/`listen` everywhere except `utils/backend.ts` and
`utils/telemetry.ts`. Telemetry calls the aptabase plugin command directly because the
`@aptabase/tauri` npm package is built for Tauri v1 — don't install it.

**Errors.** A failed command arrives as `{ code, message, details }`. `code` is one of the
strings from `src-tauri/core/src/error.rs`, and `ErrorCode`
([app/types/error.ts](app/types/error.ts)) mirrors them by hand — add new codes on both
sides. `ERROR_CATALOG` maps a code to severity, icon and i18n key; the code itself is never
translated, it goes to telemetry and reports as-is. In UI code:

```ts
await safeRun(() => call('open_url', { url }), { context: { action: t('…') } }) // reports, returns undefined
const result = await attempt(() => store.deleteInstance(id))                     // { ok, value } | { ok, error }
```

Anything uncaught (Vue errors, unhandled rejections) is reported by
`plugins/errors.client.ts`. Reports land in the error center and become toasts through
`registerErrorSink` in `app.vue`.

**Platform APIs.** Window and app APIs (`getCurrentWindow`, `getVersion`) are fine in
components. File drag-and-drop goes through `useFileDrop(onDrop)`, which returns the hover
ref. External links and folders open via `call('open_url')` / `call('open_path')`.

## Conventions

**State.** Stores are Pinia option stores mirroring Rust: filled by `bootstrap` and events,
changed by actions that call commands. Unlike vilbux there is no SSR or page data loading,
so setup stores would buy nothing — keep the option style.

**No SSR.** No `import.meta.client` guards, no `useAsyncData`/`callOnce`: load in
`onMounted` or a store action.

**i18n.** `@nuxtjs/i18n`, `no_prefix`, default `ru`, fallback `en`. Flat dotted keys in
`i18n/locales/ru.json` and `en.json` — same keys in the same order; `"_comment…"` keys are
section markers. `$t` in templates, `const { t } = useI18n()` in scripts,
`useNuxtApp().$i18n` in utils and stores; markup inside a message goes through `<i18n-t>`.
Constants hold a `labelKey`, never text. The language lives in `config.launcher.language`
(Rust reads it too) and is applied by `useLanguage()`; the locale list is only in
`nuxt.config.ts`. Dates format with the current locale (`toLocaleString(locale.value)`),
never a hardcoded one.

**UI.** Nuxt UI 4 themed in `app.config.ts`; `<AppButton>` is the launcher's own button.
Colors come only from tokens: `bg-ink-900…600`, `text-fg` / `fg-muted` / `fg-faint`,
`border-line`, `acid` (the accent the user picks in settings, via `ui.colors.primary`).
No hex in components. Fonts: Golos Text (body), `font-unbounded` (headings), `font-mono`
(labels).

**Icons.** `i-lucide-*`, plus `simple-icons:*` and `flag:*`. They are bundled offline
(`icon.provider: 'none'`, client bundle scanned from `app/**`), so an icon name must appear
as a literal string in the source — a name built at runtime is not bundled and renders
empty.

**Auto-import trap.** mlly's export scanner drops the export that follows a one-line object
literal with commas (`export const A = { w: 1, h: 2 }`). Write such objects multi-line in
auto-imported dirs; `typecheck` reports the miss as `Cannot find name`.

**Comments.** Default to none. When the code would surprise a reader (a workaround, a Rust
quirk, an order that matters), one short sentence — in Russian, like the rest of the code.
No JSDoc on internal functions, no section banners, no commented-out code.

**Style** is enforced by ESLint stylistic: run `npm run lint:fix`, don't format by hand.
The bulk-format commit is in `.git-blame-ignore-revs`
(`git config blame.ignoreRevsFile .git-blame-ignore-revs` to use it locally).

## Release

- Version: `node scripts/set-version.mjs X.Y.Z` — strictly X.Y.Z, the NSIS installer and
  the updater's version comparison break on suffixes.
- `bundle.targets` in `src-tauri/tauri.conf.json` stays an explicit list without `msi`:
  with `"all"` the updater's `windows-x86_64` entry points at the MSI and NSIS installs
  get a second, separate install.

## Related repos

[castpacks-manager](../../WebstormProjects/castpacks-manager) — the admin panel and API for
CastPacks. Its `shared/castpack.ts` mirrors `src-tauri/core/src/castpack/*.rs` by hand:
change both together, or the launcher silently drops catalog entries.
