"""The engine's Python boundary (DESIGN §5): the pool's batches are well
formed, and a game played through it replays to the same result from its
seed, decks and decision log."""

import json

import numpy as np
import zvtes_py


def random_actions(obs, rng):
    return [int(rng.integers(n)) for n in obs["opt_n"]]


def test_batches_are_well_formed():
    schema = json.loads(zvtes_py.schema())
    pool = zvtes_py.GamePool(8, ["policy", "random", "aggro", "wall"], seed=1, rotate=True)
    rng = np.random.default_rng(1)
    for _ in range(300):
        obs = pool.observe()
        b, t, _ = obs["tok_ids"].shape
        assert b == 8 and obs["tok_f"].shape == (b, t, schema["tok_f"])
        assert obs["opt_f"].shape[2] == schema["opt_f"]
        # Single-option windows are elided: a real decision has a choice.
        assert (obs["opt_n"] >= 2).all()
        for i in range(b):
            k = obs["tok_n"][i]
            ptr = obs["opt_ids"][i, : obs["opt_n"][i], 2:]
            assert ((ptr >= 0) & (ptr <= k)).all()
            assert (obs["tok_ids"][i, k:] == 0).all()
            assert obs["opt_ids"][i, : obs["opt_n"][i], 1].max() < schema["vocab"]
        pool.act(random_actions(obs, rng))


def test_games_replay_from_their_log():
    pool = zvtes_py.GamePool(4, ["policy"] * 4, seed=7, keep_logs=True)
    rng = np.random.default_rng(7)
    done = []
    while len(done) < 4:
        done += pool.act(random_actions(pool.observe(), rng))
    for ep in done:
        assert not ep["truncated"]
        assert zvtes_py.replay(ep["seed"], ep["decks"], ep["log"]) == ep["half_vps"]
