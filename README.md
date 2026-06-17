# lifelog

> Commit your days. A git-inspired daily logger with a contribution grid.

`lifelog` borrows git's vocabulary - `commit`, `push`, the contribution grid - and points it at your life instead of your code. You log what you _did_ (past tense; it's a done-tracker, not a to-do list), `push` to seal the day, and watch a grid fill in over the month. Terminal first, with a GUI and mobile app to follow. Built in Rust.

## How it works

The daily loop has two moves:

- **commit** - log something you did. Saved to the database the instant you type it.
- **push** - seal the day. Lights one tile on the contribution grid.

## The grammar

A commit is `verb` + `subject`, with everything else optional:

```
make shipped the dark mode toggle
work:report finished the Q2 draft -t 120
body ran 5k -t 32
life groceries + pharmacy -s 1450
```

- The **verb** is one of the five pillars (below). It's the command itself - no `-m`, no quotes.
- A **`:scope`** narrows the verb when you'll want to filter later (`work:report` vs `work:meeting`). Optional.
- **Flags** become durable trailers, stored as structured data. The defaults:
  `-t` Mins · `-s` Spent · `-d` Dist · `-l` Loc · `-c` Carbs · `-k` Kcal · `-p` Protein · `-w` With · `-r` Reps.
- An **empty subject** (just the verb) opens an editor for a longer note.
- A commit can be **starred** as the day's highlight.

Other commands: `log` (today's commits), `push` (seal the day), `undo` (drop the last commit).

## Pillars

Five types: **body**, **work**, **make**, **mind**, **life**.

- **body** - anything that feeds the physical self: workouts, walks, meals, rest, the morning matcha.
- **work** - the job, the career, focused output.
- **make** - things you create: code, craft, the bindery.
- **mind** - reading, learning, contemplation, the quiet inner stuff.
- **life** - everything else: errands, commute, calls, outings, admin.

These are a guide, not a rulebook. Anything that doesn't obviously fit lands in
**mind** or **life** - whichever feels right. The goal is to never get stuck
deciding; when in doubt, `life` it.

## The contribution grid

One tile per day. Emerald on black, brighter the busier the day.

## Tech

- **Rust**, organized as a cargo workspace: `engine` (the shared core - grammar, types, storage) and `cli` (the first frontend).
- **SQLite** via `rusqlite` for storage. Commits, trailers, and a small `days` table for the sealed flag.
- **Ratatui** for the terminal UI.
- **Tauri** for the desktop GUI and mobile app, reusing the same `engine` core.

## Status

Early days - the core is taking shape.

- [x] Cargo workspace (`engine` + `cli`)
- [x] Commit grammar parser
- [x] Graceful error handling (`Result` + typed errors)
- [x] SQLite persistence
- [x] `log` command (read today's commits back)
- [ ] `push` / `undo` commands
- [ ] Accept typed input (log a real commit)
- [ ] Configurable flags (user-defined trailers)
- [ ] Contribution grid
- [ ] Terminal UI (Ratatui)
- [ ] Desktop GUI + mobile (Tauri)

## Build

```
cargo run -p cli
```

---

This is a personal project. Started June 16th, 2026.
