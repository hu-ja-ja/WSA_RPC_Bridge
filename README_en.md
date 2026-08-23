# WSA RPC Bridge

[English](README_en.md) | [日本語](README.md) | [Documentation](https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/)

An app (Windows desktop / Android) that retrieves media playback information playing on WSA (Windows Subsystem for Android) or Android devices and displays it on Discord Rich Presence. For user-facing setup, features, and legal information, see the [documentation site](https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/).

## Screenshots

![GUI](img/GUI.png)

![RPC](img/RPC.png)

## Tech Stack

| Layer          | Tech                                      |
|----------------|-------------------------------------------|
| Frontend       | SolidJS + [Kobalte](https://kobalte.dev/) + Vite |
| Backend        | Rust / Tauri v2                           |
| Android Native | Kotlin (Notification Access / JNI Bridge) |
| Documentation  | Astro Starlight (site/)                   |

## Repository Layout

```
src/                  SolidJS frontend (Vite SPA)
src-tauri/            Rust / Tauri app + Android (gen/android)
docs/                 Legacy Markdown docs. Migrated to site/
site/                 Documentation site (Astro Starlight). Deployed to /docs/ on GitHub Pages
scripts/              Build helper scripts (license generation, etc.)
.github/workflows/    CI / release / documentation automation
```

## Development

Tools are managed with [mise](https://mise.jdx.dev/). Commands are defined as mise tasks; pass arguments with `mise run <task> -- <args>`.

```pwsh
mise trust        # First time only
mise install      # Installs Node / pnpm / Rust / Perl / Java
mise run deps     # Installs JS dependencies
mise run dev      # Tauri + Vite dev
mise run build    # Release build
mise run lint     # oxlint
mise run test     # Rust unit tests
mise run android-test  # Android unit tests (Robolectric)
mise run generate-licenses  # Regenerate third-party licenses
```

The `dev` / `build` / `tauri` tasks go through [Infisical](https://infisical.com/) `infisical run` (manages release signing secrets). If you are not logged in, run `pnpm tauri ...` directly. See the [command reference](https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/dev/commands) for the full list.

## CI

- **ci.yml** — lint / build / test / android-test on push to main and PRs
- **release.yml** — manual releases via `workflow_dispatch`. Builds APK (aarch64) + MSI and deploys `update.json` to the Pages root. See the [release policy](https://hu-ja-ja.github.io/WSA_RPC_Bridge/docs/dev/release)
- **docs.yml** — deploys the documentation site to `/docs/` on Pages when site/ changes

## Documentation Site (site/)

Documentation is maintained in the [Astro Starlight](https://starlight.astro.build/) project under `site/`. Available in Japanese and English (English lives under `/en/`).

```pwsh
mise run docs-dev      # Local dev server (http://localhost:4321)
mise run docs-build    # Build for Pages deployment (BASE_PATH=/WSA_RPC_Bridge/docs/)
```

Build with `BASE_PATH='/WSA_RPC_Bridge/docs/' pnpm --dir site build`, output goes to `site/dist/`. Subpath deployment on GitHub Pages uses Astro's native `base` option.

## License

Copyright (C) 2026 hu-ja-ja

[MPL-2.0](LICENSE)

Third-party licenses can be found under the licenses tab in the sidebar of the app.
