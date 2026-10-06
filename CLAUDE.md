# CLAUDE.md

Guidance for Claude Code when working in this repository.

## What this is

Cast Launcher is a Minecraft launcher, a Tauri 2 desktop app. Rust does all the work
(version metadata, downloads, installs, Java, accounts, launching); the Nuxt 4 frontend
in `app/` is a client-only SPA (`ssr: false`) that renders Rust state and calls Rust
commands. The webview never touches the filesystem, the network or processes: the
capabilities in [src-tauri/capabilities/](src-tauri/capabilities/) grant window operations,
logging, telemetry and the self-updater, nothing else.

Frontend style follows `vilbux-panel` (a Nuxt site), adapted for the desktop: there is no
SSR and no HTTP API, the backend is the Rust process behind IPC.

## Commands

Package manager is **npm** (`package-lock.json`). Cargo commands run from `src-tauri/`.

| Command                                                | Result                                                 |
|--------------------------------------------------------|--------------------------------------------------------|
| `npm run tauri:dev`                                    | the app with hot reload (`nuxt dev` + the Rust side)   |
| `npm run generate`                                     | static frontend into `.output/public`, linked as `dist/` |
| `npm run build`                                        | `nuxt build`, the quickest check that every SFC compiles |
| `npm run lint` / `lint:fix`                            | ESLint (`@nuxt/eslint`, stylistic)                     |
| `npm run typecheck`                                    | `nuxt typecheck` (vue-tsc)                             |
| `cargo test -p cast-core`                              | Rust tests                                             |
| `cargo clippy --workspace --all-targets -- -D warnings` | lints, including the workspace lints from `Cargo.toml` |
| `cargo fmt --all`                                      | rustfmt                                                |

`nuxt dev` alone in a browser hangs on start: `bootstrap` has no Rust to answer it. The
frontend has no tests, so `lint` + `typecheck` is its gate. CI runs all of the above:
rustfmt and clippy block merges, clippy runs on all three platforms.

On a `*-windows-gnu` toolchain cargo needs a working MinGW gcc for bundled SQLite; under
Git Bash it fails, so run cargo from PowerShell.

## Layout

```
src-tauri/
├─ core/            # cast-core: all launcher logic + unit tests, no Tauri dependency
├─ src/             # cast-launcher: thin #[tauri::command]s, events, AppState, orchestration
└─ capabilities/    # what the webview may do
app/
├─ app.vue          # tooltip provider, toaster, appearance + language, error toasts
├─ assets/css/      # tailwind.css: @theme tokens · main.css: --cast-* palette, accents, :root dark / html.light
├─ components/
│  ├─ ui/           # shadcn-vue primitives, rewritten for the launcher: <Button>, <Dialog>, <Select>, …
│  ├─ kit/          # launcher blocks built from ui/: <KitPanel>, <KitField>, <KitConfirmDialog>, …
│  ├─ app/          # window shell: TitleBar, NavRail, WindowControls, ErrorCenter, ActiveDownloads, …
│  ├─ instance/     # instance page tabs, cards, icon, create modal, blocked files
│  ├─ import/       # launcher import wizard, modpack file import
│  └─ …             # cast/, castpack/, onboarding/, search/, settings/, skin/
├─ composables/     # use* only: useLauncherEvents, useInstanceActions, useFileDrop, useAppToast, …
├─ utils/           # functions + their private constants (auto-imported): backend, error, log, …
├─ types/           # one file per entity: types mirroring Rust structs + constants; ui.ts for kit props
├─ lib/             # cn() and the shared looks of ui/ (lib/styles.ts), imported by hand
├─ stores/          # Pinia option stores mirroring Rust state
└─ layouts/ · pages/ · middleware/ · plugins/
i18n/locales/       # ru.json, en.json
```

Components are path-prefixed: `components/instance/Card.vue` is `<InstanceCard>`. The
exception is `components/ui/`, registered without a prefix (`<Button>`, `<DialogContent>`).
`checkUnknownComponents` is on, so `typecheck` catches a stale tag after a move.
`utils/`, `composables/` and `stores/` are auto-imported, don't import them by hand.
`types/` is not scanned: import its types and constants explicitly.

## The one rule that matters: Rust is the backend

**Commands.** Every Rust command has a twin in `Commands`
([app/types/backend.ts](app/types/backend.ts)): `name: Command<Args, Result>`, `void` for
none. Call it as `call('name', args)` ([app/utils/backend.ts](app/utils/backend.ts)): typed
by that map, rejects with `LauncherError`. A new command is a `#[tauri::command]` in
`src-tauri/src`, an entry in `generate_handler!` in `src-tauri/src/lib.rs` and a line in
`Commands`. Argument keys are camelCase on the JS side (`instanceId`), as Tauri expects.

**Events.** Rust emits exactly one event, `launcher://event`, with a `LauncherEvent` tagged
by `type` ([src-tauri/core/src/events.rs](src-tauri/core/src/events.rs)). The frontend
listens once, in `useLauncherEvents()`, which also runs `bootstrap` and feeds the stores.
A new event kind needs the Rust variant, the TS union member and a case in that switch.

ESLint rejects raw `invoke`/`listen` everywhere except `utils/backend.ts` and
`utils/telemetry.ts`. Telemetry calls the aptabase plugin command directly because the
`@aptabase/tauri` npm package is built for Tauri v1, don't install it.

**Text for the user lives in the locales, never in Rust.** Rust sends a `UiText`
(`{ key, params }`, [src-tauri/core/src/text.rs](src-tauri/core/src/text.rs)) and the
frontend renders it with `uiText()`. A param can itself be a `UiText`, for a reason inside
a sentence. Keys Rust sends: `error.reason.*`, `install.phase.*` / `install.message.*`,
`settings.import.step.*`, `import.blocked.*`, `catalog.unsupported.*`. Add the key to both
locales together with the Rust code: `cargo test` fails on a key that is missing in
`ru.json` or `en.json`. Wording Rust needs before the frontend sees anything (native dialog
titles, the page the browser shows after sign-in, the name of a copied skin) is passed in
as a command argument.

**Errors.** A failed command arrives as `{ code, text, details? }`
([src-tauri/core/src/error.rs](src-tauri/core/src/error.rs)). There is no prose in it:

- `code` is an `ErrorCode` and names the *cause*: the frontend picks the title, icon and
  generic hint by it (`ERROR_CATALOG` in [app/types/error.ts](app/types/error.ts), mirrored
  by hand). Bad user input is `INVALID_INPUT`, "not right now" (a running game, an install
  in progress) is `CONFLICT`, a missing entity is `NOT_FOUND`, a feature we lack is
  `UNSUPPORTED`. Never reach for `FS_ERROR` or `MANIFEST_INVALID` just because the error
  happened near a file.
- `text` is what happened, as an `error.reason.*` key with params. Every constructor takes
  that key: `CommandError::network("error.reason.network.connect").param("url", url)`.
  `CommandError::io(key, path, e)` adds the path as `{path}` by itself.
- `details` is technical data only: an OS or library error, a path, an HTTP body, hashes.
  Anything the user should read goes into `text` as a param.

The toast shows the title and the text, the error center adds the code's hint and the
details. Rust logs an error as `[CODE] key {params}`; `writeErrorLog()` on the frontend
renders the text from `en.json` for the log file.

In UI code:

```ts
await safeRun(() => call('open_url', { url }), { context: { action: t('…') } }) // reports, returns undefined
const result = await attempt(() => store.deleteInstance(id))                     // { ok, value } | { ok, error }
```

Anything uncaught (Vue errors, unhandled rejections) is reported by
`plugins/errors.client.ts`. Reports land in the error center, become toasts through
`registerErrorSink` in `app.vue`, and are written to the log file by `writeErrorLog()`.

**Platform APIs.** Window and app APIs (`getCurrentWindow`, `getVersion`) are fine in
components. File drag-and-drop goes through `useFileDrop(onDrop)`, which returns the hover
ref. External links and folders open via `call('open_url')` / `call('open_path')`.

## Rust conventions

**Thin app crate.** A command takes state, calls core, emits events and telemetry, returns.
Any domain decision (validation, picking a version, building a `PackSource`) lives in
`cast-core` with a test; `InstanceUpdate::normalized` and `packs::switch` are the pattern.
`cast-launcher` has `test = false` (on windows-gnu a test binary that links Tauri does not
start, there is no Common-Controls v6 manifest), so a test there silently never runs: don't
write one, move the code to core instead.

**Logging.** Only the `log` macros, in English; clippy rejects `println!`/`eprintln!`
(stderr goes nowhere in a Windows release build, the `log` file is what users send us).
`warn!` for a failure we recover from, `error!` for one that breaks a flow, `info!` for
facts worth having in a bug report. Log an error where it is swallowed; errors that reach
the frontend are logged there.

**Lints.** `[workspace.lints]` in `src-tauri/Cargo.toml` flag printing, `dbg!`, `todo!`,
`unimplemented!` and `unwrap()` (`clippy.toml` allows unwrap and printing in tests), and CI
turns every clippy warning into an error.
`expect()` is for invariants only, with a message that states the invariant.

## Frontend conventions

**State.** Stores are Pinia option stores mirroring Rust: filled by `bootstrap` and events,
changed by actions that call commands. Unlike vilbux there is no SSR or page data loading,
so setup stores would buy nothing: keep the option style.

**No SSR.** No `import.meta.client` guards, no `useAsyncData`/`callOnce`: load in
`onMounted` or a store action.

**i18n.** `@nuxtjs/i18n`, `no_prefix`, default `ru`, fallback `en`. Flat dotted keys in
`i18n/locales/ru.json` and `en.json`, same keys in the same order; `"_comment…"` keys are
section markers. `$t` in templates, `const { t } = useI18n()` in scripts,
`useNuxtApp().$i18n` in utils and stores; markup inside a message goes through `<i18n-t>`.
Constants hold a `labelKey`, never text. The language lives in `config.launcher.language`
(Rust reads it too) and is applied by `useLanguage()`; the locale list is only in
`nuxt.config.ts`. Dates format with the current locale (`toLocaleString(locale.value)`),
never a hardcoded one.

**UI.** Three layers, each built only from the one below:

- `components/ui/` holds shadcn-vue primitives on Reka UI. They come from the CLI and are
  then rewritten for the launcher, as in vilbux-panel: our tokens instead of shadcn's, icons
  through `<Icon>`, `cva` variants for every look. Add one with
  `npx shadcn-vue@latest add <name>` and `npm run lint:fix`, then restyle it (under a proxy
  the CLI needs `HTTPS_PROXY` unset). Looks shared by several primitives (a field, a floating
  panel, a menu row) live in `lib/styles.ts`.
- `components/kit/` holds launcher blocks: panel, field, row, status line, alert-like note,
  confirm dialog, selects, search and number inputs, stat grid, list item, tile, page header.
- Screens use only these two. A screen with its own button, field, label, badge or status
  color is a missing variant or a missing kit block: add it there instead.

`<Button>` variants: `fill` (default, the accent sweep), `danger`, `outline`, `dashed`,
`ghost`, `toolbar` (active state through `aria-pressed`), `quiet`, `quiet-danger`, `link`;
sizes `xs…xl` and `icon-sm` / `icon` / `icon-lg`. It takes `icon`, `loading` and `to`.
Toasts go through `useAppToast().add({ title, description, icon, color, actions })`, which
renders `ui/sonner/Toast.vue` in vue-sonner.

Colors come only from tokens: `bg-ink-900…600`, `text-fg` / `fg-muted` / `fg-faint`,
`border-line`, `acid` (the accent the user picks, set as `data-accent` on `<html>`),
`success`, `warning`, `danger` / `danger-strong`. No hex and no Tailwind palette colors in
components. Type sizes are tokens too: `text-micro` 9px, `text-label` 10px, `text-caption`
11px, `text-body` 12px, `text-title` 13px, `text-lead` 14px, `text-heading` 15px; uppercase
mono labels use `tracking-caps` (`tracking-caps-wide` for eyebrows), Unbounded headings
`tracking-heading` / `tracking-display`. A one-off `text-[Npx]` is for display sizes only.
A new size or tracking token also goes into the `extendTailwindMerge` list in
`lib/utils.ts`, otherwise `cn()` drops it next to a color class. Fonts: Golos Text (body),
`font-unbounded` (headings), `font-mono` (labels).

The window is at least 1080px wide, so `sm:`, `md:` and `lg:` always apply; only `xl:` and
`2xl:` are real breakpoints. Scrollbars are native and styled in `main.css`; there is no
`ScrollArea`.

**Icons.** `<Icon name="i-lucide-…">`, plus `simple-icons:*` and `flag:*` (flags need
`mode="svg"`, they are multicolored). They are bundled offline
(`icon.provider: 'none'`, client bundle scanned from `app/**`), so an icon name must appear
as a literal string in the source: a name built at runtime is not bundled and renders
empty.

**Auto-import trap.** mlly's export scanner drops the export that follows a one-line object
literal with commas (`export const A = { w: 1, h: 2 }`). Write such objects multi-line in
auto-imported dirs; `typecheck` reports the miss as `Cannot find name`. Auto-import also
ignores a name used only inside `${…}` of a template string; `typecheck` does not see that one
and the page throws `is not defined` at runtime, so import such a name explicitly.

## Everywhere

**Comments.** Default to none. When the code would surprise a reader (a workaround, a Rust
quirk, an order that matters), one short sentence. Comments, log messages and test
assertion messages are in English and use no em dashes. No JSDoc on internal functions,
no section banners, no commented-out code.

**Style** is enforced by tools: `npm run lint:fix` and `cargo fmt --all`, don't format by
hand. The bulk-format commits are in `.git-blame-ignore-revs`
(`git config blame.ignoreRevsFile .git-blame-ignore-revs` to use it locally).

## Release

- Version: `node scripts/set-version.mjs X.Y.Z`. Strictly X.Y.Z: the NSIS installer and
  the updater's version comparison break on suffixes.
- `bundle.targets` in `src-tauri/tauri.conf.json` stays an explicit list without `msi`:
  with `"all"` the updater's `windows-x86_64` entry points at the MSI and NSIS installs
  get a second, separate install.

## Related repos

[castpacks-manager](../../WebstormProjects/castpacks-manager) is the admin panel and API
for CastPacks. Its `shared/castpack.ts` mirrors `src-tauri/core/src/castpack/*.rs` by hand:
change both together, or the launcher silently drops catalog entries.

A `.cast` file (`castpack/file.rs`) is a zip with the same manifest inside `cast.json`, plus
files under `files/` that the manifest lists as `embedded`. The catalog refuses embedded
files, so the panel never needs them. An instance from a file is a CastPack with
`origin: file`: its pack archive is the `.cast` itself, and a newer file updates it.
