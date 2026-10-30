# Contributing to Activity Manager

Thanks for taking the time to help. Bug reports, ideas, translations and pull requests are all welcome.

## Reporting a bug

Use the **Bug report** form in [Issues](https://github.com/giannoccarol/Activity-manager-linux/issues/new/choose) and include:

- the app version (**Settings → Updates**, or `gestione-attivita --version`), your distribution, the Plasma version and whether you are on Wayland or X11;
- what you did, what you expected and what happened instead;
- for crashes and errors, what the app prints in a terminal. Quit it from the system tray first, otherwise the new command only hands over to the running instance, then start `gestione-attivita` from a terminal.

For wrong or missing data, `gestione-attivita --dump` (processes and resources) and `gestione-attivita --sensors` (hardware sensors) print what the app reads, as JSON. That output includes your process list and command lines: remove anything private before posting it.

## Building

You need [Rust](https://rustup.rs) 1.80 or newer, Node.js 20 or newer and the WebKitGTK 4.1 development libraries (see the [Tauri prerequisites](https://tauri.app/start/prerequisites/)).

```bash
npm install
npm run dev          # interface in the browser with demo data (http://localhost:1420)
npx tauri dev        # full app with real data
npx tauri build      # release binary in src-tauri/target/release
```

Always build release binaries with `npx tauri build`: a plain `cargo build --release` produces a binary that looks for the development server instead of the embedded interface.

The browser preview accepts a few parameters, handy for screenshots and visual checks: `?page=sensors` (any page), `?theme=glass`, `paper` or `graphite`, `?lang=en` or `it`, and `?view=secure` for the Ctrl+Alt+Del screen.

## Guidelines

- **Performance first.** The app runs all day, often in the background. Measure changes on the installed app (`gestione-attivita --bench` writes its results to `~/.cache/gestione-attivita/bench.txt`), keep animations short and on `transform` and `opacity`, and avoid infinite animations: with the Glass theme KWin recomputes the blur on every frame.
- **Read the kernel directly.** Prefer `/proc` and `/sys` over external tools, never add a privileged daemon, and never wake a suspended GPU just to read it.
- **Every visible string is translated.** Add it to both `src/locales/en.json` and `src/locales/it.json` with the same key; strings used by the Rust backend go in the `core` section.
- **English** for code, comments, commit messages and pull requests. Commit subjects are short and in the imperative mood ("Add …", "Fix …").
- **Demo data in screenshots.** Use `npm run dev` or `npx tauri dev` with demo data, so no personal processes, networks or names end up in the repository.

## Adding a language

1. Copy `src/locales/en.json` to `src/locales/<code>.json` and translate the values. Keep the keys and the `{placeholders}` as they are; values with `one` and `other` are plural forms.
2. Register the language in `src/lib/i18n.svelte.ts` (the `Lang` type, the dictionaries and the locale used for numbers and dates) and in `src-tauri/src/i18n.rs` (the backend dictionaries and the detection from the system locale).
3. Add it to the language choice in `src/pages/Settings.svelte`, with a `settings.language.<code>` label in every dictionary.
4. Check each page with `npm run dev`.

## Pull requests

- Keep each pull request to one topic, and describe what changes for the user and how you tested it. Add screenshots for visual changes.
- CI builds the interface, the backend and the AUR package on every pull request.
- By contributing you agree that your work is released under the [GNU General Public License v3.0 or later](LICENSE), like the rest of the project.
