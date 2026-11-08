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
(and for a release's checks), `git worktree prune` then `git worktree add
--detach <job tmp>/pin main` when gone (a restart gives a new job tmp),
`git -C <job tmp>/pin checkout --detach main` before each run; both write
to the main checkout's ignored `data/`.

## Recurring

- **Daily — the harvest**: no report of the day under the main
  checkout's `data/server-records/reports/` → run the training skill's
  "Harvest" (`python3 scripts/harvest.py`, in the detached checkout). Its
  weaknesses and anything it shows → Z-train's `intake`.
- **Daily — p17's trial** (served as a trial, owner 2026-11-06: back to
  p14 if the human games worsen): the day's report, the humans' win rate
  and VP against p17 beside p14's. Worse beyond the report's small-sample
  mark → the owner (serving p14 again is a deploy); otherwise nothing.
  Deleted when a run replaces p17.
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

## Dated

- **2026-11-15 — the owner's answers to upkeep's proposals of
  2026-11-08** (the commit carrying that pass's `Upkeep:` trailer): each
  accepted one made then (a skill line, a page edit), each declined one
  dropped; each still unanswered re-dated a week on in a new item. This
  item is deleted as it fires.

## On an event

- **50 of the owner's cross-table combats harvested** (the report's `waste`
  humans group, no seat ousted; 11 on 2026-10-15): the pairing taught
  cross-table restraint in neither p11 nor p12 and is off from p13 (owner,
  2026-10-17); the owner's 4-seat games are its target (at 4 seats, cooperate
  with the cross-table seat); at 50 → Z-train's `intake`: compare the owner's
  cross-table play with the served network's, decision by decision, and propose
  the lever it points at (done when the comparison is in DESIGN §11.2 and the
  owner has chosen).
