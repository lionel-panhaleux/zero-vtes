"""PPO (DESIGN §5, P3–P4) on 4- and 5-seat tables of a `zvtes_py.GamePool`:
self-play (the learner, `net:0`, at every seat), or with `--league` the
tables a `train.league.League` sets — one learner seat, an exploiter seat
(a second trained network), the rest bots and frozen snapshots, or only
frozen previous bests (`--opponents`). Each
trained seat's decisions form a chain, rewarded at the game's end by
(VPs + `gw_weight` × the game win) / 5. Rollouts are truncated: a chain's
last decision in a rollout bootstraps the ones before it and is carried
into the next rollout. Evaluation plays the learner (sampling) against
random seats and the scripted bots, or the `--opponents`.

    uv run python -m train.ppo --iterations 200 --out runs/first
"""

import argparse
import copy
import dataclasses
import itertools
import json
import time
from pathlib import Path

import numpy as np
import torch
import zvtes_py

from train.league import EXPLOITER, HABITS, TABLES, League, label, net_id
from train.model import Critic, Policy, remap_vocab


@dataclasses.dataclass
class Config:
    games: int = 1024
    steps: int = 64
    iterations: int = 100
    epochs: int = 3
    minibatch: int = 1024
    # A minibatch is run in chunks of at most this many rows × tokens²
    # (gradients accumulated: the same step). The attention maps of 1024
    # rows of ~200 tokens (observation V2) filled the MPS allocator's cache
    # past the machine's memory (2026-10-03).
    attention_budget: int = 16_000_000
    lr: float = 3e-4
    # The learning rate tapers linearly over the run (from its start
    # iteration, a resumed one too) to `lr_final` × `lr`.
    lr_final: float = 0.1
    gamma: float = 1.0
    lam: float = 0.95
    clip: float = 0.2
    value_coef: float = 0.5
    entropy_coef: float = 0.02
    # Adaptive entropy bonus: after each update the coefficient is scaled by
    # `entropy_step` toward keeping the policy's entropy at `entropy_target`
    # (0: fixed coefficient). Every run so far peaked against the bots near
    # 0.64 and declined once the entropy fell under ~0.6 (DESIGN §11.2).
    entropy_target: float = 0.65
    entropy_step: float = 1.1
    # The omniscient critic (DESIGN §5): the learner's values come from a
    # separate network reading every hand (`encode_critic`), trained on the
    # same returns; it starts as a copy of the policy's encoder and value
    # head. Training only: the policy plays from the legal view.
    critic: bool = True
    # A new critic's values are poor (a new input): its first
    # `critic_warmup` iterations train it alone, the policy frozen.
    critic_warmup: int = 10
    # The game win's weight in the reward, in VPs (tournament ranking puts
    # game wins first). p9 runs at 5 (owner, 2026-10-10: "penalize doing
    # only 1vp and not winning": ousting a prey then dying earned half a
    # 3-VP win's reward at 2; win-only would leave ~70% of the games
    # without a gradient).
    gw_weight: float = 2.0
    d: int = 96
    layers: int = 2
    # The observation the learner reads (`zvtes::obs::Version`): 2 adds
    # the ash heaps' contents and the last public choices; 3 reads the
    # same in ~40% fewer tokens (the deck pooled out of attention, others'
    # face-down vampires as counts) and the transfers left (owner,
    # 2026-10-06: lighter training). Frozen networks read the one they
    # were trained on (`obs` in their config, else 1).
    obs: int = 3
    # Network seats play the default politics (zvtes::politics, DESIGN
    # §11.2): referendum terms by rule, wrong-side votes masked. Temporary
    # guidance, to be relaxed later in training (owner, 2026-10-04).
    politics: bool = True
    # Network seats never decline a chore (`zvtes_policy::chores`; owner,
    # 2026-10-08: options "a human is allowed to forget … but it's never
    # good to do so"): the Edge's pool, a hunting ground's blood.
    chores: bool = True
    seed: int = 0
    # Evaluations, every `eval_every` iterations (owner, 2026-10-05:
    # iterations faster; evaluation took ~40% of p7's time). Against the
    # bots, once frozen opponents select `best.pt`, only at the first
    # evaluation from each multiple of `bots_every` (their scores sat at
    # 2.4–2.8 VP for hundreds of iterations).
    eval_every: int = 20
    # 0: never, once frozen opponents select (owner, 2026-10-10: "scripted
    # bots are too bad to matter").
    bots_every: int = 50
    eval_games: int = 256
    budget: int = 20_000
    device: str = "auto"
    out: str = "runs/ppo"
    resume: str = ""
    # The league (train/league.py).
    # Deck families drawn per seat, comma-separated: "precons" (the 14 V5
    # precons), "anniversary" (the reviewed anniversary decks: Reign of
    # Stanislava, The Endless Dance), "sabbat" (the Sabbat V5 precons, drawn by
    # path), "twda" (tournament winners fully in the pool). The New Blood
    # minis are half decks, not legal: never trained on. A seat draws a clan,
    # then a deck of it (Reign of Stanislava is a Gangrel deck).
    decks: str = "precons,anniversary"
    league: bool = False
    snapshot_every: int = 10
    opponents_kept: int = 4
    # The learner's seats a league table (owner, 2026-10-07: lighter
    # training): 2 trains on twice the decisions for the same inference
    # (~80% of the rollout's is the opponents' with 1).
    learner_seats: int = 1
    # The share of league tables with a second learner seat across the
    # table from the first (owner, 2026-10-14: "pairing the trainee with a
    # copy of the trainee network cross-table"): restraint between
    # cross-table seats, which mostly helps their neighbours until one is
    # ousted, pays only once both sides learn it — frozen opponents never
    # do. Kept to a share: a cross-table seat that holds back is not given.
    partner_rate: float = 0.0
    # The share of opponent seats drawn among the bots (the rest among the
    # snapshots): the evaluation is against the bots (owner, 2026-09-29).
    bot_seats: float = 0.6
    # The bots drawn for those seats, comma-separated among aggro, wall,
    # random (the evaluation always plays aggro and wall).
    bots: str = "aggro,wall,random"
    # At most this many bot seats a table (once a snapshot exists).
    table_bots: int = 5
    # Checkpoint selection (`best.pt`): see `run`.
    best_seeds: int = 3
    # 0.05 (was 0.1): on p7's last 44 evaluations, 24 re-measures instead
    # of 39, none of its 15 new bests missed (0 would have missed 2).
    best_margin: float = 0.05
    pfsp_rate: float = 0.1
    retire_random: float = 0.85
    # Off: with the learner's reward and weights it never led the learner
    # (DESIGN §11.2, second league run).
    exploiter: bool = False
    exploiter_beaten: float = 0.3
    exploiter_reset: int = 50
    # Frozen opponents (checkpoint paths, comma-separated): with `league`,
    # every other seat is one of them, and `best.pt` is selected on games
    # against them (train/league.py).
    opponents: str = ""
    # The share of frozen opponent seats playing with the punishing habits
    # (`net:k+habits`, `zvtes::habits`; owner, 2026-10-04: opponents that
    # punish what the replays show the network never does); evaluated as
    # `vs_habits4/5`, not used for `best.pt`.
    habit_seats: float = 0.0
    # Which habits those seats play (`zvtes::habits::Set`): a forced rescue
    # made the network stronger, a forced burn, diablerie or minion kept
    # home weaker (DESIGN §11.2).
    habits: str = "rescue,burn,diablerie,home"
    # The nudges (`zvtes_policy::Nudges`; owner, 2026-10-05: "encouraged
    # more as a guiding harness … until it learns by itself"): added to
    # every network seat's logits, in the distribution sampled and trained
    # alike. `rescue_bonus` on a rescue of one's own vampire; `pass_bonus`
    # on passing one's discard phase with a card to cycle (the hand kept);
    # `reaction_bonus` taken off cycling a reaction card; `bleed_bonus`
    # taken off a bleed for nothing (a minion bleeding for 0 before any
    # action modifier: Aggressive Corpse, `zvtes_policy::empty_bleed`;
    # owner, 2026-10-05: "an action with no effect should not be
    # favored"); `idle_bonus` taken off ending the influence phase while a
    # crypt draw is still offered with 5 pool or more
    # (`zvtes_policy::idle_influence`). All full until
    # iteration `nudge_from`, then fading linearly to 0 at `nudge_until`
    # (0: never fades). Evaluation plays with the iteration's nudges.
    rescue_bonus: float = 0.0
    pass_bonus: float = 0.0
    reaction_bonus: float = 0.0
    bleed_bonus: float = 0.0
    idle_bonus: float = 0.0
    # Added to a crypt draw with 5 pool or more (owner, 2026-10-10: "the
    # bot never pays 4 transfers to draw from the crypt").
    draw_bonus: float = 0.0
    # Taken off a transfer onto a second copy of a unique vampire one has
    # in play (`zvtes_policy::stuck_transfer`: never offered out while
    # the first stays) and off a nightmare counter onto one's own minion
    # (`zvtes_policy::own_nightmare`; owner, 2026-10-10).
    stuck_bonus: float = 0.0
    nightmare_bonus: float = 0.0
    nudge_from: int = 0
    nudge_until: int = 0
    # Starts (owner, 2026-10-04): the share of training tables begun from a
    # decision of one of the games in `starts` (a directory of records or
    # comma-separated files; `zvtes_protocol::start`), the learner at a
    # random seat, the libraries and crypts shuffled again.
    starts: str = ""
    start_tables: float = 0.0
    # Imitation (owner, 2026-10-10, the p9 plan; AlphaStar's recipe: RL
    # leads, the human term guides): each PPO minibatch adds `bc_coef` ×
    # the cross-entropy of the policy (its own logits, no nudge) on
    # `bc_batch` of the humans' choices with an alternative, from the
    # records in `bc` (as `starts`). Every `bc_holdout`-th game (0: none)
    # is held out: its loss is logged, never trained on (overfitting).
    # `bc_held_games` (comma-separated parts of file names, the p10 plan:
    # games never trained on, for an honest held-out) holds those out
    # instead.
    bc: str = ""
    bc_coef: float = 0.0
    bc_batch: int = 256
    # The imitation term on every `bc_every`-th minibatch, at `bc_every` ×
    # `bc_coef`: the same expected imitation gradient before clipping (the
    # clip and Adam see the k× step), a quarter of its cost at 4 —
    # human rows are long, a 256-row batch cost a quarter of p12's update
    # (DESIGN §11.2 "Faster iterations", 2026-10-16).
    bc_every: int = 1
    bc_holdout: int = 8
    bc_held_games: str = ""
    # Weak decks (deck indices, comma-separated; owner, 2026-10-10, the p9
    # plan): the learner's seat is given one of them on `weak_rate` of
    # the tables, its opponents' decks drawn as ever; in a start, the
    # learner sits at a seat playing one when there is one.
    weak_decks: str = ""
    weak_rate: float = 0.0
    # The human winners' choices with a weak deck drawn into the imitation
    # minibatches `bc_weak_weight` times as often as the others (owner,
    # 2026-10-21, p14: the rush and wall decks).
    bc_weak_weight: float = 1.0


def device_of(name):
    if name != "auto":
        return torch.device(name)
    return torch.device("mps" if torch.backends.mps.is_available() else "cpu")


FIELDS = ("tok_ids", "tok_f", "opt_ids", "opt_f", "deck_ids", "deck_f")
# A row's items (`rows`): the observation, `V3`'s deck rows (empty
# before), the critic's tokens and deck rows (None without the critic),
# the options' logit bias (the nudges).
TOK, TOK_F, OPT, OPT_F, DECK, DECK_F, C_TOK, C_TOK_F, C_DECK, C_DECK_F, BIAS = range(11)

# A decision's arrays as `zvtes_py.replay_decisions` names them, in a
# row's order (`rows`, the bias aside).
ROW_KEYS = [
    f"{c}{k}"
    for c, ks in (("", FIELDS), ("critic_", ("tok_ids", "tok_f", "deck_ids", "deck_f")))
    for k in ks
]

# Batch shapes are rounded up to these multiples (rows, tokens, options):
# the MPS backend compiles and keeps a graph per shape, never freed — the
# shapes of observation V2's longer, more varied rows grew it by ~1 GB an
# iteration (2026-10-03). Padding is masked: the values are the same.
ROWS, TOKENS, OPTIONS = 64, 16, 8


def bucket(n, k):
    return -(-n // k) * k


def shaped(obs, idx):
    """Rows `idx` of a pool observation, their count padded to a multiple
    of ROWS (repeating the first) and their tokens and options to TOKENS
    and OPTIONS (zeros past the counts)."""
    idx = np.concatenate([idx, np.full(bucket(len(idx), ROWS) - len(idx), idx[0])])
    out = {}
    for k, v in obs.items():
        v = v[idx]
        if v.ndim == 3:
            m = OPTIONS if k.startswith("opt") else TOKENS
            v = np.pad(v, ((0, 0), (0, bucket(v.shape[1], m) - v.shape[1]), (0, 0)))
        out[k] = v
    return out


def rows(obs):
    """Per-game observations, trimmed to their token and option counts:
    tuples of the items `TOK` … `BIAS`."""
    out = []
    critic = "critic_tok_n" in obs
    for i in range(len(obs["tok_n"])):
        t, o, k = obs["tok_n"][i], obs["opt_n"][i], obs["deck_n"][i]
        row = (
            obs["tok_ids"][i, :t],
            obs["tok_f"][i, :t].astype(np.float16),
            obs["opt_ids"][i, :o],
            obs["opt_f"][i, :o].astype(np.float16),
            obs["deck_ids"][i, :k],
            obs["deck_f"][i, :k].astype(np.float16),
        )
        if critic:
            c, ck = obs["critic_tok_n"][i], obs["critic_deck_n"][i]
            row += (
                obs["critic_tok_ids"][i, :c],
                obs["critic_tok_f"][i, :c].astype(np.float16),
                obs["critic_deck_ids"][i, :ck],
                obs["critic_deck_f"][i, :ck].astype(np.float16),
            )
        else:
            row += (None, None, None, None)
        row += (obs["opt_bias"][i, :o],)
        out.append(row)
    return out


def pad_rows(samples, ids, f, device, prefix):
    """Items `ids` and `f` of the rows padded into `{prefix}_ids`,
    `{prefix}_f`, `{prefix}_n` tensors. The features cross to the device
    as the rows keep them (float16: half the bytes of the copy, which
    costs more than the padding) and are widened there."""
    b = len(samples)
    t = bucket(max(s[ids].shape[0] for s in samples), TOKENS)
    out_ids = np.zeros((b, t, samples[0][ids].shape[1]), np.int64)
    out_f = np.zeros((b, t, samples[0][f].shape[1]), samples[0][f].dtype)
    n = np.zeros(b, np.int64)
    for i, s in enumerate(samples):
        n[i] = s[ids].shape[0]
        out_ids[i, : n[i]] = s[ids]
        out_f[i, : n[i]] = s[f]
    return {
        f"{prefix}_ids": torch.from_numpy(out_ids).to(device),
        f"{prefix}_f": torch.from_numpy(out_f).to(device).float(),
        f"{prefix}_n": torch.from_numpy(n).to(device),
    }


def collate_bias(samples, width, device):
    """The rows' option biases, padded to `width`."""
    out = np.zeros((len(samples), width), np.float32)
    for i, s in enumerate(samples):
        out[i, : len(s[BIAS])] = s[BIAS]
    return torch.from_numpy(out).to(device)


def collate_critic(samples, device):
    """The critic's inputs (tokens, then deck rows) padded into a batch."""
    tok = pad_rows(samples, C_TOK, C_TOK_F, device, "tok")
    deck = pad_rows(samples, C_DECK, C_DECK_F, device, "deck")
    return (tok["tok_ids"], tok["tok_f"], tok["tok_n"], deck["deck_ids"], deck["deck_f"], deck["deck_n"])


def collate(samples, device):
    """Pad trimmed observations into a batch."""
    b = len(samples)
    o = bucket(max(s[OPT].shape[0] for s in samples), OPTIONS)
    opt_ids = np.zeros((b, o, samples[0][OPT].shape[1]), np.int64)
    opt_f = np.zeros((b, o, samples[0][OPT_F].shape[1]), np.float32)
    opt_n = np.zeros(b, np.int64)
    for i, s in enumerate(samples):
        opt_n[i] = s[OPT].shape[0]
        opt_ids[i, : opt_n[i]] = s[OPT]
        opt_f[i, : opt_n[i]] = s[OPT_F]
    batch = pad_rows(samples, TOK, TOK_F, device, "tok")
    batch |= pad_rows(samples, DECK, DECK_F, device, "deck")
    batch["opt_ids"] = torch.from_numpy(opt_ids).to(device)
    batch["opt_f"] = torch.from_numpy(opt_f).to(device)
    batch["opt_n"] = torch.from_numpy(opt_n).to(device)
    return batch


def batch_of(obs, device):
    batch = {k: torch.from_numpy(obs[k]).to(device) for k in FIELDS}
    batch["tok_ids"] = batch["tok_ids"].long()
    batch["opt_ids"] = batch["opt_ids"].long()
    batch["deck_ids"] = batch["deck_ids"].long()
    batch["tok_n"] = torch.from_numpy(obs["tok_n"]).long().to(device)
    batch["opt_n"] = torch.from_numpy(obs["opt_n"]).long().to(device)
    batch["deck_n"] = torch.from_numpy(obs["deck_n"]).long().to(device)
    return batch


def forward(model, batch):
    return model(
        batch["tok_ids"],
        batch["tok_f"],
        batch["tok_n"],
        batch["opt_ids"],
        batch["opt_f"],
        batch["opt_n"],
        batch["deck_ids"],
        batch["deck_f"],
        batch["deck_n"],
    )


def scores(ep):
    """VPs and game wins per seat (tournament rules 3.7: at least 2 VPs and
    strictly more than every other seat)."""
    vps = np.array(ep["half_vps"], np.float32) / 2
    best = vps.max()
    return vps, (vps == best) & (best >= 2) & ((vps == best).sum() == 1)


def rewards(ep, gw_weight):
    """Per seat: (VPs + `gw_weight` for the game win) / 5."""
    vps, win = scores(ep)
    return (vps + gw_weight * win) / 5


def selection(e4, e5, gw_weight):
    """An evaluation's score at 4 and 5 seats (`Trainer.evaluate`'s
    results): the reward's VPs + `gw_weight` × game wins, averaged."""
    return sum(e["vps"] + gw_weight * e["gw"] for e in (e4, e5)) / 2


def eval_tables(opponents, games):
    """`games` seatings: the learner (`net:0`) at every seat in turn, then
    the next distinct order of `opponents` around it."""
    orders = sorted(set(itertools.permutations(opponents)))
    n = len(opponents) + 1
    out = []
    for g in range(games):
        seats = list(orders[(g // n) % len(orders)])
        seats.insert(g % n, "net:0")
        out.append(seats)
    return out


@torch.no_grad()
def act(nets, obs, device, rng):
    """Sample every row's option from its network (`obs["agent"]` → `nets`):
    actions, their log-probabilities and the values."""
    b = len(obs["tok_n"])
    actions, logp, values = np.zeros(b, np.int64), np.zeros(b), np.zeros(b, np.float32)
    for agent in np.unique(obs["agent"]):
        idx = np.nonzero(obs["agent"] == agent)[0]
        part = shaped({k: obs[k] for k in (*FIELDS, "tok_n", "opt_n", "deck_n")}, idx)
        logits, v = forward(nets[int(agent)], batch_of(part, device))
        logits, v = logits[: len(idx), : obs["opt_ids"].shape[1]], v[: len(idx)]
        # The nudges: part of the distribution sampled and trained.
        logits = logits.float() + torch.from_numpy(obs["opt_bias"][idx]).to(logits.device)
        probs = torch.softmax(logits.float(), -1).cpu().numpy().astype(np.float64)
        probs /= probs.sum(1, keepdims=True)
        a = np.array([rng.choice(len(p), p=p) for p in probs])
        actions[idx] = a
        logp[idx] = np.log(probs[np.arange(len(a)), a] + 1e-12)
        values[idx] = v.float().cpu().numpy()
    return actions, logp, values


def nudges(cfg, it):
    """The nudges at iteration `it` (`Config.rescue_bonus`, `pass_bonus`,
    `reaction_bonus`, `bleed_bonus`, `idle_bonus`, `draw_bonus`, `stuck_bonus`, `nightmare_bonus`), as `GamePool.nudges` takes them."""
    full = {
        "rescue": cfg.rescue_bonus,
        "pass": cfg.pass_bonus,
        "reaction": cfg.reaction_bonus,
        "bleed": cfg.bleed_bonus,
        "idle": cfg.idle_bonus,
        "draw": cfg.draw_bonus,
        "stuck": cfg.stuck_bonus,
        "nightmare": cfg.nightmare_bonus,
    }
    if cfg.nudge_until <= 0 or it <= cfg.nudge_from:
        return full
    span = max(1, cfg.nudge_until - cfg.nudge_from)
    fade = max(0.0, 1 - (it - cfg.nudge_from) / span)
    return {k: v * fade for k, v in full.items()}


def made(record):
    """A record's decision entries the game went on with: an undo takes
    back those from its `to` on (`zvtes_host`'s reader, `Replay::new`) —
    `load_replays`' log, to tell the same game kept twice."""
    out = []
    for e in record["entries"]:
        if e.get("type") == "undo":
            out = [d for d in out if d["decision"] < e["to"]]
        elif e.get("type") == "decision":
            out.append(e)
    return out


def load_replays(spec):
    """Game records (`--starts`, `--bc`: a directory of `.json` files or
    comma-separated paths; `data/records`): (path, text, seats) each. A
    game kept twice (the same spec, one's decisions the other's start) is
    kept once, its longest record."""
    if not spec:
        return []
    paths = []
    for part in spec.split(","):
        p = Path(part.strip())
        paths += sorted(p.glob("*.json")) if p.is_dir() else [p]
    games = [(p, p.read_text()) for p in paths]
    parsed = [json.loads(t) for _, t in games]
    for g in parsed:
        g["log"] = [e["option"] for e in made(g)]
    out = []
    for i, ((p, text), g) in enumerate(zip(games, parsed)):
        n = len(g["log"])
        longer = any(
            j != i
            and h["spec"] == g["spec"]
            and h["log"][:n] == g["log"]
            and (len(h["log"]) > n or j < i)
            for j, h in enumerate(parsed)
        )
        if not longer:
            out.append((p, text, len(g["spec"]["seats"])))
    return out


def deck_index(seat):
    """A replay seat's deck (KRCG id -> count) as a pool deck index, None
    when it is none of them (a human's own list)."""
    global DECK_INDEX
    if DECK_INDEX is None:
        n = len(zvtes_py.deck_names())
        DECK_INDEX = {frozenset(zvtes_py.deck_cards(d)): d for d in range(n)}
    return DECK_INDEX.get(frozenset((int(k), v) for k, v in seat["deck"].items()))


DECK_INDEX = None


# The ends of a game played out: its winner won at the end.
PLAYED_OUT = {"last_standing", "time_limit"}

# The bias of an option the human was not shown (their settings hid it):
# no probability in the imitation target.
HIDDEN = -1e9


def imitation(replays, obs, holdout, games=()):
    """The choices of the human who won, in `replays` (owner, 2026-10-15:
    "only imitate games where the human actually won at the end" — a
    strong bot, and not every human is good; a game played out,
    `PLAYED_OUT`: an abandoned or cut-short one gives nothing, its leader
    recorded as winner or not), with an
    alternative among the options they were shown (a record's `shown`: the
    others, hidden by their settings, get the bias `HIDDEN`, else 0) — rows
    as the rollout's, and the option chosen: (trained, held out), every
    `holdout`-th game held out — or, with `games` (parts of file names),
    those games (and their names); and the games the engine stops reading
    partway (a rules change since they were played), left out whole: their
    choices up to the stop would train on a game cut short, and a held-out
    one would shrink the evaluation set unseen (2026-10-18: rules-qa 82.1
    stopped six harvested games). Last, each trained choice's deck: the
    winner's pool deck index (`deck_index`), None for a list of their own."""
    for part in games:
        if not any(part in path.name for path, _, _ in replays):
            raise ValueError(f"no replay named like {part!r} to hold out")
    train, held, names, stopped, decks = [], [], [], [], []
    for g, (path, text, _) in enumerate(replays):
        game = zvtes_py.replay_decisions(text, obs)
        if "stopped" in game:
            stopped.append(path.name)
            continue
        if games:
            out = held if any(part in path.name for part in games) else train
        else:
            out = held if holdout and g % holdout == holdout - 1 else train
        if out is held:
            names.append(path.name)
        record = json.loads(text)
        result = record.get("result") or {}
        winner = result.get("winner") if (result.get("reason") or {}).get("type") in PLAYED_OUT else None
        deck = deck_index(record["spec"]["seats"][winner]) if winner is not None else None
        for d in game["decisions"]:
            if d["chosen"] is None or d["seat"] != winner or d["seat"] not in game["humans"]:
                continue
            # The entry the read took, never the n-th: a pass the reader
            # reads past (rules-qa 82.1's short declines, a window no longer
            # asked) leaves its entry unpaired.
            entry = record["entries"][d["entry"]]
            bias = np.zeros(len(d["options"]), np.float32)
            # `shown` indexes the recorded options, which a later engine may
            # move (the record keeps no option count): read only when the
            # chosen one kept its recorded index and is among those shown,
            # and every index is still an option — never a hidden choice;
            # an option inserted after the chosen one may still hide a
            # wrong alternative.
            shown = entry.get("shown") or []
            if (
                shown
                and entry["option"] == d["chosen"]
                and d["chosen"] in shown
                and max(shown) < len(d["options"])
            ):
                bias[:] = HIDDEN
                bias[shown] = 0
            if (bias == 0).sum() > 1:
                out.append((tuple(d[k] for k in ROW_KEYS) + (bias,), d["chosen"]))
                if out is train:
                    decks.append(deck)
    return train, held, names, stopped, decks


def sampling(decks, weak, weight):
    """The imitation minibatches' draw over choices of `decks` (`imitation`'s
    last): a weak deck's `weight` times another's; None, uniform, at 1."""
    if weight == 1:
        return None
    w = np.array([weight if d in weak else 1.0 for d in decks])
    return w / w.sum()


class Learner:
    """A trained network: its model, optimizer and open chains."""

    def __init__(self, agent, model, lr):
        self.agent = agent
        self.model = model
        self.opt = torch.optim.Adam(model.parameters(), lr=lr)
        # (slot, seat) -> transitions [obs, action, logp, value, reward, done]
        self.chains = {}

    def restart(self, weights, lr):
        """New weights and a fresh optimizer; the open chains (sampled by
        the old policy) are dropped."""
        self.model.load_state_dict(weights)
        self.opt = torch.optim.Adam(self.model.parameters(), lr=lr)
        self.chains = {}


class Trainer:
    def __init__(self, cfg):
        self.cfg = cfg
        self.device = device_of(cfg.device)
        self.schema = json.loads(zvtes_py.schema())
        torch.manual_seed(cfg.seed)
        self.rng = np.random.default_rng(cfg.seed)
        model = Policy(self.schema, d=cfg.d, layers=cfg.layers).to(self.device)
        self.learner = Learner(0, model, cfg.lr)
        self.model = model
        self.start = 1
        saved, same_shapes = {}, False
        self.vocab = zvtes_py.vocab_keys()
        if cfg.resume:
            saved = torch.load(cfg.resume, map_location=self.device)
            old = saved.get("vocab", self.vocab)
            model.load_state_dict(remap_vocab(saved["model"], old, self.vocab, model.state_dict()))
            # The optimizer's moments are shaped by the vocabulary and the
            # token types: kept only when neither changed.
            same_shapes = old == self.vocab and all(
                v.shape == saved["model"][k].shape for k, v in model.state_dict().items()
            )
            if "optimizer" in saved and same_shapes:
                self.learner.opt.load_state_dict(saved["optimizer"])
            self.start = saved.get("iteration", 0) + 1
        self.critic = None
        self.warmup = 0
        if cfg.critic:
            self.critic = Critic(self.schema, d=cfg.d, layers=cfg.layers).to(self.device)
            if "critic" in saved:
                old = saved.get("vocab", self.vocab)
                self.critic.load_state_dict(
                    remap_vocab(saved["critic"], old, self.vocab, self.critic.state_dict())
                )
            else:
                # A copy of the policy's encoder and value head.
                self.critic.load_state_dict(model.state_dict(), strict=False)
                self.warmup = cfg.critic_warmup
            self.critic_opt = torch.optim.Adam(self.critic.parameters(), lr=cfg.lr)
            if "critic_optimizer" in saved and same_shapes:
                self.critic_opt.load_state_dict(saved["critic_optimizer"])
            self.warmup = saved.get("critic_warmup", self.warmup)
            # A new observation is a new input for the critic too, a new
            # game-win weight a new scale for its returns.
            old_cfg = saved.get("config", {})
            if saved and (old_cfg.get("obs", 1) != cfg.obs or old_cfg.get("gw_weight", 2.0) != cfg.gw_weight):
                self.warmup = cfg.critic_warmup
        # The best selection evaluation so far (`best.pt`: against the bots
        # or the frozen opponents; kept across a resume into the same
        # directory).
        same = cfg.resume and Path(cfg.resume).parent.resolve() == Path(cfg.out).resolve()
        self.best = saved.get("best", float("-inf")) if same else float("-inf")
        self.entropy_coef = saved.get("entropy_coef", cfg.entropy_coef) if same else cfg.entropy_coef
        self.out = Path(cfg.out)
        self.out.mkdir(parents=True, exist_ok=True)
        self.league = None
        self.exploiter = None
        self.legacy = []
        self.net_obs = []
        if cfg.opponents and not cfg.league:
            raise ValueError("--opponents needs --league true")
        if cfg.bc_every < 1:
            raise ValueError("--bc-every is a positive count of minibatches")
        if cfg.partner_rate > 0 and not cfg.league:
            raise ValueError("--partner-rate needs --league true")
        if cfg.habit_seats > 0 and not cfg.opponents:
            raise ValueError("--habit-seats needs --opponents (frozen seats)")
        if cfg.league:
            self.league = League(cfg, self.rng, self.out)
            if cfg.opponents:
                self.league.load_opponents(
                    lambda c: Policy(self.schema, d=c.get("d", 96), layers=c.get("layers", 2)).to(
                        self.device
                    ),
                    self.vocab,
                    self.device,
                )
                self.legacy = [k for k, v in self.league.frozen_versions.items() if v == 1]
                # Each frozen network reads the observation it was trained on.
                self.net_obs = list(self.league.frozen_versions.items())
            self.league.load(model, self.vocab)
            if not self.league.learner_snapshots() and self.start > 1 and not cfg.opponents:
                self.league.add(self.start - 1, model)
            if "pfsp" in saved:
                self.league.restore(saved["pfsp"])
            if cfg.opponents:  # beat rates of other runs' opponents: noise
                names = set(self.league.candidates())
                self.league.p = {k: v for k, v in self.league.p.items() if k in names}
            if cfg.exploiter:
                self.exploiter = Learner(EXPLOITER, copy.deepcopy(model), cfg.lr)
                if "exploiter" in saved:
                    self.exploiter.model.load_state_dict(saved["exploiter"])
                    self.exploiter.opt.load_state_dict(saved["exploiter_optimizer"])
        families = dict(zvtes_py.deck_families())
        names = [f.strip() for f in cfg.decks.split(",")]
        if not set(names) <= {"precons", "anniversary", "sabbat", "twda"}:
            raise ValueError(
                f"deck families: precons, anniversary, sabbat, twda (got {cfg.decks!r})"
            )
        self.decks = [i for f in names for i in families[f]]
        self.starts = load_replays(cfg.starts)
        # Each start's seats' pool decks (None: a human's own list).
        self.start_decks = [
            [deck_index(s) for s in json.loads(text)["spec"]["seats"]] for _, text, _ in self.starts
        ]
        self.weak = [int(d) for d in cfg.weak_decks.split(",") if d.strip()]
        self.bc, self.bc_held, self.bc_p = [], [], None
        if cfg.bc_coef > 0:
            self.bc, self.bc_held, held, stopped, decks = imitation(
                load_replays(cfg.bc),
                cfg.obs,
                cfg.bc_holdout,
                [g.strip() for g in cfg.bc_held_games.split(",") if g.strip()],
            )
            if not self.bc:
                raise ValueError("--bc-coef needs --bc replays with human choices")
            self.bc_p = sampling(decks, self.weak, cfg.bc_weak_weight)
            print(
                json.dumps(
                    {
                        "bc": len(self.bc),
                        "bc_weak": sum(d in self.weak for d in decks),
                        "bc_held": len(self.bc_held),
                        "held_out": held,
                        "bc_stopped": stopped,
                    }
                ),
                flush=True,
            )
        if cfg.start_tables > 0 and not self.starts:
            raise ValueError("--start-tables needs --starts")
        self.pool = zvtes_py.GamePool(
            cfg.games,
            ["net:0"] * 4,
            seed=cfg.seed,
            decks=self.decks,
            budget=cfg.budget,
            tables=[self.spec() for _ in range(cfg.games)],
            critic=cfg.critic,
            obs=cfg.obs,
            legacy=self.legacy,
            net_obs=self.net_obs,
            politics=cfg.politics,
            chores=cfg.chores,
            replays=[text for _, text, _ in self.starts],
            habits=cfg.habits,
        )
        if self.starts:
            readable = dict(zip((p.name for p, _, _ in self.starts), self.pool.starts))
            print(json.dumps({"starts": readable}), flush=True)

    def learners(self):
        return [self.learner] + ([self.exploiter] if self.exploiter else [])

    def nets(self):
        if not self.league:
            return {0: self.model}
        return self.league.nets(self.model, self.exploiter.model if self.exploiter else None)

    def spec(self):
        start, n, at = None, None, None
        if self.starts and self.rng.random() < self.cfg.start_tables:
            start = int(self.rng.integers(len(self.starts)))
            n = self.starts[start][2]
            weak = [s for s, d in enumerate(self.start_decks[start]) if d in self.weak]
            if weak:
                at = int(self.rng.choice(weak))
        if self.league:
            spec = self.league.spec(n, at)
        else:
            spec = {"seats": ["net:0"] * (n or int(self.rng.choice(TABLES)))}
        if start is not None:
            spec["start"] = start
        elif self.league and self.weak and self.rng.random() < self.cfg.weak_rate:
            decks = [None] * len(spec["seats"])
            decks[spec["seats"].index("net:0")] = int(self.rng.choice(self.weak))
            spec["decks"] = decks
        return spec

    def rollout(self):
        """Play `steps` pool steps; each learner's transitions with their
        advantages and returns, and episode statistics."""
        stats = {"episodes": 0, "truncated": 0, "turns": 0, "starts": 0, "panics": 0}
        by_agent = {lr.agent: lr for lr in self.learners()}
        closed = {a: [] for a in by_agent}
        self.pool.queue([self.spec() for _ in range(self.cfg.games)])
        for _ in range(self.cfg.steps):
            obs = self.pool.observe()
            actions, logp, values = act(self.nets(), obs, self.device, self.rng)
            if self.critic is not None:
                values = self.critic_values(obs, values)
            for i, row in enumerate(rows(obs)):
                learner = by_agent.get(int(obs["agent"][i]))
                if learner is None:
                    continue
                learner.chains.setdefault((i, int(obs["seat"][i])), []).append(
                    [row, int(actions[i]), float(logp[i]), float(values[i]), 0.0, False]
                )
            for ep in self.pool.act(actions.tolist()):
                if "panic" in ep:
                    # An engine bug (named on stderr): the game's chains
                    # are dropped, not rewarded.
                    stats["panics"] += 1
                    for s in range(len(ep["seats"])):
                        for learner in self.learners():
                            learner.chains.pop((ep["slot"], s), None)
                    continue
                stats["episodes"] += 1
                stats["truncated"] += ep["truncated"]
                stats["turns"] += ep["turns"]
                stats["starts"] += "start" in ep
                if self.league:
                    self.league.record(ep, scores(ep)[0])
                r = rewards(ep, self.cfg.gw_weight)
                for s, name in enumerate(ep["seats"]):
                    learner = by_agent.get(net_id(name))
                    chain = learner and learner.chains.pop((ep["slot"], s), None)
                    if chain:
                        chain[-1][4], chain[-1][5] = float(r[s]), True
                        closed[learner.agent].append(chain)
        samples = {}
        for learner in self.learners():
            out = []
            for chain in closed[learner.agent]:
                out += self.advantages(chain, bootstrap=None)
            # Open chains: the last decision bootstraps the others and waits.
            for key, chain in list(learner.chains.items()):
                if len(chain) > 1:
                    out += self.advantages(chain[:-1], bootstrap=chain[-1][3])
                    learner.chains[key] = [chain[-1]]
            samples[learner.agent] = out
        return samples, stats

    @torch.no_grad()
    def critic_values(self, obs, values):
        """The learner's rows valued by the omniscient critic."""
        idx = np.nonzero(obs["agent"] == 0)[0]
        if len(idx):
            keys = [f"critic_{a}_{b}" for a in ("tok", "deck") for b in ("ids", "f", "n")]
            part = shaped({k: obs[k] for k in keys}, idx)
            t = {k: torch.from_numpy(part[k]).to(self.device) for k in keys}
            args = [t[k].long() if not k.endswith("_f") else t[k] for k in keys]
            values = values.copy()
            values[idx] = self.critic(*args).cpu().numpy()[: len(idx)]
        return values

    def advantages(self, chain, bootstrap):
        cfg, out, gae = self.cfg, [], 0.0
        next_value = bootstrap if bootstrap is not None else 0.0
        for row, a, logp, v, r, done in reversed(chain):
            if done:
                next_value, gae = 0.0, 0.0
            delta = r + cfg.gamma * next_value - v
            gae = delta + cfg.gamma * cfg.lam * gae
            out.append((row, a, logp, gae, gae + v))
            next_value = v
        return out

    def minibatches(self, samples):
        """An epoch's minibatches, bucketed by length: a random order cut
        into chunks of 8 minibatches, each sorted by token count (the
        critic's when it reads them: the costlier encoder) and cut in
        minibatches, which are then shuffled — every sample once, little
        padding (tokens vary 20–80 a decision). A last partial minibatch is
        dropped (its shape would be new: `ROWS`), unless it is the only
        one; the epochs' orders differ, so each drops other samples."""
        mb = self.cfg.minibatch
        width = C_TOK if samples[0][0][C_TOK] is not None else TOK
        size = np.array([s[0][width].shape[0] for s in samples])
        order = self.rng.permutation(len(samples))
        batches = []
        for k in range(0, len(order), 8 * mb):
            chunk = order[k : k + 8 * mb]
            chunk = chunk[np.argsort(size[chunk], kind="stable")]
            batches += [chunk[j : j + mb] for j in range(0, len(chunk), mb)]
        if len(batches) > 1:
            batches = [b for b in batches if len(b) == mb]
        return [batches[i] for i in self.rng.permutation(len(batches))]

    def chunks(self, samples, idx):
        """`idx` cut so that each chunk's attention maps (rows × tokens²,
        the longest row's tokens: the critic's when it reads them) stay
        within `attention_budget`, halving: power-of-two sizes of a
        power-of-two minibatch (few shapes: `ROWS`)."""
        width = C_TOK if samples[0][0][C_TOK] is not None else TOK
        t = bucket(max(samples[i][0][width].shape[0] for i in idx), TOKENS)
        size = len(idx)
        while size > 1 and size * t * t > self.cfg.attention_budget:
            size = -(-size // 2)
        return [idx[k : k + size] for k in range(0, len(idx), size)]

    def update(self, learner, samples):
        cfg = self.cfg
        if not samples:
            return {}
        adv = np.array([s[3] for s in samples], np.float32)
        adv = (adv - adv.mean()) / (adv.std() + 1e-8)
        losses, bc = [], []
        critic = self.critic is not None and learner.agent == 0
        warming = critic and self.warmup > 0
        for _ in range(cfg.epochs):
            for idx in self.minibatches(samples):
                learner.opt.zero_grad()
                if critic:
                    self.critic_opt.zero_grad()
                # Gradients accumulated over chunks of the minibatch, each
                # within `attention_budget` (the same step, less memory).
                totals = np.zeros(5)
                for part in self.chunks(samples, idx):
                    w = len(part) / len(idx)
                    rows = [samples[i][0] for i in part]
                    logits, values = forward(learner.model, collate(rows, self.device))
                    logits = logits.float() + collate_bias(rows, logits.shape[1], self.device)
                    if critic:
                        values = self.critic(*collate_critic(rows, self.device))
                    logp_all = torch.log_softmax(logits, -1)
                    a = torch.tensor([samples[i][1] for i in part], device=self.device)
                    logp = logp_all.gather(1, a[:, None]).squeeze(1)
                    old = torch.tensor([samples[i][2] for i in part], device=self.device)
                    ad = torch.tensor(adv[part], device=self.device)
                    ret = torch.tensor([samples[i][4] for i in part], device=self.device)
                    ratio = torch.exp(logp - old)
                    pg = -torch.min(ratio * ad, ratio.clamp(1 - cfg.clip, 1 + cfg.clip) * ad).mean()
                    vf = ((values - ret) ** 2).mean()
                    ent = -(logp_all.exp() * logp_all).sum(-1).mean()
                    loss = pg + cfg.value_coef * vf - self.entropy_coef * ent
                    if warming:
                        loss = cfg.value_coef * vf
                    (w * loss).backward()
                    with torch.no_grad():
                        # How far the update has moved the policy from the
                        # one that played (k3 estimator), and the share of
                        # samples the clip holds.
                        kl = ((ratio - 1) - (logp - old)).mean()
                        clipped = ((ratio - 1).abs() > cfg.clip).float().mean()
                    totals += w * np.array([pg.item(), vf.item(), ent.item(), kl.item(), clipped.item()])
                if self.bc and learner.agent == 0 and not warming and len(losses) % cfg.bc_every == 0:
                    pick = self.rng.choice(
                        len(self.bc), min(cfg.bc_batch, len(self.bc)), replace=False, p=self.bc_p
                    )
                    nll, _ = self.imitation_loss(learner.model, [self.bc[i] for i in pick])
                    (cfg.bc_every * cfg.bc_coef * nll).backward()
                    bc.append(nll.item())
                torch.nn.utils.clip_grad_norm_(learner.model.parameters(), 1.0)
                if not warming:
                    learner.opt.step()
                if critic:
                    torch.nn.utils.clip_grad_norm_(self.critic.parameters(), 1.0)
                    self.critic_opt.step()
                losses.append(totals)
        pg, vf, ent, kl, clipfrac = np.mean(losses, 0)
        # Of the last epoch: what the epochs add up to.
        last = np.mean(losses[-max(1, len(losses) // cfg.epochs) :], 0)
        out = {"pg": pg, "vf": vf, "entropy": ent, "kl": last[3], "clipfrac": last[4]}
        if bc:
            out["bc_nll"] = float(np.mean(bc))
        return out

    def imitation_loss(self, model, pairs):
        """The policy's mean -log p of the humans' choices in `pairs` (row,
        chosen), and how often it is the policy's favourite."""
        rows = [r for r, _ in pairs]
        logits, _ = forward(model, collate(rows, self.device))
        # The options the human was not shown out of the target (`HIDDEN`).
        logits = logits.float() + collate_bias(rows, logits.shape[1], self.device)
        chosen = torch.tensor([c for _, c in pairs], device=self.device)
        logp = torch.log_softmax(logits, -1)
        nll = -logp.gather(1, chosen[:, None]).squeeze(1).mean()
        return nll, (logits.argmax(-1) == chosen).float().mean().item()

    @torch.no_grad()
    def imitation_check(self):
        """The held-out human choices' loss and agreement (and the trained
        ones', on as many), the policy in eval mode."""
        out = {}
        for name, pairs in (("bc_val", self.bc_held), ("bc_fit", self.bc[: len(self.bc_held) or 512])):
            if not pairs:
                continue
            n, nll, acc = 0, 0.0, 0.0
            for k in range(0, len(pairs), 256):
                part = pairs[k : k + 256]
                a, b = self.imitation_loss(self.model, part)
                nll, acc, n = nll + a.item() * len(part), acc + b * len(part), n + len(part)
            out[f"{name}_nll"], out[f"{name}_acc"] = round(nll / n, 4), round(acc / n, 4)
        return out

    def evaluate(self, opponents, games, seed=0):
        """The learner's mean VPs and game-win rate over `games` games
        against `opponents`: every seat for the learner and every distinct
        order of the opponents around it, in turn (a fixed order would, at
        4 seats, always sit the same opponent across the table)."""
        tables = [{"seats": seats} for seats in eval_tables(opponents, games)]
        # Every game at once, none after (`refill`): the network's batches
        # as wide as they can be, no throwaway game in them.
        pool = zvtes_py.GamePool(
            games,
            tables[0]["seats"],
            seed=10_000 + 1_000 * seed + self.cfg.seed,
            decks=self.decks,
            budget=self.cfg.budget,
            tables=tables,
            obs=self.cfg.obs,
            legacy=self.legacy,
            net_obs=self.net_obs,
            politics=self.cfg.politics,
            chores=self.cfg.chores,
            habits=self.cfg.habits,
            refill=False,
        )
        pool.nudges = self.pool.nudges
        rng = np.random.default_rng(seed)
        vps, wins, done, truncated = [], 0, 0, 0
        # The queued tables are games 0..games-1; later ones (the default
        # table, once the queue runs out) are not counted.
        while done < games:
            actions, _, _ = act(self.nets(), pool.observe(), self.device, rng)
            for ep in pool.act(actions.tolist()):
                if ep["game"] >= games:
                    continue
                done += 1
                truncated += ep["truncated"]
                v, w = scores(ep)
                for s, kind in enumerate(ep["seats"]):
                    if kind == "net:0":
                        vps.append(float(v[s]))
                        wins += bool(w[s])
        return {"vps": round(float(np.mean(vps)), 3), "gw": round(wins / games, 3), "truncated": truncated}

    def checkpoint(self, it):
        saved = {
            "model": self.model.state_dict(),
            "optimizer": self.learner.opt.state_dict(),
            "iteration": it,
            "config": dataclasses.asdict(self.cfg),
            "pfsp": self.league.state() if self.league else {},
            "best": self.best,
            "entropy_coef": self.entropy_coef,
            "vocab": self.vocab,
        }
        if self.critic is not None:
            saved["critic"] = self.critic.state_dict()
            saved["critic_optimizer"] = self.critic_opt.state_dict()
            saved["critic_warmup"] = self.warmup
        if self.exploiter:
            saved["exploiter"] = self.exploiter.model.state_dict()
            saved["exploiter_optimizer"] = self.exploiter.opt.state_dict()
        torch.save(saved, self.out / "model.pt")
        return saved

    def release(self):
        """Return the MPS allocator's cached blocks: batches of every shape
        leave some, and they are not reused across shapes."""
        if self.device.type == "mps":
            torch.mps.empty_cache()

    def run(self):
        cfg = self.cfg
        log = open(self.out / "metrics.jsonl", "a")
        for it in range(self.start, cfg.iterations + 1):
            t0 = time.perf_counter()
            self.pool.nudges = nudges(cfg, it)
            done = (it - self.start) / max(1, cfg.iterations - self.start)
            lr = cfg.lr * (1 - (1 - cfg.lr_final) * done)
            opts = [learner.opt for learner in self.learners()]
            if self.critic is not None:
                opts.append(self.critic_opt)
            for opt in opts:
                for group in opt.param_groups:
                    group["lr"] = lr
            for learner in self.learners():
                learner.model.eval()
            if self.critic is not None:
                self.critic.eval()
            samples, stats = self.rollout()
            self.release()
            t1 = time.perf_counter()
            metrics = {}
            for learner in self.learners():
                learner.model.train()
                m = self.update(learner, samples[learner.agent])
                prefix = "" if learner.agent == 0 else "exploiter_"
                metrics.update({prefix + k: v for k, v in m.items()})
                learner.model.eval()
            self.release()
            if self.bc:
                metrics.update(self.imitation_check())
            t2 = time.perf_counter()
            warming = self.warmup > 0
            self.warmup = max(0, self.warmup - 1)
            if cfg.entropy_target > 0 and "entropy" in metrics and not warming:
                up = metrics["entropy"] < cfg.entropy_target
                self.entropy_coef *= cfg.entropy_step if up else 1 / cfg.entropy_step
                self.entropy_coef = min(max(self.entropy_coef, 1e-3), 0.2)
            stats["turns"] = round(stats["turns"] / max(stats["episodes"], 1), 1)
            row = {
                "iteration": it,
                "samples": len(samples[0]),
                "rollout_s": round(t1 - t0, 2),
                "update_s": round(t2 - t1, 2),
                "lr": float(f"{lr:.3g}"),
                **({"critic_warmup": True} if warming else {}),
                "entropy_coef": float(f"{self.entropy_coef:.3g}"),
                **stats,
                **{k: round(float(v), 4) for k, v in metrics.items()},
                # The learner's (offered, taken) per habit this rollout;
                # "keep": (discard phases with a card to cycle, passed),
                # "reaction": (those holding a reaction, cycling one).
                "habits": {k: list(v) for k, v in self.pool.habits().items()},
                **(
                    {"nudges": {k: round(v, 3) for k, v in self.pool.nudges.items()}}
                    if any(self.pool.nudges.values()) or any(nudges(cfg, 0).values())
                    else {}
                ),
            }
            if self.league:
                if self.league.step():
                    self.league.retire_exploiter(it, self.exploiter.model)
                    self.exploiter.restart(self.model.state_dict(), cfg.lr)
                    row["exploiter_retired"] = True
                if it % cfg.snapshot_every == 0 and not self.league.frozen:
                    self.league.add(it, self.model)
            if it % cfg.eval_every == 0 or it == cfg.iterations:
                frozen = self.league.candidates() if self.league and self.league.frozen else []
                if not frozen:
                    row["vs_random"] = self.evaluate(["random"] * 4, cfg.eval_games)
                bots = not frozen or cfg.bots_every > 0 and (
                    it % cfg.bots_every < cfg.eval_every or it == cfg.iterations
                )
                if bots:
                    row["vs_bots4"] = self.evaluate(["aggro", "wall", "aggro"], cfg.eval_games)
                    row["vs_bots5"] = self.evaluate(["aggro", "wall", "aggro", "wall"], cfg.eval_games)
                # The selection's tables: the frozen opponents in turn
                # around the learner, else the bots.
                if frozen:
                    sel = [[frozen[k % len(frozen)] for k in range(n - 1)] for n in TABLES]
                    row["vs_frozen4"] = self.evaluate(sel[0], cfg.eval_games)
                    row["vs_frozen5"] = self.evaluate(sel[1], cfg.eval_games)
                    if cfg.habit_seats > 0:
                        punish = [[k + HABITS for k in t] for t in sel]
                        row["vs_habits4"] = self.evaluate(punish[0], cfg.eval_games)
                        row["vs_habits5"] = self.evaluate(punish[1], cfg.eval_games)
                    row["pfsp"] = {
                        label(k, self.league.frozen_labels): round(v, 2)
                        for k, v in sorted(self.league.p.items())
                    }
                    first = (row["vs_frozen4"], row["vs_frozen5"])
                else:
                    sel = [["aggro", "wall", "aggro"], ["aggro", "wall", "aggro", "wall"]]
                    first = (row["vs_bots4"], row["vs_bots5"])
                if self.league and not frozen and self.league.latest() is not None:
                    snap = f"net:{self.league.latest()}"
                    row["vs_snapshot"] = self.evaluate([snap] * 4, cfg.eval_games)
                    row["pfsp"] = {label(k): round(v, 2) for k, v in sorted(self.league.p.items())}
                # `best.pt`: the reward's measure at 4 and 5 seats (`sel`):
                # VPs + `gw_weight` × game wins, averaged (owner,
                # 2026-10-10: winning first; VPs alone until p9) —
                # a candidate within `best_margin` of the best is measured
                # again on `best_seeds` - 1 more evaluation seeds, and
                # kept on the mean (one evaluation's noise, ±0.15 VP, would
                # otherwise pick lucky checkpoints).
                score = selection(*first, cfg.gw_weight)
                better = False
                if score > self.best - cfg.best_margin:
                    scores = [score]
                    for k in range(1, cfg.best_seeds):
                        b4 = self.evaluate(sel[0], cfg.eval_games, seed=k)
                        b5 = self.evaluate(sel[1], cfg.eval_games, seed=k)
                        scores.append(selection(b4, b5, cfg.gw_weight))
                    mean = float(np.mean(scores))
                    row["frozen_mean" if frozen else "bots_mean"] = round(mean, 3)
                    better = mean > self.best
                    if better:
                        self.best = mean
                        row["best"] = True
                saved = self.checkpoint(it)
                if better:
                    torch.save(saved, self.out / "best.pt")
            self.release()
            print(json.dumps(row), flush=True)
            log.write(json.dumps(row) + "\n")
            log.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for f in dataclasses.fields(Config):
        kind = type(f.default)
        if kind is bool:
            kind = lambda s: s.lower() in ("1", "true", "yes")  # noqa: E731
        parser.add_argument("--" + f.name.replace("_", "-"), type=kind, default=f.default)
    args = parser.parse_args()
    Trainer(Config(**vars(args))).run()


if __name__ == "__main__":
    main()
