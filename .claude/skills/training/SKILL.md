---
name: training
description: >
  Run the bot-training loop of zero-vtes: prepare a run from the previous
  one and a measured baseline, launch it from a pinned worktree, monitor it,
  evaluate and export it, take new engine versions, and learn from the
  games humans play on engine.krcg.org (record harvest, reports, imitation,
  evaluation sets, weakness mining). Use for any training run, bot
  evaluation, record pull or report, or question about how the bots learn.
---

# Training the bots

The bots improve in runs (p8 …), each from the last, judged against the
served weights, and fed by the games humans play online. Card additions
never wait on training; training takes engine updates at the start of a
run. Read first: DESIGN §5 (the agent) and §11.2 (the bot layer, the
measures, what measurement settled, "Where the bots stand", the runs —
the last one's command in full), `docs/boards/training.md`. This skill is
the procedure; what a flag does and why is §11.2's.

What a run aims at: DESIGN §11.2 "Where the bots stand" and the
roadmap's M:stronger-bot (a gain against humans; bot against bot only
checks it did not get worse).

## A run

1. **Prepare.** Start from the last run's command (§11.2 "The runs");
   change one or two things, each with a reason and a baseline measured as
   step 4 measures. The owner decides the deck mix, nudges and what to try.
   Write a "pN, prepared" entry in §11.2 first (why, baseline numbers,
   settings). New code gets a pytest in `train/tests/` and the checks
   (docs/operations.md "Checks"). Measures compare like with like only from
   a clean build: `waste`, `decks.rs` and `humanset` compile from the working
   tree, so run them in the run's worktree (or another at a commit), never in
   the shared checkout. Settled, not to retry without new information
   (§11.2 "Settled by measurement"): reward shaping, the pairing, faded
   nudges, the weak-deck routing (`--weak-rate`), search at play time.
   Held out of imitation and starts (`--bc-held-games`, parts of file
   names): the five games held since p10 (`2026-10-11T12-31-56`,
   `2026-10-11T15-45-47`, `2026-10-12T21-02-13`, `2026-10-12T21-50-21`,
   `e33fab34fee8ca4b`) and every human win harvested since the served
   weights' run was launched; the list kept in the run's §11.2 entry, not
   only in `runs/` (p17's was lost with its machine). The five were starts
   of the p10–p16 tables, so every network since descends from tables begun
   in them: `humanset` reports them apart, and a gain is read on the newer
   wins, which no ancestor met.
2. **Launch** from a dedicated worktree at the launch commit (`git worktree
   add .claude/worktrees/pN <sha>`, `uv sync` there: a fresh `.venv` builds the
   extension at that commit): `uv run` rebuilds from whatever is on disk,
   and other sessions leave edits in the shared checkout. `nohup uv run
   python -m train.ppo --league true --resume <abs>/runs/pPREV/best.pt
   --opponents <…> --out <abs>/runs/pN … > <abs>/runs/pN.log 2>&1 &`, the
   paths absolute into the main checkout's `runs/` (ignored). A network
   saved before a new input layer (the seats' context) resumes with it at
   zero and fresh optimizer moments, once. A larger network starts from
   the served one by distillation (`--teacher <ckpt> --d … --layers …`),
   measured as a run before any RL from it.
3. **Monitor** by events (a Monitor on the log), not polling: `panics` and
   `truncated` ~0; `kl`, `clipfrac`, `entropy`; `bc_val_acc`; habits and
   nudges counts; the first lines' `bc_stopped`, `starts_left_out`. An
   evaluation every 20 iterations; `best.pt` moves only on a beaten
   re-measure. After a crash: `--resume runs/pN/model.pt` (saved at
   evaluations only). Machine sleep stalls a run, it does not crash it.
4. **Evaluate**, pN and the served weights alike, on the run's engine:
   `uv run python -m train.export runs/pN/best.pt runs/pN/best.zvp`, then
   - `humanset` (`cargo run -p zvtes-policy --profile ci --example humanset
     -- <pN.zvp> <served.zvp> 20 --apart <the five> --map <records>`) on the
     held-out wins of step 1, the five scored apart: the agreement, by window
     and choice kind with the share collapsed under 1% (DESIGN §11.2 "Missed
     opportunities"), and the wins from the positions — the human-facing
     measure;
   - `decks.rs`: pN at one seat against three of the served weights (its
     last line the VP a seat and game wins with two standard errors), and
     per deck;
   - `waste` (`-- - 0 <records or dirs>`, `--seats 5`, `--twda`, `--by-deck`,
     `--cards N`): its impact lines (A&B a round, R its own, its prey's and
     its predator's) at 4 and 5 seats and on TWDA decks, each with two
     standard errors; the other counters are diagnostics;
   - `uv run python -m train.position runs/pN/best.pt --records
     data/records,data/server-records`: the critic by stage.
   Write the "pN run" entry in §11.2 with the numbers.
5. **Serve** (by the measure, owner 2026-10-23; what counts, owner
   2026-11-09). Not degrading, each within two standard errors of the
   served weights: pN's VP a seat against three of them at or above even,
   1.0; `humanset`'s agreement and wins (a rate's error 2·√(p(1−p)/n)); the
   ledger's A&B a round and its prey's R at 4 and 5 seats and on TWDA. A
   gain, beyond two standard errors: on the human-facing measures
   (`humanset`'s wins on the held-out wins no ancestor met; the online A/B
   once it runs, docs/watch.md) — what closes M:stronger-bot; or one at bot
   tables (a deck's VPs in `decks.rs`, the exploitability probe once it is
   step 4's, training board), which serves but does not close it. Z-train
   sends Z-main the numbers; a run failing the not-degrading test goes to
   the owner instead. Before the deploy:
   - the per-deck file, at the commit being deployed (a clean worktree at
     main's tip): `cargo run -p zvtes-policy --profile ci --example decks --
     --file <served.zvp> 100` (about two and a half hours), written as
     `<served>.decks.json` beside the weights, which `deploy/deploy.py` takes
     if present (DESIGN §11.2 "Bots on any deck"); rewritten in place at
     main's tip, and Z-main asked to redeploy, whenever main's kept TWDA
     decks gain one dated within six months;
   - the served export from a copy (`cp runs/pN/best.pt runs/served-pN.pt`,
     then `train.export runs/served-pN.pt runs/served-pN.zvp`: `best.pt`
     moves while a run goes on), named by its sha prefix in the entry; the
     previous weights kept as `runs/served-prev-<date>-<run>.zvp` with their
     `.decks.json`; weights that have no per-deck file measured before
     serving (a rollback too);
   - Z-main redeploys engine.krcg.org with `ZVTES_WEIGHTS` explicit
     (CLAUDE.md "Sessions"); the deploy keeps the checkpoint on frankfurt by
     the weights' hash (docs/operations.md "Releases and deploys"; on a new
     machine `scp frankfurt:/opt/zvtes/weights/<sha>.pt`). The
     served profile (temperature, nudges) is `crates/policy/src/profiles.rs`
     (`NETWORK`), read by the server, `humanset` and `decks.rs --file`.

## Taking a new engine

- Card rounds: no retraining; `remap_vocab` maps the grown vocabulary on
  resume and for frozen opponents. Check that the run starts. A commit that
  gains a kept TWDA deck dated within six months rewrites the per-deck file
  (step 5; a diff of `crates/engine/src/twda.rs` lists the decks gained).
- A record the pinned engine stops reading partway (a rules fix since it
  was played) gives imitation nothing past the stop: read the run's first
  line (`bc_stopped`) before letting it go on. Before the launch, in its
  worktree, `cargo run -p zvtes-policy --profile ci --example waste -- - 0
  <records>` prints `read to its decision N: why` for each. A record from
  protocol 2.4.0 on carries a position digest and stops at the first
  divergence; an older one can drift silently where a rules change reorders
  hidden cards with no decision (7d7d339: a diablerie of an older vampire
  shuffles the library; replayed later, it opens
  `Interrupt::DisciplineSearch`, `draws.rs`, told in `replay_decisions`'
  `before`), so it is cut at the last such change before its stop.
- Observation changes (a new `Choice` kind, `WindowLabel`, `ActionKind` or
  token type) touch `obs`'s fixed id tables: a new `obs::Version` or a slot
  shown unused; the engine side tells this line before committing
  (`add-cards` §3). A `WindowLabel` after `CombatEndOfRound` would take the
  first clan's id; a new `ActionKind` after `Ability` needs its
  `action_slot` arm. Interrupts, purposes, a new marker kind and the api's
  wire ids are not encoded. `git diff -M <pin> <main> --
  crates/engine/src/obs.rs crates/env/src/obs.rs crates/env/src/context.rs
  crates/engine/src/profile.rs crates/engine/src/decision.rs` says whether a
  table the networks read moved; a change to how an encoded state is
  counted moves a value, not a table.
- After an engine update the first evaluation is a new bar: scores compare
  within a run only.

## Learning from human games

- **Harvest** (daily, docs/watch.md): `python3 scripts/harvest.py` in
  Z-main's detached checkout at `main`'s tip (it refuses another commit or
  engine edits), then the day's report read and what it shows taken through
  `intake`. The corpus is always the main checkout's `data/server-records/`
  (a worktree's own ignored `data/` would split it and its cursor). Its two
  steps: `scripts/fetch_games.py --finished` pulls the finished games since
  `data/server-records/.since` from the admin API, one `<game id>.json`
  each; `scripts/games_report.py` writes `reports/<date>.md` and
  `reports/total.md` (games by engine and weights; the humans' win rate and
  VP a seat by weights, small samples marked; decks; notable human wins
  with their replay command; the bots' `waste` lines). The admin token
  stays in `~/.config/zvtes/admin_token`: never printed, logged or
  committed.
- **Promotion**: curated games go to `data/records/` (committed: a test
  fixture and a training input) by an explicit step — the owner's games,
  games the owner flags, human wins worth learning from: `python3
  scripts/promote.py <game id> … --from <main checkout>/data/server-records`
  in the run's worktree (its engine reads each whole). It refuses a game not
  played out with a human, a test client's, one this engine stops, and one
  failing the stranger's check (`--allow-text` once the owner has read the
  feedback texts).
- **Use**: `--bc` and `--starts` over `data/records` and
  `data/server-records`; the trainer reads players' games only and names
  the starts it left out; Z-main's `python3 scripts/harvest.py --list
  <file>` drops the short games too (`--bc data/records,$(cat <file>)`).
  Imitation keeps only the human who won (§11.2 "Training"); the held-out
  games as step 1 says; `--bc-weak-weight`, `--bc-games-weight` (the
  owner's games, `zero-vtes-replay`, a harvested id) weight by deck and
  game; `--start-winner` puts the learner in a human winner's seat;
  `--held-games N` evaluates from the held-out wins (`vs_held`).
- **Weakness mining**: a recurring bad choice becomes a `waste` counter
  first; the fix is looked for in exploration and opponents that punish it
  (training board) before a nudge, which needs the owner (the owner prefers
  emergent fixes).

## Rules

- Commit once checks and egress are clean (CLAUDE.md "Owner rules");
  weights never in a commit.
- Served weights: named by sha prefix, previous ones kept (step 5).
- Findings (a bot's weakness, a harvest surprise) go through `intake`, onto
  the training board.
