# polishd

Desktop helper that **polishes selected text** using AI (via [OpenRouter](https://openrouter.ai/)). Select text in any app, press a global shortcut, and the polished result is pasted back. Optional **custom instruction** modal (**Ctrl+Shift+D** on Linux/Windows, **Cmd+Shift+D** on macOS) runs your own prompt on the selection.

Stack: **Tauri 2**, **React 19**, **TypeScript**, **Vite**.

## Features

- System tray icon with **Settings** and **Quit**
- Configurable **global hotkey** (default **Ctrl+Shift+E**)
- **OpenRouter API key** stored locally (see [Configuration](#configuration))
- **Light / dark** theme in the settings window

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS recommended)
- [Rust](https://www.rust-lang.org/tools/install) (`rustc`, `cargo`)
- OS packages Tauri expects for your platform — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

On **Linux**, you typically need WebKitGTK and related dev packages (the Tauri docs list per-distro commands).

## Setup

```bash
git clone <your-repo-url>
cd publishd-oss
npm install
```

## Development

```bash
npm run tauri dev
```

Runs the Vite dev server and the Tauri app with hot reload for the frontend.

## Build

```bash
npm run tauri build
```

Frontend: `npm run build` (runs `tsc && vite build`).  
Native app: `npm run tauri build` (uses `beforeBuildCommand` from `src-tauri/tauri.conf.json`).

> **Note:** In this repo, `bundle.active` is set to `false` in `tauri.conf.json`. Enable bundling there when you want installers.

## Configuration

Settings are stored in the app data directory as **`polishd.json`** (Tauri store plugin), including:

| Key        | Purpose                                      |
| ---------- | -------------------------------------------- |
| `api_key`  | OpenRouter API key                           |
| `hotkey`   | Global shortcut for polish                   |
| `theme`    | `light` or `dark`                            |

Get an API key at [openrouter.ai/keys](https://openrouter.ai/keys).

## Shortcuts (defaults)

| Action              | Default (Linux / Windows) | macOS        |
| ------------------- | --------------------------- | ------------ |
| Polish selection    | **Ctrl+Shift+E**            | **Cmd+Shift+E** |
| Custom instruction  | **Ctrl+Shift+D**            | **Cmd+Shift+D** |

Change the polish shortcut in the app; the transform shortcut is fixed in the UI copy but uses the platform-appropriate modifier.

---

## GitHub CLI (`gh`) — install and push easily

The [GitHub CLI](https://cli.github.com/) lets you create repos, authenticate, and push from the terminal without juggling the website as much.

### Install on Linux (Debian / Ubuntu)

**Option A — official apt repository** (current packages):

```bash
sudo apt update
sudo apt install -y gh
```

If `gh` is not in your distro’s repos, use GitHub’s documented install: [Installing gh on Linux](https://github.com/cli/cli/blob/trunk/docs/install_linux.md) (apt repo, yum, manual tarball, etc.).

**Option B — download** — [Releases](https://github.com/cli/cli/releases) (`.deb` / tarball).

### Log in

```bash
gh auth login
```

Follow the prompts (HTTPS or SSH, browser or token). After this, `git push` to GitHub often works without storing passwords manually.

### Create the repo and push (first time)

From the project root, after your first commit on `main`:

```bash
gh repo create polishd --public --source=. --remote=origin --push
```

Adjust `polishd` to your repo name. Use `--private` instead of `--public` for a private repo.

If the repo already exists on GitHub:

```bash
git remote add origin https://github.com/YOUR_USER/YOUR_REPO.git
git push -u origin main
```

### Useful commands

```bash
gh repo view --web          # open repo in browser
gh pr create                # open a PR from a branch
gh auth status              # check login
```

---

## License

Add a `LICENSE` file if you open-source this project.
