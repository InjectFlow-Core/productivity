# Daily Planner

A Tauri desktop app for Linux that opens at scheduled times to help you plan your morning and review your evening.

## Motivation

My work and schedule have felt scattered lately, so I built this app to force a little structure into my day.

The first time I open my laptop to work, the it takes over the screen and requires me to write my plan for the day before I can do anything else.

Once I submit it:

- A small sticky window stays at the side of my screen, reminding me what I need to get done. I can minimize or close it when needed.
- At the end of the day, a review dashboard appears so I can mark what I completed.
- The next morning, unfinished tasks from the previous day show up as suggestions for the new day. I can carry them over or ignore them.
- At the end of the week, an AI reviews everything I worked on and gives me a clear sense of how my week actually went. I can also request that review anytime.

It is more of a personal tooI needed something more forceful than a regular to-do app.
## How it works

### Morning form

Fires at the configured time (default 05:00). If no plan exists for today, the app opens fullscreen. Fill in your tasks and submit — the window closes automatically.

- Type a task and press `Enter` (or `+`) to add it.
- Select a category before adding: **Personal**, **9-5**, **Freelance**, or **Community**.
- Tasks are grouped by category. Use ↑/↓ to reorder within a group.
- At least one task is required to submit.
- Daily intention and notes are optional.

If the computer is off at the scheduled time, the morning form fires on the next login (the morning timer uses `Persistent=true`).

### Evening review

Fires at the configured time (default 18:00). If today's plan exists but hasn't been reviewed, the app opens fullscreen showing today's tasks grouped by category.

- Check off completed tasks (saved to disk immediately).
- Add optional reflection notes.
- Submit to mark the day as reviewed — the dashboard opens automatically.

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
- **Settings** — configure morning/evening times and your Claude API key. Saving patches the installed systemd timer files and reloads them automatically.

If the systemd timers are not installed, a banner appears at the top of the dashboard with a reminder to run `install.sh`.

### Sticky tasks

Open a compact always-on-top view of today's tasks with:

```bash
daily-planner --sticky
```

The sticky window shows today's completion count and live-toggleable task checkboxes. It opens automatically after you submit the morning plan, and the login startup flow opens it when a plan already exists for today. Close it when you do not need it; reopen it from the **Daily Planner Sticky** launcher or with the command above.

## Development

Install dependencies:

```bash
npm install
```

Run in development:

```bash
npm run dev
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

After installation, times can be changed from the Settings panel inside the dashboard — no manual systemd editing needed.

## Uninstall

```bash
./uninstall.sh
```

Stops and disables all systemd units, removes the binary, and removes the desktop entry and icon.

## Data

Plans are stored as JSON files:

```
~/.local/share/daily-planner/plans/YYYY-MM-DD.json
```

Config (scheduled times, API key) is stored at:

```
~/.local/share/daily-planner/config.json
```

## Platform

Linux. The morning and evening forms open as a standard fullscreen window — no input grabbing or window manager bypass.

## Next feature?

- [x] You should be made to make decisions about your incomplete tasks for the previous day.
