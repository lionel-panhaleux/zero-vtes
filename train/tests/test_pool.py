"""The engine's Python boundary (DESIGN §5): the pool's batches are well
formed, and a game played through it replays to the same result from its
seed, decks and decision log."""

import json

import numpy as np
import pytest
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
        assert 60 <= ep["time_limit"] <= 90
        assert zvtes_py.replay(ep["seed"], ep["decks"], ep["log"], ep["time_limit"]) == ep["half_vps"]


def test_queued_tables_route_rows_to_their_networks():
    pool = zvtes_py.GamePool(6, ["policy"] * 4, seed=3)
    specs = [
        {"seats": ["net:0", "net:1", "aggro", "net:1", "random"]},
        {"seats": ["net:2", "net:0", "wall", "random"], "decks": [14, 15, 16, 17]},
    ] * 20
    pool.queue(specs)
    rng = np.random.default_rng(3)
    done = []
    while len(done) < 12:
        obs = pool.observe()
        assert set(obs["agent"]) <= {0, 1, 2}
        done += pool.act(random_actions(obs, rng))
    queued = [ep for ep in done if ep["seats"] != ["net:0"] * 4]
    assert queued, "queued tables were played"
    for ep in queued:
        assert ep["seats"] in (specs[0]["seats"], specs[1]["seats"])
        assert len(ep["half_vps"]) == len(ep["seats"])
        if len(ep["seats"]) == 4:
            assert ep["decks"] == [14, 15, 16, 17]


def test_only_tournament_tables():
    for n in (3, 6):
        with pytest.raises(ValueError):
            zvtes_py.GamePool(2, ["policy"] * n)
    pool = zvtes_py.GamePool(2, ["policy"] * 5)
    with pytest.raises(ValueError):
        pool.queue([{"seats": ["net:0", "aggro", "wall"]}])


def test_vocabulary_keys_and_deck_families():
    keys = zvtes_py.vocab_keys()
    assert len(keys) == json.loads(zvtes_py.schema())["vocab"]
    assert len(set(keys)) == len(keys) and keys[0] == "none"
    families = dict(zvtes_py.deck_families())
    assert len(families["precons"]) == 14 and len(families["minis"]) == 14
    assert len(families["twda"]) >= 10
    names = zvtes_py.deck_names()
    assert sum(len(v) for v in families.values()) == len(names)
    pool = zvtes_py.GamePool(4, ["policy"] * 5, decks=families["twda"])
    assert pool.n_games == 4


def test_tables_avoid_decks_sharing_their_crypt():
    families = dict(zvtes_py.deck_families())
    pool = zvtes_py.GamePool(64, ["policy"] * 5, seed=2, decks=families["precons"])
    rng = np.random.default_rng(2)
    done = []
    while len(done) < 32:
        done += pool.act(random_actions(pool.observe(), rng))
    for ep in done:
        assert len(set(ep["decks"])) == len(ep["decks"]), "a precon twice at a table"
