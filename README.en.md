# OpenCode Config Switch Tool

A Windows desktop app built with Tauri 2 that keeps separate JSON configuration presets for [OpenCode](https://opencode.ai/docs/config/) and [oh-my-openagent](https://github.com/code-yeongyu/oh-my-openagent). Switch the active configuration by writing a preset to the target file, after backing up whatever was there. The interface uses Material Design 3 colors, component states, and Material Symbols icons, with light, dark, and system themes.

**The UI is Chinese only.** There is no English interface and no language setting. This document is the English guide to the app, not a translation of its screens.

[中文说明](README.md)

## Download and run

Grab the portable Windows x64 build for version 1.0.0 from the [releases page](https://github.com/xiushabei/OpenCodeConfigswitchTool/releases/latest).

1. Download the zip for Windows x64.
2. Extract it into a folder you can write to, such as one under your user profile. Avoid read-only or protected locations, since the app stores its own data beside itself.
3. Keep the executable in that folder, next to the included `switcher.settings.json`. The app looks for that settings file in the executable's folder and its parents, then in the working directory and its parents. If you move the executable, move the whole folder with it.

The build is portable: there is no installer and nothing is added to the system PATH.

Click and operation-result sounds can be disabled through the Chinese setting “操作音效”. This preference is saved locally. Typing and startup are silent. Audio assets from [Kenney Interface Sounds](https://kenney.nl/assets/interface-sounds) are bundled for offline playback under CC0; see `src/assets/sounds/ATTRIBUTION.md`.

WebView2 is required. Windows 11 normally has it already; on older systems, install the Evergreen Runtime from Microsoft if the window fails to open.

## Requirements for building

- Windows x64
- Bun or Node.js
- Rust MSVC toolchain
- Microsoft C++ Build Tools with the "Desktop development with C++" workload
- WebView2

Tauri's [prerequisite guide](https://v2.tauri.app/start/prerequisites/) is the authority on Windows build requirements.

## Presets and switching

Presets belong to one target, either OpenCode or oh-my-openagent. Each preset holds a name, an optional description, and the raw JSON text of a configuration.

- **Create** a preset by picking a target, choosing New, and entering a JSON object.
- **Edit** the name and description in place. The content is edited as raw text in a strict JSON editor: the app checks that it parses as a JSON object and never rewrites or guesses OpenCode or oh-my-openagent fields.
- **Duplicate** a preset to branch from an existing one.
- **Import** `.json` or `.jsonc` files, by picker or by dragging them onto the window. JSONC comments are accepted on import. The category is taken from the file name when it identifies OpenCode or OMO, and from the selected category otherwise. Exported preset files carry the category recorded inside them.
- **Export** a preset to `./exports/`, from where it can be imported again.
- **Delete** a preset. This removes only the stored preset, never the target configuration file.

Before writing, open the diff view to compare the preset against the target file's current content, line by line.

Applying a preset validates the JSON again, copies the existing target's original bytes into `./backups/` under a timestamped name, then writes the selected configuration.

### Active markers

A preset is marked active when its JSON/JSONC content is equivalent to the target file. Comments, formatting and object key order are ignored; array order, value types and exact numeric values remain significant. Refresh recalculates markers from disk. The diff uses the same semantics without rewriting editor text; invalid JSON shows an error and falls back to a raw text comparison.

## Startup import, current config import, and restore

Automatic import, manual import, and restore use the following rules.

**Startup import runs once.** On the first launch only, the app reads the OpenCode and OMO files named in the settings and adds them as "Current OpenCode" and "Current OMO" presets. The completed flag is stored with the preset data, so later launches never import automatically again, even if the target files change. Preset data created by older versions counts as already completed. Renamed legacy presets such as "Startup import OpenCode" become "Current OpenCode"; presets you renamed yourself are left alone.

Upgrade exception: for the default OMO path `~/.omo/omo.jsonc`, startup adds the current configuration when no equivalent OMO preset exists. Unchanged content is not duplicated on later launches; genuinely changed content adds one new preset. Custom OMO paths are excluded. The old default path migrates only when the new file exists, after backing up the switcher's settings.

**Import current config always appends.** Each click reads both configured files and adds new presets. Even identical content creates a new entry rather than overwriting an earlier one, and duplicate names gain suffixes like `(2)` and `(3)`.

**Restore to original config restores this launch's baseline.** At every launch, before anything else, the app records the raw contents of both targets. Refresh, preset switches, manual imports, and settings changes do not move that baseline; only quitting and reopening the app does. Restoring returns the targets to the bytes captured at launch, comments and line breaks included. A target that did not exist at launch is removed, because "did not exist" is part of the baseline.

Restore asks first. The dialog requires a second click on Restore; cancelling changes no file and writes no backup. If you have unsaved editor changes, the app asks whether to discard them before proceeding. On confirmation, current targets are backed up, then the launch-time contents are written. The paths are the ones captured at launch, so a settings change cannot make old content land on a new path.

The app never restarts OpenCode. It shows a reminder to restart, because a running session may still be using the configuration it loaded at startup.

## Settings and local data

`switcher.settings.json` sits in the project folder and stores the two target paths plus the theme (`light`, `dark`, or `system`). The shipped defaults:

```json
{
  "targets": {
    "opencode": "~/.config/opencode/opencode.json",
    "omo": "~/.omo/omo.jsonc"
  },
  "theme": "system"
}
```

`~` is expanded at runtime to the current Windows user profile. Relative target paths resolve against the project root. Absolute paths are used as given. Directory paths, empty paths, and Windows device names are rejected.

Everything the app owns stays inside the project folder:

| Path | Contents |
| --- | --- |
| `switcher.settings.json` | Target paths and theme |
| `data/presets.json` | Presets, their content, and the startup-import flag |
| `data/webview/` | WebView profile data |
| `backups/` | Timestamped copies of target files, taken before writes and restores |
| `exports/` | Preset files you exported |

Only applying a preset, or restoring, writes outside the project, and only to the targets you configured.

Backups, exports, and the preset store contain configuration text. If a configuration includes API keys or other secrets, those files include them too. Keep these data directories private.

## Development

```powershell
bun install
Copy-Item switcher.settings.example.json switcher.settings.json
bun run tauri dev
```

Frontend checks and production build:

```powershell
bun run check
bun run build
bun run tauri build
```

Rust tests and lint:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

`bun run dev` alone serves a browser preview of the interface. File operations need the Tauri desktop runtime, so use `bun run tauri dev` when testing anything that reads or writes configuration.

## Checking that a switch took effect

A sane first run: point the targets at test files inside the project, switch once, inspect the result, then point them back at your real configuration.

After applying, check three things: the active marker highlights the preset, the target file contains the expected JSON, and a new timestamped backup exists. Reopen the app and the active state should still match the target's actual content. If something else edits the file, refresh to re-evaluate.

A successful write means the file changed. It does not mean the running OpenCode process picked it up. Quit OpenCode, start it again, and confirm the model or setting you changed. Judging by the current session's behavior will mislead you.

The app validates JSON object syntax only. Field meanings and version compatibility belong to the upstream projects.

## Models and OMO model assignments

The model catalogue runs the installed `opencode models` CLI, using the switcher's root as its working directory and the configured OpenCode target through `OPENCODE_CONFIG`. OpenCode applies its own merged global/project configuration, credentials and plugins. The explicit refresh action runs `opencode models --refresh`.

OMO presets have a model-selection view alongside the raw JSON and diff views. It edits only `[opencode].agents.<key>.model` and `[opencode].categories.<key>.model`; the older flat `agents.<key>.model` / `categories.<key>.model` form remains supported for backward compatibility. Unknown fields, key order and large integers are preserved by the Rust patch command. Existing model IDs absent from the catalogue remain selectable as current values.

## Chat history

Chat history opens an existing `opencode.db` read-only and detects supported v1/v2 schemas. It tries `OPENCODE_DB`, `opencode db path`, and the default user path before a one-time bounded drive scan for the exact filename. You can also enter a path. The database area reports its actual file length and lists WAL length separately when present. Sessions share that SQLite file; each session's content volume counts message/part bytes, supports sorting alongside timestamps, and is not guaranteed recoverable disk space. User and assistant text is shown in full. Tool records start as a short summary and short reasoning parts open directly; every shortened record is a native details/summary toggle that reveals the complete text by pointer or keyboard.

A read returns all messages and parts the backend can decode, and the interface does not clip them again. Full reads use memory proportional to session size and can take longer on very large sessions. The interface marks a part as incomplete only when the backend reports that it could not fully parse it; normal collapsed previews are never labelled as truncated.

Deletion requires OpenCode to be fully stopped, a second confirmation of the selected session and descendants, and verified schema relationships. The confirmation offers a checked-by-default “删除前备份” (back up before deletion) checkbox. When checked, a SQLite-consistent backup must succeed before deletion; unchecking skips backup creation. Unknown schemas fail closed. Deletion/backup tests use synthetic local databases; the live OpenCode database is never used for destructive development QA.

The first deletion attempt shows the Chinese test-phase warning below. Only “我已知晓，继续” records acknowledgement locally and continues to the scope preview and second confirmation. Cancel, close and Escape perform no deletion. Unavailable local storage causes the warning to reappear next time.

> 本功能目前处于测试阶段，常见的问题有：磁盘读写占满，电脑卡顿，整个删除过程大约5-8分钟，使用此功能产生的所有后果由用户承担

The session list mounts only visible rows and nearby rows, preserving dates, parent relationships and keyboard navigation. During resizing, layout updates are combined per animation frame and mounted rows are reused while preserving scroll position. When backup is selected, `VACUUM INTO` creates a complete compact snapshot that omits free database pages; deletion starts only after it succeeds. Large amounts of live data still require corresponding backup IO; this operation does not shrink the source database file. The warning's duration is product copy rather than a measured completion-time guarantee.

Version 1.0.0 passed 117 Rust tests, 56 Bun tests, strict Clippy and the frontend production build, with isolated browser checks for resizing and deletion controls.

## License

Apache License 2.0. See [LICENSE](LICENSE).
