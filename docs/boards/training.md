# Training board

Goal: a really great opponent (roadmap objective A), trained in runs from
a pinned worktree (`.claude/worktrees/pN`) and fed by the games humans play on
`engine.krcg.org`. Skill: `training`. Docs: DESIGN §5 (the agent), §11.2
"Policy surface and training runs" (the agent's surface, what measurement
settled, the measures, one entry a run), `data/records/README.md`, the
harvest reports (`data/server-records/reports/`, ignored).

A shrinking list (CLAUDE.md "Harness"; the milestone tokens: roadmap
"Milestones"). Order (owner, 2026-11-09: humans win 79% against p14
(and p10) and bot-against-bot is flat since p10, so the next gain is
looked for where humans beat the bot): p19 on the combats the bots enter
(owner, 2026-11-12: most decks the bot is bad at are combat decks), then
the observation.

- **p19: the combats the bots enter** [M:stronger-bot] (owner, 2026-11-12, after the combat comparison, DESIGN §11.2 "Settled by measurement": the humans' rushes put the target down 54% of the time, the bots' 17%; rushed, the bots' minion goes down 44%, the humans' 16%): from p18, both levers in one run: (a) the human winners' combat entries weighted up in imitation, their rush declarations (`zvtes::profile::rush`) and block declarations (`DeclareBlock`) drawn k times as often (a new flag beside `--bc-weak-weight`); (b) a share of the frozen opponent seats pinned to the rush-capable decks (a rush offered on 30% or more of their own turns in `waste --by-deck`, 35 of 216 TWDA decks at p18's measure, plus the rush-capable precons). Done when it is measured and served or not by the training skill's "Serve", its entry reading `waste`'s combat split (by how a combat began) against p18's. Docs: DESIGN §11.2 (its entry), the training skill.
- **Paths in the observation**: a vampire's path (`Minion::path`, read live) as a minion-token feature (added to `obs::Version::V4` while no network reads it, else a slot shown unused or a new version: one bump for both, Z-main 2026-11-08), and `Choice::FollowPath` its own choice kind (folded into kind 58 today, a vampire's four paths alike), both taken out of `zvtes_policy::untrained` in `crates/py` and `NetAgent` together. Done when a run trains with them and the server's bot plays it. Docs: DESIGN §5, §11.2.
