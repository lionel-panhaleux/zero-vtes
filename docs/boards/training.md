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
looked for where humans beat the bot; owner, 2026-11-14, after p19: the
learning signal first, on a bench, no full-pool run meanwhile): the
turn-level value, the bench, then the observation.

- **A turn-level value, trained offline** [M:stronger-bot] [doing: Z-train]: (owner, 2026-11-14, after p19: the critic explains 0.09 / 0.16 / 0.33 / 0.65 of a seat's result by quarter on self-play, −0.18 / −0.09 / 0.12 / 0.54 on the human records, so early advantages are mostly noise; Suphx's global reward predictor) a network predicting a seat's final result (VPs and the game win, as PPO's reward) at any decision, as the critic does (a share of the decisions sampled, `train.value`), from the omniscient view (`encode_critic`), trained supervised on about a million positions of the served p18 playing itself at the served settings, held-out games for its measure; its turn-to-turn difference becomes a reward only if it passes (reopens "Settled by measurement"'s no-shaping line: p15's Φ was a linear fit of 26 visible features at 0.01 in the first quarter). Done when its explained variance by quarter on held-out self-play and on the human records is written in DESIGN §11.2 beside the critic's, passing (above the critic at every quarter on both) or dropped. Docs: DESIGN §11.2 (the measures; "Settled by measurement"), `train/` (its pytest).
- **The bench: a training signal that learns** [M:stronger-bot]: (owner, 2026-11-14, after p19: ten runs flat against their predecessors, 0.88 to 1.03 VP a seat) 4 seats, five decks: the Gangrel (Anarch), Tremere, Hecata and Nosferatu (Fifth Edition) precons and Ring that Bell v2.0 (TWDA, a rush offered on 65% of its turns under p18), the learner from p18 against p18 frozen; three variants of a few hundred iterations each: as today, Monte-Carlo returns (λ 1) at a higher learning rate, and the turn-level value's reward if it passed; the exploitability probe (+0.10 VP in 100 iterations on one deck) the yardstick. Done when each variant's VP a seat on the bench against p18 (two errors) and `humanset` on the held-out human wins with those decks are in a DESIGN §11.2 entry, naming the setup that gains beyond two errors on both (it goes to the full pool, a run prepared from it) or none (then the limit is the games' volume: an ask of the owner on compute). Docs: DESIGN §11.2 (an entry), the training skill if the bench becomes a step, `train/` (pytests for new flags).
- **Paths in the observation** [M:stronger-bot]: a vampire's path (`Minion::path`, read live) as a minion-token feature (added to `obs::Version::V4` while no network reads it, else a slot shown unused or a new version: one bump for both, Z-main 2026-11-08), and `Choice::FollowPath` its own choice kind (folded into kind 58 today, a vampire's four paths alike), both taken out of `zvtes_policy::untrained` in `crates/py` and `NetAgent` together. Done when a run trains with them and the server's bot plays it. Docs: DESIGN §5, §11.2.
