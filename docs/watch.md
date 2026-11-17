# Watch: what is checked, not done

Items that never complete by effort: a schedule, a date, an outside
event. None of them is a board line (CLAUDE.md "Harness"). Each says
**when**, **what to check** and **what each outcome does**; a dated item
or an event's item is deleted when it fires, a recurring one faces
upkeep's eviction like a line.

The clock is one Z-main schedule, not one per item: a daily cron (09:00)
whose prompt is "read docs/watch.md and fire what is due". Session crons
expire after seven days and live in one session: Z-main checks it with
`CronList` at every start and recreates it when it is gone. A fired item
runs in Z-main and hands its result to the line's session (SendMessage)
or to `intake`; a session cannot be woken by another's cron.

Whether an item is due is read from its evidence (a report's date, a
counter), never from state kept here. The harvest and the sweeps run on
`main`'s tip with no edit, so not in the shared checkout (the lines'
work in progress is there): Z-main keeps a detached checkout for them
(and for a release's checks), `git worktree prune` then `git worktree
add --detach .claude/worktrees/pin main` when gone, `git -C
.claude/worktrees/pin checkout --detach main` before each run; both
write to the main checkout's ignored `data/`.

## Recurring

- **Daily — the harvest**: the main checkout's
  `data/server-records/reports/total.md` not rewritten today (a day with no
  new game writes no day's report) → run the training skill's
  "Harvest" (`python3 scripts/harvest.py`, in the detached checkout). Its
  weaknesses and anything it shows → Z-train's `intake`.
- **Daily — p18 against the humans** (served 2026-11-12 on a gain at bot
  tables only, owner): the day's report, the humans' win rate and VP
  against p18 (`0535d0a0be63`) beside p17's (`f426338f6c80`). Worse beyond
  the report's small-sample mark → the owner (serving p17 again is a
  deploy, `runs/served-prev-2026-11-12-p17.zvp`); otherwise nothing.
  Deleted when a run replaces p18.
- **Daily — the outside's reports**: the players' feedback since the last
  check (`GET /admin/feedback?since=`, intake "Sources"), `gh issue list`
  and `gh pr list` here and on `lionel-panhaleux/succubus-club` → each new
  one through `intake`.
- **Daily — upkeep**: `python3 scripts/harness.py due` says due → Z-main
  runs `upkeep`.
- **Weekly — the deep sweeps**: no report this week under the main
  checkout's `data/sweeps/` → `python3 scripts/sweeps.py`, in the
  detached checkout. Refused (a training run) → retried the next day; a
  failure → Z-engine's `intake`.

## On a date

- **2026-11-20 — the eighth upkeep's proposals** (asked in the digest,
  2026-11-17: the ledger's `n/a` reasons re-checked with the "not in the
  pool" clauses; a batch's cross-card readings in rules-qa's Index;
  protocol.md's counts dropped; a measured run's entry cut to its
  settled lines; "Paths in the observation" to the open beta's watch
  item): unanswered → asked again in the digest; each answer → made, or a
  line on its board when it is code; then the item is deleted.

## On an event

- **The platform opened to more players** (owner, 2026-11-09: planned
  for about 2026-11-23; the roadmap's phase): the online A/B — each game
  on engine.krcg.org served the current or the candidate weights at
  random, the humans' win rate and VP compared by weights in the harvest
  report — is the owner's to start, not before the opening. Opened → the
  owner asked in the digest; started with a candidate → Z-engine's
  `intake` (the server's random choice of weights, the report's split by
  arm) and Z-train's (the candidate), and the item is deleted; no
  candidate yet → the item stays until one exists.
