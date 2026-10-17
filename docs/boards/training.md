# Training board

Goal: a really great opponent (roadmap objective A), trained in runs from
a pinned worktree (`../zero-vtes-pN`) and fed by the games humans play on
`engine.krcg.org`. Skill: `training`. Docs: DESIGN §5 (the agent), §11.2
"Policy surface and training runs" (every run's command, numbers and
lessons), `data/records/README.md`, the harvest reports
(`data/server-records/reports/`, ignored).

A shrinking list (CLAUDE.md "Harness"): one line an ask, its done-condition
and the docs it changes; done is deleted; order is priority; the only
state is `[doing: <session>]`. Order: the beta's bot first (the run under way
finished, measured, served), then the loop the beta's games feed (harvest, report,
promotion, evaluation sets), then the levers toward a great opponent on
any deck, then speed and layout.

- **p12 to its end** [doing: Z-train]: `runs/p12` (`../zero-vtes-p12` at cb3b4a6) monitored to its end and evaluated (`waste` against p11@4340, `decks.rs` per deck, p12 against three p11, `humanset` against p11's scores, the human win rate from the harvest); served on the owner's call (weights by sha prefix, the previous kept). Done when the "p12" entry in DESIGN §11.2 holds its numbers and the owner has decided on serving it. Docs: DESIGN §11.2.
- **Promotion as a script**: curated harvested games (the owner's, the flagged, human wins worth learning from) copied to `data/records` after a check of what a stranger's record holds. Done when the script promotes a game and refuses one failing the check. Docs: `data/records/README.md`, the training skill (Promotion).
- **The owner's games weighted up in `--bc`**: a per-player weight (today every winning human seat weighs the same). Done when `--bc` takes the weight and a test checks it. Docs: the training skill.
- **Cross-table restraint, a new lever**: p11's pairing (`--partner-rate 0.5`) did not teach it (DESIGN §11.2 p11: cross-table combats stay harder, 1.21 blood against 0.95); the owner's 4-seat games are its target (at 4 seats, cooperate with the cross-table seat), too few so far to read. Ask the owner after p12 ends, 2026-10-18: keep pairing, mine the owner's 4-seat cross-table decisions once more games are in, or drop it. Done when the owner has chosen. Docs: DESIGN §11.2.
- **Bots on any deck**: per-deck strength (`decks.rs`) on a sample of kept TWDA decks the runs never seated, then the next run's deck mix from it. Done when the per-deck table is in a run note and the next run's mix cites it. Docs: DESIGN §11.2.
- **Revealed cards in the observation**: since "Revealed cards told to every viewer" (main 15fcfe3, Z-main 2026-10-16) every seat is told a library or crypt card an effect reveals (Mesu Bedshet, Petra Resonance's top cards, Gathii, Andrew Stuart, Recruitment Exercise, revealing searches), but the legal view and `Engine::history` leave them out: a human knows them, the networks never do. Done when the observation carries a public reveal (a new `obs::Version` or a slot shown unused) and a test shows a network seat sees one. Docs: DESIGN §5.
- **Take the `zvtes-env` crate**, after the engine board's "`zvtes-env`, step 2": `py` and `policy` import it, the observation's id tables owned there. Done when `uv run pytest` and the policy tests pass on it. Docs: the training skill, DESIGN §5.
- **Paths in the observation**, after the engine board's "Paths": a new `obs::Version` or a slot shown unused. Done when the observation encodes a vampire's path. Docs: DESIGN §5.
- **DESIGN §11.2 as the run record**: one entry a run (the command, its numbers, what it settled), the "prepared" notes folded in, the older runs' narratives cut to that (git keeps them); the training skill's "A run" says so. Done when every run of §11.2 reads as one such entry. Docs: DESIGN §11.2.
