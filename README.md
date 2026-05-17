# Daily Planner

A Tauri desktop app for Linux/X11 that locks the screen until you plan your day, and locks it again in the evening to review what you did.

## How it works

### Morning lock

Fires at the configured time (default 05:00). If no plan exists for today, the app opens fullscreen and attempts to grab X11 keyboard and pointer input until the plan is submitted. If the X11 grab fails after its retry attempts, the app still opens but runs without the hard input grab.

- Type a task and press `Enter` (or `+`) to add it.
- Select a category before adding: **Personal**, **9-5**, **Freelance**, or **Community**.
- Tasks are grouped by category. Use ↑/↓ to reorder within a group.
- At least one task is required to submit.
- Daily intention and notes are optional.

If the computer is off at the scheduled time, the morning lock fires on the next login (the morning timer uses `Persistent=true`).

### Evening review

Fires at the configured time (default 18:00). If today's plan exists but hasn't been reviewed, the same hard lock appears showing today's tasks grouped by category.

- Check off completed tasks (saved to disk immediately).
- Add optional reflection notes.
- Submit to mark the day as reviewed and open the dashboard.

The evening timer does not use `Persistent=true` — a day you miss is a day you miss.

### Dashboard

Open the dashboard directly with:

```bash
daily-planner --dashboard
```

Shows:

- **This week** — days planned, tasks done/total, completion percentage. An AI Review button sends this week's plans to Claude and returns a focused ≤120-word analysis: what you're completing, what's slipping, and one sharp recommendation.
- **Today** — current plan with live-toggleable checkboxes, grouped by category.
- **Past plans** — collapsible cards for every previous day.
- **Settings** — configure morning/evening lock times and your Claude API key. Saving patches the installed systemd timer files and reloads them automatically.

If the systemd timers are not installed, a banner appears at the top of the dashboard with a reminder to run `install.sh`.

## Development

Install dependencies:

```bash
npm install
```

Run in development:

```bash
npm run dev
```

Run Rust unit tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Build the release binary:

```bash
npm run build
```

The release binary lands at:

```
src-tauri/target/release/daily-planner
```

## Install

```bash
./install.sh
```

The install script:

- Builds the app if the release binary does not exist.
- Copies the binary to `~/.local/bin/daily-planner`.
- Installs five systemd units to `~/.config/systemd/user/`.
- Enables the login service, morning timer, and evening timer.

After installation, lock times can be changed from the Settings panel inside the dashboard — no manual systemd editing needed.

## Data

Plans are stored as JSON files:

```
~/.local/share/daily-planner/plans/YYYY-MM-DD.json
```

Config (lock times, API key) is stored at:

```
~/.local/share/daily-planner/config.json
```

Old plan files that predate categories are loaded without migration — the deserializer handles plain strings, `{text, done}` objects, and the current `{text, done, category}` format.

## Platform

Linux + X11 only. The lock uses `override_redirect` to bypass the window manager and `XGrabKeyboard`/`XGrabPointer` to capture input. Wayland is not supported.
