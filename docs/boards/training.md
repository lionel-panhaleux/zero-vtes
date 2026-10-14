# Training board

Goal: a really great opponent (roadmap objective A), trained in runs from
a pinned worktree (`../zero-vtes-pN`) and fed by the games humans play on
`engine.krcg.org`. Skill: `training`. Docs: DESIGN §5 (the agent), §11.2
"Policy surface and training runs" (every run's command, numbers and
lessons), `data/records/README.md`, the harvest reports
(`data/server-records/reports/`, ignored).

Status: `todo` · `doing` · `blocked:<what>` · `done`. Order: the beta's
bot first (p11 finished, measured, served), then the loop the beta's games
feed (harvest, report, promotion, evaluation sets), then the levers toward
a great opponent on any deck, then speed and layout.

| # | Item | Why / source | Status |
|---|---|---|---|
| T1 | p11 (`runs/p11`, `../zero-vtes-p11` at 45debc9): monitored to its end, evaluated (`waste` against p10@3840, `decks.rs` per deck and p11 against three p10), the "p11 run" note in DESIGN §11.2; served on the owner's call (weights by sha prefix, the previous kept) | DESIGN §11.2 p11 | doing |
| T2 | p11's two questions measured on its snapshots: cross-table restraint (`--partner-rate 0.5`; p10: a bleed took 1.36 pool from its prey, 0.14 cross-table, blood lost 0.96 / 1.19 adjacent / cross) and Wider View (p10: taken at 27% of offers, 26% of uses under 4 crypt cards; the owner 12%, none); a training signal if Wider View stays (no nudge: owner) | DESIGN §11.2 p11; findings E7 | todo |
| T3 | The harvest on a schedule: `fetch_games.py --finished` then `games_report.py`, daily (a launchd job or a scheduled agent; none found locally in launchd or cron: check the scheduled routines before adding one), the report read after each pull | training skill ("Harvest (scheduled)"); harvest (training session) §3 | todo |
| T4 | `waste` takes a records directory (today record paths), and the report gains the `waste` lines of the bot seats in the pulled games | harvest (training session) §3 | todo |
| T5 | Promotion as a script: curated harvested games (the owner's, the flagged, human wins worth learning from) copied to `data/records` after a check of what a stranger's record holds | training skill (Promotion); harvest (training session) §3 (privacy) | todo |
| T6 | Harvested games as training input: `--bc` and `--starts` take a list file or a filter (finished, a human seat, not abandoned or short), never the raw `data/server-records` | training skill (Use) | todo |
| T7 | The owner's games weighted up in `--bc` (a per-player weight; today every human seat weighs the same) | training skill (To build) | todo |
| T8 | An evaluation set of "positions where a human beat the bot": the decision points before human wins, named and never trained on, scored per network (agreement with the human's choice, the outcome replayed) | training skill (To build); DESIGN §11.4 P4 (evaluation vs humans) | todo |
| T9 | The human win rate as objective A's measure: per weights and bot profile from the report's totals, with sample sizes, quoted in each run's DESIGN note | DESIGN §11.4 P4; objective A | todo |
| T10 | Weakness mining per pull: a recurring bad choice becomes a `waste` counter first, a nudge only if it persists across runs and the owner agrees | training skill (Weakness mining) | todo |
| T11 | Bots on any deck: per-deck strength (`decks.rs`) on a sample of kept TWDA decks the runs never seated, then the next run's deck mix | objective B (a bot with a player's deck, a full bot table to test decks) | todo |
| T12 | Take the `zvtes-env` crate (E24): `py` and `policy` imports, the observation's id tables owned there | architecture; harvest (training session) §5 | blocked:E24 |
| T13 | Paths in the observation (E28): a new `obs::Version` or a slot shown unused | DESIGN §11.3 paths | blocked:E28 |
| T14 | Iteration speed, the next levers: ~80% of rollout inference is the frozen opponents' (p11: ~2 min an iteration, 38 s rollout, 76 s update); measure batching their forwards or lighter frozen opponents against the owner's 2–5× (reached ~2.3×) | DESIGN §11.2 "Faster iterations"; architecture | todo |
| T15 | The early survival shaping of §5 (the reward is tournament scoring only, the game-win weight 5 since p9): kept, tried or dropped (the owner) | DESIGN §5, §11.4 P3 | blocked:owner |
| T16 | Strength levels players pick (the served weights at a temperature, a weaker snapshot), named as server profiles (E18) | objective A; beta | blocked:owner |
