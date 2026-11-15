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
looked for where humans beat the bot; owner, 2026-11-15, after the bench:
p20, a full-pool run at the bench's learning rate): p20, then the
observation.

- **p20: the learning rate on the full pool** [M:stronger-bot] [doing: Z-train]: (owner, 2026-11-15, after the bench, DESIGN §11.2 "The bench": λ 0.95 at `--lr 1e-4` gained +0.34 / +0.35 VP a seat on p18 frozen on five decks, nothing shown on the human positions) p18's command with one change, `--lr 1e-4` against 3e-5, from served p18 against it. Done when it is measured and served or not by the training skill's "Serve", its entry naming whether the learning rate moved the full pool. Docs: DESIGN §11.2 (its entry).
- **Paths in the observation** [M:stronger-bot]: a vampire's path (`Minion::path`, read live) as a minion-token feature (added to `obs::Version::V4` while no network reads it, else a slot shown unused or a new version: one bump for both, Z-main 2026-11-08), and `Choice::FollowPath` its own choice kind (folded into kind 58 today, a vampire's four paths alike), both taken out of `zvtes_policy::untrained` in `crates/py` and `NetAgent` together. Done when a run trains with them and the server's bot plays it. Docs: DESIGN §5, §11.2.
