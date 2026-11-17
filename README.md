# Onyx

Onyx is a desktop app for getting your game onto Steam. It wraps SteamCMD and the Steamworks build and upload workflow in a native GUI, so you can set up a project once and then ship builds without hand-editing VDF files or retyping long command lines. It is written entirely in Rust, with an immediate-mode UI built on egui and eframe.

It also ships with a headless command line mode, so the same projects and pipelines can run from CI or a scheduled task.

Onyx currently targets Windows. Parts of the code have Linux and macOS fallbacks, but Windows is the only platform that is built and released.

## What it does

### Projects

- Set up a project with your Steam app ID, content folder, build output folder, SteamCMD path, depots and branches.
- Import an existing setup instead of starting over: drop in an `app_build` or depot VDF file, or point Onyx at a content folder and let it detect depots, target OS and engine (Unity, Unreal or Godot).
- Mark branches as Dev, Beta or Live. Live branches get an extra confirmation before anything is pushed.
- Set ignore patterns, a description template with variables, a version file and an automatic build counter.
- Keep a release checklist per project, and export or import a project as JSON.
- A first-run setup wizard can download SteamCMD for you and walk through the basics.

### Building and uploading

- A guided build flow: preflight checks, review, upload, then finish.
- Preflight checks and a secret scanner look for common mistakes before you upload, such as private keys, AWS keys, Discord webhooks and hardcoded secret assignments. You can ignore individual findings.
- A manifest diff shows what changed since the last build, so you can see what you are about to ship.
- Live console output from SteamCMD, with progress through each stage (connecting, logging in, scanning, uploading, committing) and a running upload speed.
- A preview mode that runs the build without pushing it live.
- After an upload you can verify it by downloading the build and comparing file hashes against the manifest.
- Optional zip backups of each build, with restore and delete from the app.
- The machine can be kept awake while a build is running.

### Automation

- Pipelines are ordered lists of steps. The step types are:
  - Build game (with presets for Unity, Unreal and Godot)
  - Upload to Steam
  - Run command
  - Clean
  - Backup
  - Notify
  - Wait for approval
- Each step can be set to run always, or only when a previous step failed.
- Schedules can run a pipeline once, daily or weekly, or react to a trigger: files changing in a folder, or a new git tag appearing. If the app was closed when a run was due, you choose whether to catch up once or skip it.
- Schedules can also be registered with the operating system scheduler, so they run even when the app is not open.
- A runs view shows live and past pipeline runs, and approval steps pause a run until someone approves or rejects it.

### History and insight

- A build history table with details for every run, including duration and status.
- A patch notes tab that reads your git commits since the last build and groups them into notes. You can export them as Markdown, plain text or BBCode.
- An analytics tab with build trends and an advisor that points out large files that change often, duplicate files and high-entropy files that may already be compressed or encrypted.

### Notifications

- Desktop toast notifications on success and failure, configurable per outcome.
- Discord webhook messages for build results.

### Accounts and secrets

- Manage several Steam accounts and pick a default per project.
- Passwords and API keys are stored in the operating system credential store, not in plain text config files.
- Onyx works with SteamCMD's cached login. The app suggests using a dedicated build account with only the permissions it needs, rather than your personal Steam login.

### Doctor

The Doctor page checks the things that most often break uploads: SteamCMD presence, internet access, free disk space, write access to the output folder, cached login, project configuration and the tools your pipeline depends on. Some problems come with a fix button.

### Interface

- Five pages: Projects, Build, History, Automation and Settings, reachable with `Ctrl+1` to `Ctrl+5`.
- A command palette with fuzzy search for jumping to pages and running actions.
- Drag and drop import, a reduce motion option, and a quit guard that warns you if you close the app during a build.

## Command line

Run `onyx` with arguments to skip the GUI and use it headless.

```bash
onyx list-projects
onyx validate --project <name>
onyx doctor
onyx build --project <name> [--branch <branch>] [--desc <text>] [--preview] [--no-diff]
onyx pipeline list --project <name>
onyx pipeline run [--project <name>] [--pipeline <name>] [--schedule <id>] [--if-triggered]
```

`onyx --version` prints the installed version.

`onyx --open <folder>` is the one argument that opens the window instead: it shows the project whose content folder holds `<folder>` on the Build page, or offers to create a project from the folder when none does. GodotHub uses it to hand a finished export to Onyx.

Exit codes are meant for scripts and CI:

| Code | Meaning |
| ---- | ------- |
| 0 | Success |
| 1 | Build or run failed |
| 2 | Validation error or bad arguments |
| 3 | Steam login problem |
| 130 | Cancelled |

## Building from source

You need a recent stable Rust toolchain (the project uses the 2024 edition).

```bash
cargo build --release
```

The binary ends up in `target/release/onyx.exe`.

To run the tests, build the fake SteamCMD helper first, since some tests use it in place of the real thing:

```bash
cargo build --bin fake_steamcmd
cargo test
```

You also need [SteamCMD](https://developer.valvesoftware.com/wiki/SteamCMD) and a Steamworks account to upload anything. The setup wizard can download SteamCMD for you.

## Releases

Pushing a tag such as `v0.1.0` runs the release workflow on GitHub Actions. It checks that the tag matches the version in `Cargo.toml`, runs the tests, builds a Windows x64 release and publishes a portable zip and an installer (`-setup.exe`), each with a SHA-256 checksum.

## Contributing

Pull requests are closed for now.

```bash
git config core.hooksPath .githooks
```

## Fonts

Onyx bundles Inter, Manrope, JetBrains Mono and Phosphor icons. Their licenses are listed in `assets/fonts/LICENSES.md`.
