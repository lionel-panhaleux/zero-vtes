"""Tiny PPO runs end to end, evaluation included, and change the policy:
self-play, and the league (one learner seat, the exploiter, snapshots and
bots at 4- and 5-seat tables)."""

import numpy as np
import torch

from train.league import EXP, EXPLOITER, League
from train.ppo import Config, Trainer


def test_one_iteration(tmp_path):
    cfg = Config(
        games=8, steps=24, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", out=str(tmp_path),
    )
    trainer = Trainer(cfg)
    before = [p.detach().clone() for p in trainer.model.parameters()]
    trainer.run()
    assert (tmp_path / "model.pt").exists()
    assert any(not torch.equal(a, b) for a, b in zip(before, trainer.model.parameters()))


def test_league_iterations(tmp_path):
    cfg = Config(
        games=8, steps=24, iterations=2, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=2, eval_games=4, device="cpu", out=str(tmp_path),
        league=True, snapshot_every=1,
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
    cfg = Config(pfsp_rate=0.1, exploiter_reset=100)
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
