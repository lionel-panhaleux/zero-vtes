"""Tiny PPO runs end to end, evaluation included, and change the policy:
self-play, and the league (one learner seat, the exploiter, snapshots and
bots at 4- and 5-seat tables)."""

import json

import numpy as np
import pytest
import torch

from train.league import EXP, EXPLOITER, FROZEN, League
from train.model import remap_vocab
from train.ppo import Config, Trainer, eval_tables, imitation, load_replays


def test_one_iteration(tmp_path):
    cfg = Config(
        games=8, steps=24, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", out=str(tmp_path), critic_warmup=0,
    )
    trainer = Trainer(cfg)
    before = [p.detach().clone() for p in trainer.model.parameters()]
    trainer.run()
    assert (tmp_path / "model.pt").exists() and (tmp_path / "best.pt").exists()
    assert trainer.learner.opt.param_groups[0]["lr"] == cfg.lr, "one iteration: no taper yet"
    saved = torch.load(tmp_path / "model.pt")
    assert "critic" in saved and "bots_mean" in open(tmp_path / "metrics.jsonl").read()
    assert any(not torch.equal(a, b) for a, b in zip(before, trainer.model.parameters()))


def test_league_iterations(tmp_path):
    cfg = Config(
        games=8, steps=24, iterations=2, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=2, eval_games=4, device="cpu", out=str(tmp_path),
        league=True, snapshot_every=1, exploiter=True,
    )
    trainer = Trainer(cfg)
    before = [p.detach().clone() for p in trainer.exploiter.model.parameters()]
    trainer.run()
    assert trainer.league.learner_snapshots() == [1, 2]
    assert (tmp_path / "snapshots" / "1.pt").exists()
    assert any(
        not torch.equal(a, b) for a, b in zip(before, trainer.exploiter.model.parameters())
    ), "the exploiter trains"
    names = trainer.league.candidates()
    assert "aggro" in names and "net:2" in names
    for _ in range(20):
        seats = trainer.league.spec()["seats"]
        assert len(seats) in (4, 5)
        assert seats.count("net:0") == 1 and seats.count(EXP) == 1
    resumed = Trainer(Config(**{**vars(cfg), "iterations": 3, "resume": str(tmp_path / "model.pt")}))
    assert resumed.start == 3 and resumed.league.p == trainer.league.p


def test_beat_rates_move_once_per_iteration(tmp_path):
    cfg = Config(pfsp_rate=0.1, exploiter=True, exploiter_reset=100)
    league = League(cfg, np.random.default_rng(0), tmp_path)
    ep = {"truncated": False, "seats": ["net:0", "random", EXP, "aggro"]}
    for _ in range(30):
        league.record(ep, [3, 0, 2, 0])
    assert not league.step()
    assert league.p == {"random": 0.55, EXP: 0.55, "aggro": 0.55}
    league.p["random"] = 0.9
    assert "random" not in league.candidates(), "retired over retire_random"
    league.p[EXP] = 0.2
    assert league.step(), "an exploiter that beats the learner is retired"
    league.retire_exploiter(7, torch.nn.Linear(1, 1))
    assert f"net:{EXPLOITER + 7}" in league.candidates()
    assert league.p[f"net:{EXPLOITER + 7}"] == 0.2 and EXP not in league.p


def test_tournament_decks_train(tmp_path):
    cfg = Config(
        games=8, steps=16, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", out=str(tmp_path), decks="precons,twda",
    )
    trainer = Trainer(cfg)
    assert len(trainer.decks) >= 14 + 10
    trainer.run()
    try:
        Trainer(Config(**{**vars(cfg), "decks": "minis"}))
        raise AssertionError("minis are not legal decks")
    except ValueError:
        pass


def test_embeddings_follow_their_keys():
    types = torch.tensor([[0.0], [5.0]])
    old = {"vocab.weight": torch.tensor([[0.0], [1.0], [2.0]]), "token_type.weight": types, "other": torch.ones(1)}
    fresh = {
        "vocab.weight": torch.full((4, 1), -1.0),
        "token_type.weight": torch.full((3, 1), -1.0),
        "other": torch.zeros(1),
    }
    out = remap_vocab(old, ["none", "crypt:1", "library:9"], ["none", "crypt:1", "crypt:5", "library:9"], fresh)
    assert out["vocab.weight"].squeeze(1).tolist() == [0.0, 1.0, -1.0, 2.0]
    assert out["token_type.weight"].squeeze(1).tolist() == [0.0, 5.0, 0.0], "a new token type starts at zero"
    assert torch.equal(out["other"], old["other"])
    same = remap_vocab(old, ["none"], ["none"], fresh)
    assert len(same["token_type.weight"]) == 3 and torch.equal(same["vocab.weight"], old["vocab.weight"])


def test_evaluation_seats_every_opponent_order():
    tables = eval_tables(["aggro", "wall", "aggro"], 12)
    across = set()
    for seats in tables:
        me = seats.index("net:0")
        across.add(seats[(me + 2) % 4])
    assert across == {"aggro", "wall"}, "the wall is not always across the table"
    assert sorted(t.index("net:0") for t in tables) == [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3]


def test_entropy_coefficient_tracks_its_target(tmp_path):
    cfg = Config(
        games=8, steps=16, iterations=2, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=10, eval_games=4, device="cpu", out=str(tmp_path),
        entropy_target=10.0, entropy_step=2.0, critic_warmup=0,
    )
    trainer = Trainer(cfg)
    trainer.run()
    # Entropy is always under 10: the coefficient doubled each iteration.
    assert abs(trainer.entropy_coef - cfg.entropy_coef * 4) < 1e-9


def test_the_critic_reads_every_hand_and_trains(tmp_path):
    cfg = Config(
        games=8, steps=24, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=10, eval_games=4, device="cpu", out=str(tmp_path),
    )
    trainer = Trainer(cfg)
    obs = trainer.pool.observe()
    assert (obs["critic_tok_n"] >= obs["tok_n"]).all()
    assert (obs["critic_tok_n"] > obs["tok_n"]).any(), "other hands"
    # It starts as the policy's encoder and value head.
    policy = trainer.model.state_dict()
    for k, v in trainer.critic.state_dict().items():
        assert torch.equal(v, policy[k]), k
    before = [p.detach().clone() for p in trainer.critic.parameters()]
    policy_before = [p.detach().clone() for p in trainer.model.parameters()]
    trainer.run()
    assert any(not torch.equal(a, b) for a, b in zip(before, trainer.critic.parameters()))
    # Warming up: the policy is not updated.
    assert all(torch.equal(a, b) for a, b in zip(policy_before, trainer.model.parameters()))


def test_opponent_seats_are_mostly_bots(tmp_path):
    cfg = Config(bot_seats=0.6)
    league = League(cfg, np.random.default_rng(0), tmp_path)
    league.snapshots = {5: None, 6: None}
    draws = [league.draw() for _ in range(2000)]
    share = sum(d in ("aggro", "wall", "random") for d in draws) / len(draws)
    assert 0.55 < share < 0.65


def test_frozen_opponents(tmp_path):
    """`--opponents`: every other seat a frozen checkpoint, reading the
    encoding it was trained on; `best.pt` is selected against them; the
    bots evaluated only from each multiple of `bots_every`, and last."""
    small = dict(
        games=8, steps=16, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", best_seeds=1,
    )
    Trainer(Config(**small, obs=1, out=str(tmp_path / "old"))).run()
    cfg = Config(
        **{**small, "iterations": 3}, out=str(tmp_path / "new"), league=True, bots_every=2,
        opponents=str(tmp_path / "old" / "model.pt"), resume=str(tmp_path / "old" / "model.pt"),
    )
    trainer = Trainer(cfg)
    assert trainer.legacy == [FROZEN] and trainer.warmup == cfg.critic_warmup, "a new input"
    assert trainer.league.candidates() == [f"net:{FROZEN}"]
    for _ in range(20):
        seats = trainer.league.spec()["seats"]
        assert len(seats) in (4, 5) and seats.count("net:0") == 1
        assert set(seats) == {"net:0", f"net:{FROZEN}"}
    trainer.run()
    rows = [json.loads(line) for line in open(tmp_path / "new" / "metrics.jsonl").read().splitlines()]
    assert [r["iteration"] for r in rows if "vs_bots4" in r] == [2, 3]
    row = rows[-1]
    assert "vs_frozen4" in row and "vs_frozen5" in row and "frozen_mean" in row
    assert "vs_random" not in row and not trainer.league.learner_snapshots()
    assert set(row["pfsp"]) <= {"old@1"}
    assert trainer.league.frozen_labels == ["old@1"]


def test_a_v3_learner_from_a_v2_checkpoint(tmp_path):
    """`--obs 3` resumed from a `V2` run, that run frozen: the same weights
    (the deck pooled needs none), the critic warmed up again (a new input),
    the frozen network still reading `V2`; two learner seats a table."""
    small = dict(
        games=8, steps=16, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", best_seeds=1, critic_warmup=1,
    )
    Trainer(Config(**small, obs=2, out=str(tmp_path / "old"))).run()
    old = str(tmp_path / "old" / "model.pt")
    cfg = Config(
        **{**small, "iterations": 3}, obs=3, out=str(tmp_path / "new"), league=True,
        opponents=old, resume=old, learner_seats=2,
    )
    trainer = Trainer(cfg)
    assert trainer.net_obs == [(FROZEN, 2)] and trainer.warmup == 1
    assert all(trainer.league.spec()["seats"].count("net:0") == 2 for _ in range(20))
    trainer.run()
    rows = [json.loads(line) for line in open(tmp_path / "new" / "metrics.jsonl").read().splitlines()]
    assert rows[-1]["iteration"] == 3 and "vs_frozen4" in rows[-1]


def test_habit_seats_and_starts(tmp_path):
    """`--habit-seats`: frozen opponents with the punishing habits, beat
    rates apart, evaluated as `vs_habits4/5`; `--starts`/`--start-tables`:
    tables begun from the owner's games; the learner's habits logged."""
    small = dict(
        games=8, steps=16, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", best_seeds=1,
    )
    Trainer(Config(**small, out=str(tmp_path / "old"))).run()
    cfg = Config(
        **small, out=str(tmp_path / "new"), league=True,
        opponents=str(tmp_path / "old" / "model.pt"), habit_seats=0.5,
        starts="data/records/zero-vtes-replay-2026-10-04T12-09-51-567Z.json", start_tables=0.5,
    )
    trainer = Trainer(cfg)
    specs = [trainer.spec() for _ in range(60)]
    seats = {s for spec in specs for s in spec["seats"]}
    assert seats == {"net:0", f"net:{FROZEN}", f"net:{FROZEN}+habits"}
    assert any("start" in spec for spec in specs) and any("start" not in spec for spec in specs)
    assert all(len(spec["seats"]) == 4 for spec in specs if "start" in spec)
    trainer.run()
    row = json.loads(open(tmp_path / "new" / "metrics.jsonl").read().splitlines()[-1])
    assert "vs_habits4" in row and "vs_habits5" in row
    assert set(row["habits"]) == {"rescue", "burn", "diablerie", "keep", "reaction"}
    assert "starts" in row
    assert set(row["pfsp"]) <= {"old@1", "old@1+habits"}


def test_habit_seats_need_frozen_opponents(tmp_path):
    with pytest.raises(ValueError):
        Trainer(Config(games=8, d=32, layers=1, device="cpu", out=str(tmp_path), habit_seats=0.5))


def test_nudges_fade(tmp_path):
    from train.ppo import nudges

    cfg = Config(rescue_bonus=2.0, pass_bonus=4.0, nudge_from=100, nudge_until=200)
    assert [nudges(cfg, it)["rescue"] for it in (50, 100, 150, 200, 300)] == [2.0, 2.0, 1.0, 0.0, 0.0]
    assert nudges(cfg, 150) == {"rescue": 1.0, "pass": 2.0, "reaction": 0.0, "bleed": 0.0, "idle": 0.0, "draw": 0.0, "stuck": 0.0, "nightmare": 0.0}
    assert nudges(Config(reaction_bonus=1.5), 10_000)["reaction"] == 1.5
    small = dict(
        games=8, steps=16, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", best_seeds=1, critic_warmup=0,
    )
    Trainer(Config(**small, out=str(tmp_path), rescue_bonus=1.5, pass_bonus=3.0)).run()
    row = json.loads(open(tmp_path / "metrics.jsonl").read().splitlines()[-1])
    assert row["nudges"] == {"rescue": 1.5, "pass": 3.0, "reaction": 0.0, "bleed": 0.0, "idle": 0.0, "draw": 0.0, "stuck": 0.0, "nightmare": 0.0} and "habits" in row


def test_imitation_holds_out_named_games():
    """p10 (the plan): the games named held out, the rest trained on; a
    name matching no game refused."""
    a = "data/records/zero-vtes-replay-2026-10-08T21-17-05-603Z.json"
    b = "data/records/zero-vtes-replay-2026-10-09T21-51-54-561Z.json"
    replays = load_replays(",".join([a, b]))
    train, held, names = imitation(replays, 3, 8, ["2026-10-09T21-51"])
    assert names == ["zero-vtes-replay-2026-10-09T21-51-54-561Z.json"]
    assert train and held
    alone, _, _ = imitation(replays[:1], 3, 8)
    assert len(train) == len(alone)
    with pytest.raises(ValueError, match="no replay"):
        imitation(replays, 3, 8, ["1999-01-01"])


def test_imitation_and_weak_decks(tmp_path):
    """p9 (owner, 2026-10-10): the cross-entropy on the owner's choices
    trained and logged (held-out games apart), a game exported twice read
    once, the learner given a weak deck (in a start, seated at one)."""
    small = dict(
        games=8, steps=16, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", best_seeds=1,
    )
    Trainer(Config(**small, out=str(tmp_path / "old"))).run()
    # Malkavian, Ravnos, Path of Caine (the owner), Toreador.
    start = "data/records/zero-vtes-replay-2026-10-08T21-17-05-603Z.json"
    # The game recorded twice: early (a start of its decisions), then whole.
    game = json.loads(open(start).read())
    early = {**game, "entries": game["entries"][:100], "result": None}
    twice = [tmp_path / "a.json", tmp_path / "b.json"]
    twice[0].write_text(json.dumps(early))
    twice[1].write_text(json.dumps(game))
    assert [p.name for p, _, _ in load_replays(",".join(map(str, twice)))] == ["b.json"]
    cfg = Config(
        **small, out=str(tmp_path / "new"), league=True, opponents=str(tmp_path / "old" / "model.pt"),
        bc=",".join([start, "data/records/zero-vtes-replay-2026-10-09T21-51-54-561Z.json"]),
        bc_coef=0.1, bc_holdout=2, bc_batch=32,
        weak_decks="32,5", weak_rate=0.5, starts=start, start_tables=0.3, bots_every=0, critic_warmup=0,
    )
    trainer = Trainer(cfg)
    assert trainer.bc and trainer.bc_held
    specs = [trainer.spec() for _ in range(80)]
    for spec in specs:
        if "start" in spec:
            assert spec["seats"].index("net:0") == 2
        elif "decks" in spec:
            assert spec["decks"][spec["seats"].index("net:0")] in (32, 5)
            assert sum(d is not None for d in spec["decks"]) == 1
    assert any("decks" in s for s in specs) and any("decks" not in s and "start" not in s for s in specs)
    trainer.run()
    row = json.loads(open(tmp_path / "new" / "metrics.jsonl").read().splitlines()[-1])
    assert {"bc_nll", "bc_val_nll", "bc_val_acc", "bc_fit_nll"} <= set(row)
    assert "vs_bots4" not in row, "bots_every 0: no bots once frozen opponents select"
