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
   and `crates/policy/examples/decks.rs` (per-deck strength). The owner
   decides the deck mix, nudges and what to try. Write a "pN, prepared" note in DESIGN §11.2
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
   precons have it too) and `--seats 5` for 5-seat cross-table numbers;
   `--by-deck` splits each group by the seat's deck, for the combat play
   (rushes and their targets, blocks, every combat's cards and results).
   The reward's shaping (`--shaping runs/phi-<run>.json`, from p15): the
   position's potential, fitted by `train.position --phi` on the
   previous best and the run's `--gw-weight`; a new fit or coefficient
   re-warms the critic (compared by value, kept in the checkpoint).
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
   runs/pN/best.zvp`; `waste` for pN and the served weights; `decks.rs`
   per deck and pN against three of the served weights (and three pPREV
   when pPREV was not served); `humanset` (`cargo run -p zvtes-policy --profile
   ci --example humanset -- <pN.zvp> <served bots.zvp> 20 <records>`) on the
   human wins no network trained on — the run's `--bc-held-games`;
   hold out new harvested wins the same way to grow the set — for pN and
   the served weights alike; `uv run python -m train.position runs/pN/best.pt
   --records data/records,data/server-records` for the critic against
   the position by stage (DESIGN §11.2 "The critic against the
   position"). A "pN run" note in DESIGN §11.2 with the numbers.
5. **Serve** (by the measure, owner 2026-10-23): a run is served when
   step 4 shows it does not degrade the served weights and gains
   somewhere. Not degrading, each within two standard errors: pN's VP a
   seat against three of the served weights (`decks.rs`'s second
   measure, 4 seats) at or above even, 1.0 (2·s/√n over its seats);
   `humanset`'s agreement and wins from the positions, and `waste`'s
   rates no worse than the served weights' (a rate's error
   2·√(p(1−p)/n) over its n decisions): bleeds for nothing taken, wakes
   with no block after, unlocks with no action after, a playable card
   cycled while an unplayable one stayed, influence ended with a
   transfer or draw possible and none made, Wider View used,
   cross-table combats to torpor or burned (lower is better); rescues
   taken, adjacent combats to torpor or burned (higher is better). A
   gain: overall, or a niche one (a deck's VPs in `decks.rs`, a
   `humanset` score), shown by a step-4 number against the served
   weights. Z-train sends Z-main the numbers, which
   DESIGN §11.2's "pN run" note records; a run that fails the
   not-degrading test goes to the owner instead. Z-main redeploys
   engine.krcg.org with
   `ZVTES_WEIGHTS` explicit (CLAUDE.md "Sessions": only Z-main deploys); name the served weights by their sha prefix in
   the DESIGN note (the server records the weights' SHA-256 per game); keep
   the previous weights as `runs/served-prev-<date>-<run>.zvp`; the served
   profile (temperature, nudges) lives in `crates/policy/src/profiles.rs`
   (`NETWORK`), which the server and `crates/policy/examples/humanset.rs`
   read.

## Taking a new engine

- Card rounds: no retraining; `remap_vocab` maps the grown card vocabulary
  on resume and for frozen opponents. Check that the run starts.
- A record the pinned engine stops reading partway (a rules fix since it
  was played) gives imitation nothing, held out or not: the run's first
  line names it (`bc_stopped`), and a held-out one shrinks the evaluation
  set, so read that line before letting the run go on. Before the launch,
  in its worktree, `cargo run -p zvtes-policy --profile ci --example waste
  -- - 0 <records>` prints `read to its decision N: why` for each. N is
  only an upper bound: a rules change that reorders hidden cards with no
  decision (7d7d339: a diablerie of an older vampire shuffles the library)
  drifts the replay silently from that point, its hands no longer the
  human's, and the stop comes later, or never. A record from protocol
  2.4.0 on carries a position digest in each decision entry and stops at
  the first divergence. An older record, for `--bc` or as a start, is cut
  at the last such change before its stop. Scan the `--bc` records from
  before 7d7d339 for one before the winner's choices: replayed on the
  pinned engine, the diablerie of an older vampire opens
  `Interrupt::DisciplineSearch` (`draws.rs`), told in `replay_decisions`'
  `before` as the diablerie succeeding.
- Observation changes (a new `Choice` kind, `WindowLabel`, `ActionKind` or
  token type) touch `obs`'s fixed id tables: a new `obs::Version` or a
  slot shown unused; the engine side tells this line before merging
  (`add-cards` §3).
  A `WindowLabel` after the last one (`N_WINDOWS` ends at
  `CombatEndOfRound`) would take the first clan's id; a new `ActionKind`
  after `Ability` needs its `action_slot` arm (folded into `Ability`'s
  slot, as Become Anarch's), else its one-hot lands in `F_LONG`. Interrupts,
  purposes, a new marker kind and the api's wire ids are not encoded (the
  hostage, nightmare, torture and disease counts are, summed in
  `F_MARKERS`): `git diff -M <pin> <main> -- crates/engine/src/obs.rs
  crates/env/src/obs.rs crates/engine/src/decision.rs` says whether a
  table the networks read moved (both obs paths, the file having moved to
  `crates/env`, so a pin before the move diffs as a rename); a change to how an encoded state is counted (`state.rs`, a
  marker's count, `F_CONTESTED`'s inputs) moves a value, not a table.
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
  games the owner flags, human wins worth learning from.
  `python3 scripts/promote.py <game id> … --from <main
  checkout>/data/server-records` in the run's worktree (its engine reads
  each whole) copies them and lists them in the README, refusing a game
  not played out with a human, a test client's, one this engine stops,
  and one failing the stranger's check (a field it does not know, a seat
  name or client the server or Succubus Club did not write, a feedback
  text: `--allow-text` once the owner has read them). A promoted game is
  in `data/records` and in the harvest list both; the trainer reads it
  once (`load_replays` keeps a game listed twice once).
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
  `--starts`. Imitation's cost: `--bc-every 4` puts its term on every 4th
  minibatch at 4× the weight (~14% of an iteration, DESIGN §11.2 "Faster
  iterations"; from p13). Imitation (`--bc`) keeps only the human who won (owner,
  2026-10-15: the bot is strong and not every human is good; only a game
  played out, last standing or the time limit: a lost, unfinished or
  abandoned one gives nothing, its leader recorded as winner or not), among the options they were shown (a
  record's `shown`, read from the entry each decision took,
  `replay_decisions`' `entry`, never by position: a pass the reader reads
  past shifts it; those their settings hid get no probability, so
  Opportunities off is fine); held-out games named with
  `--bc-held-games` (it matches parts of file names: game ids for harvested
  files, timestamps in `data/records`), never every Nth; `--bc-weak-weight
  W` draws the choices of a winner on a `--weak-decks` deck W times as
  often (the first line logs them, `bc_weak`); `--bc-games-weight
  part=W,…` those of named games (parts of file names; the first part a
  name holds) W times, the deck's and the game's weights multiplied
  (`bc_games`): the owner's games, `zero-vtes-replay` (the 24
  converted fork games, all the owner's) and a harvested one by its id —
  no record names its player. Starts from
  recorded positions (`--starts`). Weakness mining: a recurring bad choice
  becomes a `waste` counter first, a nudge only if it persists and the owner
  agrees (the owner prefers emergent fixes).

## Rules

- Commit once checks and egress are clean (CLAUDE.md "Owner rules");
  weights never in a commit.
- Served weights: named by sha prefix, previous ones kept (see Serve).
- Findings (a bot's weakness, a harvest surprise) go through `intake`,
  onto the training board.
