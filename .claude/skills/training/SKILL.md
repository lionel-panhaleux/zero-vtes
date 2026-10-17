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

The bots improve in runs (p8 …), each from the last, measured against a
baseline, and fed by the games humans play online. Card additions never
wait on training; training takes engine updates at the start of a run.
Read first: DESIGN §5 (the agent) and §11.2 "Policy surface and training
runs" (every run's command, numbers and lessons), `docs/boards/training.md`.

## A run

1. **Prepare.** Start from the previous run's command (its DESIGN note
   quotes every flag); change one or two things, each with a reason and a
   baseline measured with `crates/policy/examples/waste.rs` (waste,
   cross-table, Wider View; the owner's records are the reference group)
   and `examples/decks.rs` (per-deck strength). The owner decides the deck
   mix, nudges and what to try. Write a "pN, prepared" note in DESIGN §11.2
   first: why, baseline numbers, settings. §11.2 is the run record, a
   standing page: once the run ends its note becomes one entry (the
   command, the numbers, what it settled), the preparation folded in. New code gets a pytest in
   `train/tests/` and the checks (`uv run pytest -q`, py clippy, fmt,
   clippy). Measures compare like with like only from a clean build: `waste`
   and `decks.rs` compile from the working tree, so run them in the run's
   worktree (or another at a commit), never in the shared checkout. `waste`
   takes record paths or directories (`data/records`, a harvest; `- 0`
   reads records only, no network), groups seats as humans or bots by
   profile and weights, needs `--twda` for Wider View on TWDA decks (four
   precons have it too) and `--seats 5` for 5-seat cross-table numbers.
2. **Launch** from a dedicated worktree at the launch commit
   (`git worktree add ../zero-vtes-pN <sha>`, `uv sync` there: a fresh
   `.venv` builds the extension at that commit; `--reinstall-package
   zvtes-py` is for a long-lived checkout whose engine changed): `uv run`
   rebuilds from whatever is on disk, and other sessions leave edits in the
   shared checkout. `nohup uv
   run python -m train.ppo --league true --resume <abs>/runs/pPREV/best.pt
   --opponents <prev bests> --out <abs>/runs/pN … > <abs>/runs/pN.log 2>&1
   &` with absolute paths into the main checkout's `runs/` (ignored).
3. **Monitor** by events (a Monitor on the log), not polling: `panics` and
   `truncated` ~0; `kl`, `clipfrac`, `entropy`; `bc_val_acc`; habits and
   nudges counts. Evaluation every 20 iterations (VPs + 5×wins at 4 and 5
   seats against the frozen opponents; `best.pt` moves only on a beaten
   re-measure). After a crash: `--resume runs/pN/model.pt` restarts at the
   last evaluation + 1 (`model.pt` is saved at evaluations only: up to 19
   iterations lost). Machine sleep stalls a run, it doesn't crash it.
4. **Evaluate**: `uv run python -m train.export runs/pN/best.pt
   runs/pN/best.zvp`; `waste` against the baseline; `decks.rs` per deck and
   pN against three pPREV; `humanset` (`cargo run -p zvtes-policy --profile
   ci --example humanset -- <pN.zvp> <served bots.zvp> 20 <records>`) on the
   human wins no network trained on — today the five `--bc-held-games`;
   hold out new harvested wins the same way to grow the set — for pN and
   pPREV alike. A "pN run" note in DESIGN §11.2 with the numbers.
5. **Serve** (the owner's call): Z-main redeploys engine.krcg.org with
   `ZVTES_WEIGHTS` explicit (CLAUDE.md "Sessions": only Z-main deploys); name the served weights by their sha prefix in
   the DESIGN note (the server records the weights' SHA-256 per game); keep
   the previous weights as `runs/served-prev-<date>-<run>.zvp`; bot nudges
   live in `crates/server/src/games.rs`.

## Taking a new engine

- Card rounds: no retraining; `remap_vocab` maps the grown card vocabulary
  on resume and for frozen opponents. Check that the run starts and that
  the `bc`/`starts` counts read every record (a record that stops replaying
  shrinks the imitation set silently).
- Observation changes (a new `Choice` kind, window label, interrupt,
  purpose or token type) touch `obs`'s fixed id tables: a new
  `obs::Version` or a slot shown unused; the engine side tells this line
  before merging (`add-cards` §3).
- After an engine update the first evaluation is a new bar: scores compare
  within a run only.

## Learning from human games

- **Harvest** (daily, on Z-main's schedule): `python3 scripts/harvest.py`
  in the shared checkout on `main`, clean (it refuses another branch or
  engine edits: the report replays the new records before protocol 1.7.0
  on its checkout and caches the turns for good, a newer one carries its
  own; `--no-turns` skips both), then read the
  day's report and take what it shows through `intake`. The corpus is
  always the main checkout's `data/server-records/` (each worktree has its
  own ignored `data/`: a pull there would split the corpus and its cursor).
  Its two steps: `scripts/fetch_games.py --finished` pulls the
  finished games since its cursor (`data/server-records/.since`) from the
  admin API into `data/server-records/` (ignored), one `<game id>.json` each,
  then `scripts/games_report.py` writes the report
  (`data/server-records/reports/<date>.md`): games by engine and weights;
  human win rate and VP per seat by bot profile and weights (small samples
  marked); decks humans win with; notable human wins with their replay
  command; abandoned and very short games counted apart; the bots' `waste`
  lines in the counted games, by profile and weights. The admin token
  stays in `~/.config/zvtes/admin_token`: never printed, logged or
  committed.
- **Promotion**: curated games go to `data/records/` (committed: a test
  fixture and a training input) by an explicit step: the owner's games,
  games the owner flags, human wins worth learning from. Check what a
  stranger's record holds before committing it.
- **Use**: train on `data/records` (curated) plus an explicit list or a
  filtered copy of harvested games, never the raw `data/server-records`
  directory (`--bc`/`--starts` read every `*.json` there: bot-only,
  abandoned and short games as starts, every stranger who won as an
  imitation target). The list: the harvested games the report
  counts (`games_report.py`'s docstring has the rule), written by
  `python3 scripts/games_report.py --list <file>` in the shared checkout
  on `main`, after the harvest (its defaults: from a worktree the list is
  empty and records before 1.7.0 would cache that worktree's engine's
  turns; not with `--no-turns`, which counts those games short), as
  absolute paths,
  comma-separated: `--bc data/records,$(cat <file>)`, the same for
  `--starts`. Imitation (`--bc`) keeps only the human who won (owner,
  2026-10-15: the bot is strong and not every human is good; only a game
  played out, last standing or the time limit: a lost, unfinished or
  abandoned one gives nothing, its leader recorded as winner or not), among the options they were shown (a
  record's `shown`: those their settings hid get no probability, so
  Opportunities off is fine); held-out games named with
  `--bc-held-games` (it matches parts of file names: game ids for harvested
  files, timestamps in `data/records`), never every Nth. Starts from
  recorded positions (`--starts`). Weakness mining: a recurring bad choice
  becomes a `waste` counter first, a nudge only if it persists and the owner
  agrees (the owner prefers emergent fixes).

## Rules

- Commit only on the owner's word; weights never in a commit.
- Served weights: named by sha prefix, previous ones kept (see Serve).
- Findings (a bot's weakness, a harvest surprise) go through `intake`,
  onto the training board.
