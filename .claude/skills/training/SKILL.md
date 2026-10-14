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

The bots improve in runs (p8 … p11), each from the last, measured against a
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
   first: why, baseline numbers, settings. New code gets a pytest in
   `train/tests/` and the checks (`uv run pytest -q`, py clippy, fmt,
   clippy). Measures compare like with like only from a clean build: `waste`
   and `decks.rs` compile from the working tree, so run them in the run's
   worktree (or another at a commit), never in the shared checkout. `waste`
   takes record paths (`data/records/*.json`), needs `--twda` for Wider View
   (in no precon) and `--seats 5` for 5-seat cross-table numbers.
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
   pN against three pPREV. A "pN run" note in DESIGN §11.2 with the numbers.
5. **Serve** (the owner's call): redeploy engine.krcg.org with
   `ZVTES_WEIGHTS` explicit; name the served weights by their sha prefix in
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

- **Harvest** (scheduled): `scripts/fetch_games.py --finished` pulls the
  finished games since its cursor (`data/server-records/.since`) from the
  admin API into `data/server-records/` (ignored), one `<game id>.json` each,
  then `scripts/games_report.py` writes the report
  (`data/server-records/reports/<date>.md`): games by engine and weights;
  human win rate and VP per seat by bot profile and weights (small samples
  marked); decks humans win with; notable human wins with their replay
  command; abandoned and very short games counted apart. The admin token
  stays in `~/.config/zvtes/admin_token`: never printed, logged or
  committed.
- **Promotion**: curated games go to `data/records/` (committed: a test
  fixture and a training input) by an explicit step: the owner's games,
  games the owner flags, human wins worth learning from. Check what a
  stranger's record holds before committing it.
- **Use**: train on `data/records` (curated) plus an explicit list or a
  filtered copy of harvested games, never the raw `data/server-records`
  directory (`--bc`/`--starts` read every `*.json` there: bot-only,
  abandoned and short games as starts, every stranger's seats as imitation
  targets). Imitation (`--bc`): held-out games named with
  `--bc-held-games` (it matches parts of file names: game ids for harvested
  files, timestamps in `data/records`), never every Nth. Starts from
  recorded positions (`--starts`). Weakness mining: a recurring bad choice
  becomes a `waste` counter first, a nudge only if it persists and the owner
  agrees (the owner prefers emergent fixes).
- **To build** (training board): weighting the owner's games up in `--bc`
  (today every human seat weighs the same, no per-player filter); an
  evaluation set of "positions where a human beat the bot", never trained
  on.

## Rules

- Commit only on the owner's word; weights never in a commit.
- Served weights: named by sha prefix, previous ones kept (see Serve).
- Log findings in `docs/findings.md` (bots: `E`-rows) and the training board.
