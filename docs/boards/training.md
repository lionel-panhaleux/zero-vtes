# Training board

Goal: a really great opponent (roadmap objective A), trained in runs from
a pinned worktree (`../zero-vtes-pN`) and fed by the games humans play on
`engine.krcg.org`. Skill: `training`. Docs: DESIGN §5 (the agent), §11.2
"Policy surface and training runs" (the agent's surface, what measurement
settled, the measures, one entry a run), `data/records/README.md`, the
harvest reports (`data/server-records/reports/`, ignored).

A shrinking list (CLAUDE.md "Harness"; the milestone tokens: roadmap
"Milestones"). Order (owner, 2026-11-09: humans win 79% against p14
(and p10) and bot-against-bot is flat since p10, so the next gain is
looked for where humans beat the bot): the map of what the bot misses,
exploration over it with the probe that shows it, the run, then the
levers toward any deck.

- **Exploration over the collapsed kinds, shown on an exploiter** [M:stronger-bot] [doing: Z-train]: an exploration share (`train/ppo.py`): a small share of the learner's rollout decisions sampled from its distribution flattened over the choice kinds the map names, PPO's ratio taken against the distribution actually sampled; then the exploitability probe with and without it: an exploiter from p17 against p17 frozen at every other seat (no imitation, its learning rate above p17's runs', one deck at a time), its VP a seat after a fixed budget of iterations. Without exploration nothing was found twice (the second league run's exploiter never led the learner, 2026-09-28; p17, from p14 against p14 alone, level after 300 iterations): the probe is a measure only if the share finds what plain PPO does not. Done when both probes' numbers, over two seeds, are in DESIGN §11.2 "The measures", and the probe, if it tells them apart, is the training skill's step 4 (else the owner hears why it does not). Docs: DESIGN §11.2, the training skill.
- **A run that explores what it collapsed, against exploiters** [M:stronger-bot] (owner, 2026-11-09): from p17, the exploration share on and exploiters at the learner's tables (`--exploiter true`, deck-specialized, with the share), p17 frozen beside them; the nudges kept until the exploration shows them redundant; no weak-deck routing (`--weak-rate 0`: it moved the decks round, not up, p10). Done when it is measured and served or not by the training skill's "Serve" (not degrading; a gain on `humanset` over the held-out wins no ancestor met closes M:stronger-bot, a gain on the probe serves without closing it). Docs: DESIGN §11.2 (its entry), the training skill.
- **Bots on any deck, and its weakest recent TWDA decks** [M:stronger-bot]: the served weights' per-deck file (DESIGN §11.2 "Bots on any deck") sets the next run's imitation weight: `--weak-decks` names the file's weakest recent decks the pool holds, `--weak-rate 0` (no routing), and a human's choices on them are drawn `--bc-weak-weight` times as often. Done when the next run's command takes its `--weak-decks` from the file, the entry citing it. Docs: the training skill, DESIGN §11.2.
- **Paths in the observation**: a vampire's path (`Minion::path`, read live) as a minion-token feature (added to `obs::Version::V4` while no network reads it, else a slot shown unused or a new version: one bump for both, Z-main 2026-11-08), and `Choice::FollowPath` its own choice kind (folded into kind 58 today, a vampire's four paths alike), both taken out of `zvtes_policy::untrained` in `crates/py` and `NetAgent` together. Done when a run trains with them and the server's bot plays it. Docs: DESIGN §5, §11.2.
