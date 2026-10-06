# student-planner-leptos

Student day planner in Rust: Leptos 0.8 (SSR + hydration), Axum 0.8, SQLx on Postgres. Version 2
of the app: a port of the Java draft in `draft/` (read-only, gitignored). Version 3 is planned in
C or C++. Docker + Render deployment is set up in the repo, but the app is not deployed yet.

## The idea
Give a student a realistic picture of their day. The Daily tab is one Google Calendar-style
timeline from wake to bed. Classes and shifts are gray blocked time. The gaps between them are
"slices" of free time. Tasks are blue event blocks sized by duration, so the student sees how much
of the day is really free and how many tasks actually fit. A red "You are here" line marks now.
Inspired by Google Calendar and interstitial journaling: log what you are doing as you go.

## Working agreement (how we work in chat)
- Claude writes all the code. Ricky applies it, runs verify, and reviews. He is learning Rust
  through this project and wants to absorb design choices, not write the code himself.
- Before coding a task: ask at most 3 questions. Then give small steps, each with a "Done when"
  (verify result with exact test counts, plus what to see in the browser for visual steps).
- Each step gets a short design note: why it's built this way, and what carries over to v3.
- Sprinkle real-world SE practices lightly, at most one per step. No homework-style tickets.
- Ricky pastes only the first error from verify and a screenshot for visual steps.
- When Ricky writes "diff" he means "different/difference", not git diff.
- At the end of a phase, Claude updates this file and calls for a new session
  (fresh repo zip + this file).

## Commands
- `./scripts/dev.sh` starts Postgres (Docker) and `cargo leptos watch` on port 3000. Open the
  forwarded `-3000` URL from the Ports tab, not localhost. Only run it once verify passes.
- `./scripts/verify.sh` runs fmt, both builds, clippy, and all tests, then prints ALL CHECKS PASSED.
  Run it after every step and before every commit. Full log: `verify.log`.
- `./scripts/ctx.sh FILE...` bundles files into `context.txt` to attach to a chat.
- After pasting snippets from chat, `grep -n ',`$' src/*.rs tests/*.rs` catches stray backticks.

## Rules that have bitten us
- The toolchain is a pinned dated nightly in `rust-toolchain.toml`. Do not change it.
- Leptos 0.8 APIs only: `signal`, `RwSignal`, `Resource::new`, `#[server]`. No `create_signal`.
- A component must never render itself (an App -> HomePage -> App loop crashed the compiler).
- Big `view!` trees hit the compiler's query depth limit. `#![recursion_limit = "256"]` is set in
  both `lib.rs` and `main.rs`. Split large views into their own components.
- `cargo check` can pass while the test build fails (layout is only computed in a real build).
- Components with more than 7 props need `#[allow(clippy::too_many_arguments)]`.
- Everything except `db.rs` and `main.rs` also compiles to wasm32: no tokio, sqlx, or std::fs there.
  Server-only code goes behind `#[cfg(feature = "ssr")]`.
- SQL is runtime-checked (no `query!` macros) so Docker builds need no database. Schema changes are
  new files in `migrations/`. Never edit a migration that has already run.
- `style/main.scss` started as the Java draft's CSS. Its `* { font-family }` rule hits every element,
  so any new text layer must set its font on its children too.
- Clippy runs with `-D warnings`: no unused imports or dead code, no items after a `#[cfg(test)]`
  module, and use `x.is_multiple_of(n)` instead of `x % n == 0`.
- Times are minutes since midnight (u32). Late-night times can pass 1440 on the day clock.
  Dates are ISO strings.
- The day runs 4 AM to 4 AM Pacific, computed in Postgres. `DAY_CUTOFF` in `db.rs` and
  `DAY_CUTOFF_MIN` in `plan.rs` must stay in sync.
- Tasks with a start time are placed by start time, never by stored slice id (slice ids are
  positional and change when the schedule changes). Only untimed tasks use the stored slice id.
- Decisions live in pure, tested functions (`plan.rs`, `slices.rs`, `editor.rs`, `notes.rs`,
  `timeline.rs`). UI files only draw what they are told. Test first, then wire to the UI.
- One vertical scale everywhere: `PX_PER_MIN = 1.5` in `timeline.rs`.
- Disk is tight in Codespaces. Build output lives in /tmp/target (symlinked as `target`).
  Do not run Docker builds casually.

## Done means
`./scripts/verify.sh` ends with ALL CHECKS PASSED. If a check fails twice for the same reason, stop
and report the exact error instead of working around it. Keep terminal output summaries short.

## Where things are
plan.rs layout engine (placing tasks, overflow, capacity, fit count, past/now/later, move target) ·
board.rs single-day timeline (draws what plan.rs decides) · slices.rs slice engine ·
model.rs shared types · db.rs + api.rs database and server functions · app.rs Daily tab ·
editor.rs + editor_modal.rs notes editor and edit dialog · history_view.rs ·
schedule_view.rs weekly schedule editor · timeline.rs hour marks, labels, shared scale ·
migrations/ (0001 tasks, 0002 archive, 0003 settings, 0004 schedule + the real timetable,
0005 task duration).

## Current state (update at the end of every phase)
Done: daily plan from the database, notes with live checklists, IDE-style notes editor with
customizable buttons, history/archive/reschedule, editable weekly schedule (weekly blocks, one-time
blocks, split times, wake/bed times).

Phase 3i, steps 1 to 5c done:
- `tasks.duration_minutes` (default 15, 1 to 1440).
- `plan.rs`: timed tasks sit at their start time, overlaps are pushed down, untimed tasks stack
  from the slice start around timed ones, overflow past a slice is measured, capacity per slice,
  "N of M to-dos fit" for the day.
- The day turns over at 4 AM. `get_day` returns the current minute (`DayData.now`) for today only.
- Daily tab is one timeline: gray class blocks, blue task blocks, time before now shaded, the
  block or free gap that "now" falls in gets a blue border, red "📍 You are here" line, page
  scrolls to it on load. Past tasks are dimmed with a "Move to …" button. Capacity labels show at
  the top of a free gap when its first 20 minutes are empty, otherwise as a hover tooltip.

Expected test counts: 58 library, 17 database.

Known issues and loose ends:
- The now line is static: it is placed at page load and does not move (step 6 fixes this).
- Notes no longer render inside timeline blocks. Click a block to open the editor (step 7 makes
  that dialog the home for notes). Unassigned and completed tasks still show notes inline.
- `BlockedTimeline` in `timeline.rs` is no longer used by the Daily tab.
- New tasks still get a placeholder `DEFAULT_DURATION` in `AddTaskForm` and `editor_modal.rs`.
- CI backlog: pin `runs-on: ubuntu-24.04` (ubuntu-latest moves to Ubuntu 26 on Oct 19, 2026);
  `cargo fmt --all` should be `cargo fmt --all -- --check`; database tests do not run in CI
  (needs a Postgres service container). CI runs #9 and #10 failed from GitHub runner outages,
  not code; re-run them.

## Decisions made in 3i (short decision log)
- Overlapping tasks: the later one is pushed down to start when the earlier one ends.
- "How many tasks fit": untimed tasks, oldest first, counted against free time left after now
  (a totals estimate, not gap-by-gap packing).
- Day cutoff: 4 AM. Unfinished tasks at day's end go to History as missed (unchanged).
- Past slices: a "Move to …" button now; drag-and-drop later. A moved task becomes timed: at the
  current minute if a slice is running, else at the start of the next slice.
- Layout pivot: one timeline for the whole day instead of a card per slice.
- Quick log start time: the next whole minute (1:03:20 PM logs at 1:04 PM), editable.
- Edit dialog: TickTick's task panel as an influence, but the IDE look and feel is the priority.

## Next task: finish 3i
Step 6, moving now line and quick log:
- The now line moves every minute in the browser (start from `DayData.now`, tick client-side).
- Quick log input "What are you doing now?" starts a task at the next whole Pacific minute
  (server computes it with `db::now_minutes`) with a 15 minute default and duration chips
  (15/30/45/60/90). Build it as its own component so `DayPlan` stays shallow.
Step 7, TickTick-style edit dialog, IDE first:
- Slim top bar: checkbox, start time, duration, priority flag. Big title. The IDE editor fills the
  body. Quiet footer.
- Duration field saves through `TaskUpdate.duration_minutes`. Replace the `DEFAULT_DURATION`
  placeholders with real choices.
Then the end-of-phase review: what we built, how we built it, and the lessons to carry into v3.

## Roadmap after 3i
3h editor line features: line numbers, current-line highlight, Bold and Code act on the current line
(an exception to the new-line rule). Then deploy to Render. Then drag-and-drop between slices,
ghost "stamp in" suggestions, templates, three dates per task (deadline, scheduled, reminder), then
slice alerts and call-style notifications.
