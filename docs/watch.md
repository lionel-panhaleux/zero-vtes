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

## On a date

- **2026-11-14 — the seventh upkeep's proposals** (asked in the digest,
  2026-11-11: the changed-decision rule for every line with its
  `harness.py stale` check; add-cards' re-check of "not in the pool"
  clauses; DESIGN §11's test counts dropped; the session settings
  committed): unanswered → asked again in the digest; each answer → made,
  or an engine-board line when it is code; then the item is deleted.

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
- **50 of the owner's cross-table combats harvested** (the `humans` group
  of `waste - 0 data/server-records`, in the detached checkout, its
  combats with no seat ousted: the report keeps the bots' groups only;
  30 on 2026-11-09): the pairing taught cross-table restraint in neither
  p11 nor p12 and is off from p13 (owner, 2026-10-17); the owner's 4-seat
  games are its target (at 4 seats, cooperate with the cross-table seat);
  at 50 → Z-train's `intake`: compare the owner's cross-table play with
  the served network's, decision by decision, and propose the lever it
  points at (done when the comparison is in DESIGN §11.2 and the owner
  has chosen).
