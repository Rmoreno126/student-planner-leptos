# student-planner-leptos

Student day planner in Rust: Leptos 0.8 (SSR + hydration), Axum 0.8, SQLx on Postgres. A port of the
Java app in `draft/` (read-only, gitignored). Docker + Render deployment is set up in the repo, but
the app is not deployed yet.

## The idea
Give a student a realistic picture of their day. Classes and shifts are blocked time. The gaps between
them are "slices" of available time. Tasks are event blocks that take up part of a slice, so the
student sees how much of the day is really free and how many tasks actually fit (even if only one
does). Inspired by Google Calendar event blocks and interstitial journaling: log what you are doing as
you go, with a default duration.

## Commands
- `./scripts/dev.sh` starts Postgres (Docker) and `cargo leptos watch` on port 3000. Open the
  forwarded `-3000` URL from the Ports tab, not localhost.
- `./scripts/verify.sh` runs fmt, both builds, clippy, and all tests, then prints ALL CHECKS PASSED.
  Run it before every commit. Full log: `verify.log`.
- `./scripts/ctx.sh FILE...` bundles files into `context.txt` to attach to a chat.

## Rules that have bitten us
- The toolchain is a pinned dated nightly in `rust-toolchain.toml`. Do not change it.
- Leptos 0.8 APIs only: `signal`, `RwSignal`, `Resource::new`, `#[server]`. No `create_signal`.
- A component must never render itself (an App -> HomePage -> App loop crashed the compiler).
- Everything except `db.rs` and `main.rs` also compiles to wasm32: no tokio, sqlx, or std::fs there.
  Server-only code goes behind `#[cfg(feature = "ssr")]`.
- SQL is runtime-checked (no `query!` macros) so Docker builds need no database. Schema changes are
  new files in `migrations/`. Never edit a migration that has already run.
- `style/main.scss` started as the Java draft's CSS. Its `* { font-family }` rule hits every element,
  so any new text layer must set its font on its children too.
- Clippy runs with `-D warnings`: no unused imports or dead code, no items after a `#[cfg(test)]`
  module, and use `x.is_multiple_of(n)` instead of `x % n == 0`.
- Times are minutes since midnight (u32), dates are ISO strings, "today" is Pacific time computed in
  Postgres.
- Slice ids are positional ("slice_2"), so they change whenever the weekly schedule changes. Place
  tasks by start time, never by a remembered slice number, once start times exist.
- Disk is tight in Codespaces. Build output lives in /tmp/target (symlinked as `target`).
  Do not run Docker builds casually.
- Put each behavior in a pure function with a unit test first (see `slices.rs`, `editor.rs`,
  `notes.rs`, `timeline.rs`), then wire it to the UI.

## Done means
`./scripts/verify.sh` ends with ALL CHECKS PASSED. If a check fails twice for the same reason, stop
and report the exact error instead of working around it. Keep terminal output summaries short.

## Where things are
slices.rs slice engine · model.rs shared types · db.rs + api.rs database and server functions ·
app.rs Daily tab · editor.rs + editor_modal.rs notes editor · history_view.rs ·
schedule_view.rs weekly schedule editor · timeline.rs calendar-style blocked time ·
migrations/ (0001 tasks, 0002 archive, 0003 settings, 0004 schedule + the real timetable).

## Current state (update at the end of every phase)
Done: daily plan from the database, notes with live checklists, IDE-style notes editor with
customizable buttons, history/archive/reschedule, editable weekly schedule (weekly blocks, one-time
blocks, split times, wake/bed times), gray calendar-style blocked time with hour lines.
Expected test counts: 44 library, 15 database.
Known issue: tasks keep the slice id they were created with, so after schedule edits some sit in the
wrong slice. Fixed by 3i.

## Next task: 3i, tasks as event blocks
Goal: tasks inside a slice look like the gray class blocks (same hour lines), sized by duration, so
the slice shows how much time is planned and how much is left.
- Migration 0005: `duration_minutes INTEGER NOT NULL DEFAULT 15` on tasks.
- A task with a start time is drawn at that time, with height proportional to its duration. The slice
  is whichever one contains the start time. A stored slice id only matters for tasks with no start
  time, which stack back to back from the slice start.
- A task that runs past the end of its slice shows an overflow note ("10 min past this slice").
  Overflow is fine: one task may fill a whole slice.
- Slice header shows capacity: "3h 10m free · 45m planned · 2h 25m left". The day shows how many tasks
  fit.
- Quick log, interstitial journaling style: an input "What are you doing now?" starts a task at the
  current Pacific time (to the minute) with a 15 minute default and optional duration chips
  (15/30/45/60/90). Logging at 1:03 PM places it at 1:04 or from now, editable. A moving "now" line
  shows in the current slice.
- Duration field in the edit dialog.
- Pure layout function first, with tests (placing tasks in a slice, overflow, remaining time).
- Ask the user before coding (max 3 questions): when two tasks overlap, push the later one down to
  start when the first ends, or show them side by side?

## Roadmap after 3i
3h editor line features: line numbers, current-line highlight, Bold and Code act on the current line
(an exception to the new-line rule). Then deploy to Render. Then ghost "stamp in" suggestions,
templates, three dates per task (deadline, scheduled, reminder), then slice alerts and call-style
notifications.
