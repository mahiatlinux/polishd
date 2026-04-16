# polishd

Desktop app that **polishes selected text** with AI through [OpenRouter](https://openrouter.ai/). Select text in any app, hit the shortcut, and the polished text is pasted back. Use **Ctrl+Shift+D** (Windows/Linux) or **Cmd+Shift+D** (macOS) to open a **custom instruction** modal and run your own prompt on the selection.

Built with **Tauri 2**, **React**, **TypeScript**, and **Vite**.

## What it does

- Runs in the **system tray** (Settings, Quit)
- **Global hotkey** for polish (default **Ctrl+Shift+E** / **Cmd+Shift+E** on macOS)
- Stores your **OpenRouter API key** locally in `polishd.json` under the app data directory
- **Light / dark** theme in the settings window

## Local development

**Needs:** [Node.js](https://nodejs.org/) (LTS), [Rust](https://www.rust-lang.org/tools/install), and [Tauri’s system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/mahiatlinux/polishd.git
cd polishd
npm install
npm run tauri dev
```

## Build locally

```bash
npm run tauri build
```

Installers and packages appear under `src-tauri/target/release/bundle/` for your platform.

## CI vs GitHub Releases

- **Every push to `main`** (and PRs) runs the **Build** workflow. It uploads **workflow artifacts** only — those sit under the Actions run and are not the same as [Releases](https://github.com/mahiatlinux/polishd/releases).
- **GitHub Releases** (installers attached to a version on the Releases tab) are created when you **push a version tag** that matches the app version in `src-tauri/tauri.conf.json` (e.g. version `0.1.0` → tag `v0.1.0`):

```bash
git tag v0.1.0
git push origin v0.1.0
```

The **Release** workflow then builds Linux, Windows, and macOS and uploads bundles to that release. In the repo **Settings → Actions → General → Workflow permissions**, use **Read and write** so `GITHUB_TOKEN` can publish the release.

## Configuration

| Key       | Purpose            |
| --------- | ------------------ |
| `api_key` | OpenRouter API key |
| `hotkey`  | Polish shortcut    |
| `theme`   | `light` or `dark`  |

Get a key at [openrouter.ai/keys](https://openrouter.ai/keys).

## Shortcuts (defaults)

| Action             | Linux / Windows | macOS          |
| ------------------ | --------------- | -------------- |
| Polish selection   | Ctrl+Shift+E    | Cmd+Shift+E    |
| Custom instruction | Ctrl+Shift+D    | Cmd+Shift+D    |

You can change the polish shortcut in the app settings.
