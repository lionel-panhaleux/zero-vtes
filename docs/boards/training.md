# Training board

Goal: a really great opponent (roadmap objective A), trained in runs from
a pinned worktree (`../zero-vtes-pN`) and fed by the games humans play on
`engine.krcg.org`. Skill: `training`. Docs: DESIGN §5 (the agent), §11.2
"Policy surface and training runs" (every run's command, numbers and
lessons), `data/records/README.md`, the harvest reports
(`data/server-records/reports/`, ignored).

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`. Order: the beta's bot first (p11 finished,
measured, served), then the loop the beta's games feed (harvest, report,
promotion, evaluation sets), then the levers toward a great opponent on
any deck, then speed and layout.

- **p11 to its end** [doing: Z-train]: `runs/p11` (`../zero-vtes-p11` at 45debc9) monitored to its end and evaluated (`waste` against p10@3840, `decks.rs` per deck, p11 against three p10); served on the owner's call (weights by sha prefix, the previous kept). Done when the "p11 run" note in DESIGN §11.2 holds its numbers and the owner has decided on serving it. Docs: DESIGN §11.2.
- **p11's two questions measured**: on its snapshots, cross-table restraint (`--partner-rate 0.5`; p10: a bleed took 1.36 pool from its prey, 0.14 cross-table; blood lost 0.96 adjacent, 1.19 cross) and Wider View (p10: taken at 27% of offers, 26% of uses under 4 crypt cards; the owner 12%, none); a training signal if Wider View stays high (no nudge: owner). Done when both numbers are in the p11 note. Docs: DESIGN §11.2.
- **The harvest on a schedule** [doing: Z-train]: `fetch_games.py --finished` then `games_report.py` as one daily command, run on a schedule by Z-main's session (owner, 2026-10-14: a pilot session; no launchd or cron job), the report read after each pull and what it shows taken through `intake`. Done when Z-main's schedule has run it once and the report was read. Docs: the training skill ("Harvest").
- **`waste` over a records directory**: it takes a directory (today record paths), and the report gains the `waste` lines of the bot seats in the pulled games. Done when the report prints them for a pull. Docs: the training skill.
- **Promotion as a script**: curated harvested games (the owner's, the flagged, human wins worth learning from) copied to `data/records` after a check of what a stranger's record holds. Done when the script promotes a game and refuses one failing the check. Docs: `data/records/README.md`, the training skill (Promotion).
- **Harvested games as training input**: `--bc` and `--starts` take a list file or a filter (finished, a human seat, not abandoned or short), never the raw `data/server-records`. Done when a run reads a filtered list. Docs: the training skill (Use).
- **The owner's games weighted up in `--bc`**: a per-player weight (today every winning human seat weighs the same). Done when `--bc` takes the weight and a test checks it. Docs: the training skill.
- **An evaluation set of positions where a human beat the bot**: the decision points before human wins, named and never trained on, scored per network (agreement with the human's choice, the outcome replayed). Done when a network's score on the set prints. Docs: DESIGN §5 (evaluation), the training skill.
- **The human win rate as objective A's measure**: per weights and bot profile from the report's totals, with sample sizes. Done when the report prints it and the next run's DESIGN note quotes it. Docs: DESIGN §11.2, roadmap objective A.
- **Bots on any deck**: per-deck strength (`decks.rs`) on a sample of kept TWDA decks the runs never seated, then the next run's deck mix from it. Done when the per-deck table is in a run note and the next run's mix cites it. Docs: DESIGN §11.2.
- **Take the `zvtes-env` crate**, after the engine board's "`zvtes-env`, step 2": `py` and `policy` import it, the observation's id tables owned there. Done when `uv run pytest` and the policy tests pass on it. Docs: the training skill, DESIGN §5.
- **Paths in the observation**, after the engine board's "Paths": a new `obs::Version` or a slot shown unused. Done when the observation encodes a vampire's path. Docs: DESIGN §5.
- **DESIGN §11.2 as the run record**: one entry a run (the command, its numbers, what it settled), the "prepared" notes folded in, the older runs' narratives cut to that (git keeps them); the training skill's "A run" says so. Done when every run of §11.2 reads as one such entry. Docs: DESIGN §11.2.
- **Iteration speed, the next levers**: ~80% of rollout inference is the frozen opponents' (p11: ~2 min an iteration, 38 s rollout, 76 s update); measure batching their forwards or lighter frozen opponents against the owner's 2–5× (~2.3× reached). Done when the measure is in DESIGN §11.2 "Faster iterations" and the better lever is kept. Docs: DESIGN §11.2.
