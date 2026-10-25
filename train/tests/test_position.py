"""The position a seat reads (`GamePool.positions`, `replay_decisions`'
`position`) and the measure against the critic (`train.position`; the
training board's "The critic against the position, measured")."""

import json

import numpy as np
import torch
import zvtes_py

from train.model import Critic, Policy
from train.position import STAGES, fit, measure, predict, r2, self_play, table, unshaped
from train.tests.test_assess import record, spec

NAMES = zvtes_py.position_names()


def test_opening_position():
    # Every Methuselah begins with 30 pool and no VP (rulebook, setup).
    for seats in (4, 5):
        pool = zvtes_py.GamePool(3, ["policy"] * seats, seed=3)
        pool.observe()
        f = pool.positions()
        assert f.shape == (3, len(NAMES))
        col = dict(zip(NAMES, f.T))
        for k in ("pool", "prey_pool", "predator_pool"):
            assert np.allclose(col[k], 1.0), k
        assert np.allclose(col["seats"], seats / 5)
        assert np.allclose(col["vps"], 0) and np.allclose(col["best_other_vps"], 0)
        assert np.allclose(col["cross_pool"], 1.0)
        assert np.allclose(col["ready"], 0), "no minion in play before the first influence"


def test_positions_follow_the_game():
    pool = zvtes_py.GamePool(4, ["policy"] * 4, seed=5)
    rng = np.random.default_rng(5)
    seen = []
    for _ in range(400):
        obs = pool.observe()
        f = pool.positions()
        assert np.isfinite(f).all() and f.shape[0] == len(obs["tok_n"])
        seen.append(f)
        pool.act([int(rng.integers(n)) for n in obs["opt_n"]])
    f = np.concatenate(seen)
    col = dict(zip(NAMES, f.T))
    assert (col["pool"] < 1).any(), "pool spent on influence"
    assert (col["ready"] > 0).any(), "vampires brought out"


def test_replayed_decisions_carry_the_position():
    game = zvtes_py.replay_decisions(json.dumps(record(spec(11), [0] * 30)), 1)
    for d in game["decisions"]:
        assert len(d["position"]) == len(NAMES)
    assert dict(zip(NAMES, game["decisions"][0]["position"]))["pool"] == 1.0


def test_measure_recovers_a_linear_result():
    rng = np.random.default_rng(1)
    n = 4000
    X = rng.normal(size=(n, len(NAMES)))
    stage = rng.uniform(0, 1, n)
    y = X[:, 0] - 0.5 * X[:, 1] + 0.01 * rng.normal(size=n)
    w = fit(X, y)
    assert r2(y, predict(w, X)) > 0.99
    noise = y + rng.normal(size=n)
    games = np.arange(n)
    t = (X, noise, y, stage, games)
    report = measure(t, {"same": t})
    assert len(report["same"]) == len(STAGES)
    for row in report["same"]:
        assert row["features"] > 0.99 and row["critic"] < row["features"]
        assert row["both"] >= row["features"] - 1e-3


def test_self_play_ends_on_truncated_games():
    # A game cut at its decision budget has no result: dropped, counted,
    # and the measure still ends.
    schema = json.loads(zvtes_py.schema())
    torch.manual_seed(0)
    policy, critic = Policy(schema, d=32, layers=1).eval(), Critic(schema, d=32, layers=1).eval()
    cfg = {"obs": 3, "decks": "precons"}
    out, over, dropped = self_play(policy, critic, cfg, 3, 4, 1, torch.device("cpu"), budget=40)
    assert (out, over, dropped) == ([], {}, 3)
    samples, over, dropped = self_play(policy, critic, cfg, 2, 4, 1, torch.device("cpu"), budget=20_000)
    assert dropped + len(over) == 2 and over, "seeded: a game ends within the budget"
    X, v, y, stage, games = table(samples, over)
    assert X.shape == (len(samples), len(NAMES)) and np.isfinite(v).all()
    assert set(games) == set(over) and ((stage >= 0) & (stage <= 1.01)).all()


def test_a_shaped_critic_is_read_with_its_potential():
    # A critic trained with shaping learns the result less coef × Φ: the
    # measure adds Φ back before comparing it with the result.
    rng = np.random.default_rng(2)
    X = rng.normal(size=(50, len(NAMES)))
    w, b, coef = rng.normal(size=len(NAMES)), 0.3, 0.7
    y = rng.normal(size=50)
    v = y - coef * (X @ w + b)
    t = (X, v, y, np.zeros(50), np.arange(50))
    assert unshaped(t, None) is t
    _, back, *_ = unshaped(t, {"weights": w.tolist(), "intercept": b, "coef": coef})
    assert np.allclose(back, y)
