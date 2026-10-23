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

- **The seats' context as an input, for p16** [doing: Z-train]: the owner with Tom (2026-10-22): "a better view of the opponents' action history: what they did, what they're capable of … as part of a context vector", and the position as a context vector for the policy too (owner, 2026-10-23). Built (DESIGN §5 "The seats' context": `zvtes_env::context`, `zvtes::profile`, the zero-started layer in the policy, the critic and the server's `Net`); left: p16 trained with it and served. Done when the server's bot plays a network trained with it and p16's entry in DESIGN §11.2 measures it. Docs: DESIGN §11.2, the training skill.
- **Rush decks' combat, the next lever**: p14 (DESIGN §11.2 "p14") took the deck mix and the imitation weight toward the rush and wall decks (Tzimisce, Gangrel, Salubri, Nosferatu, Brujah, Banu Haqim, Hecata) and moved only the rushes of the decks that already rushed (Brujah 29→41%, Hecata 42→56%); Nosferatu, Gangrel and Banu Haqim still rush on under one turn in eight (the humans 21–42%) and the opposing minion goes down in 12–36% of the combats (the humans 26–91%), as for p13. Count what "Combat play by deck" left uncounted, the strikes and maneuvers that waste (which a card's strike or maneuver gains nothing is a reading per card, not a mask), for p14 and the humans by deck; then the owner chooses the lever (a rush nudge, a combat-outcome reward, per-card fixes in the bots' layer). Done when the count is in DESIGN §11.2 and the owner has chosen. Docs: DESIGN §11.2, the training skill.
- **Cross-table restraint from the owner's 4-seat games**: the pairing taught it in neither p11 nor p12 and is off from p13 (owner, 2026-10-17); the owner's 4-seat games are its target (at 4 seats, cooperate with the cross-table seat). Once the harvest holds 50 of the owner's cross-table combats (`waste`'s humans group, no seat ousted; 11 in `data/records` on 2026-10-15), compare the owner's cross-table play with the served network's decision by decision and propose the lever it points at. Done when that comparison is in DESIGN §11.2 and the owner has chosen. Docs: DESIGN §11.2.
- **Bots on any deck**: per-deck strength (`decks.rs`) on a sample of kept TWDA decks the runs never seated, then the next run's deck mix from it. Done when the per-deck table is in a run note and the next run's mix cites it. Docs: DESIGN §11.2.
- **Revealed cards in the observation**: since "Revealed cards told to every viewer" (main 15fcfe3, Z-main 2026-10-16) every seat is told a library or crypt card an effect reveals (Mesu Bedshet, Petra Resonance's top cards, Gathii, Andrew Stuart, Recruitment Exercise, revealing searches), but the legal view and `Engine::history` leave them out: a human knows them, the networks never do. Done when the observation carries a public reveal (a new `obs::Version` or a slot shown unused) and a test shows a network seat sees one. Docs: DESIGN §5.
- **Paths in the observation**, after the engine board's "Paths": a new `obs::Version` or a slot shown unused. Done when the observation encodes a vampire's path. Docs: DESIGN §5.
- **DESIGN §11.2 as the run record**: one entry a run (the command, its numbers, what it settled), the "prepared" notes folded in, the older runs' narratives cut to that (git keeps them); the training skill's "A run" says so. Done when every run of §11.2 reads as one such entry. Docs: DESIGN §11.2.
