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
    assert len(families["anniversary"]) >= 1
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


def test_each_network_reads_its_encoding():
    """`obs=2` adds the ash heap and event tokens (types 9 and up) to the
    networks' observations, but the `legacy` ones'."""
    pool = zvtes_py.GamePool(
        16, ["net:0", "net:1", "net:0", "net:1"], seed=4, rotate=True, obs=2, legacy=[1]
    )
    rng = np.random.default_rng(4)
    added = {0: 0, 1: 0}
    for _ in range(80):
        obs = pool.observe()
        for i, agent in enumerate(obs["agent"]):
            t = obs["tok_n"][i]
            added[int(agent)] += int((obs["tok_ids"][i, :t, 0] >= 9).sum())
        pool.act([int(rng.integers(k)) for k in obs["opt_n"]])
    assert added[0] > 0 and added[1] == 0


def test_default_politics_answer_the_terms():
    """`politics=True`: referendum terms are answered by rule, never sent to
    Python, and games still replay from their log (the engine's indices)."""
    names = zvtes_py.deck_names()
    political = [i for i, n in enumerate(names) if n.split(" (")[0] in ("Ventrue", "Toreador", "Lasombra")]
    terms = 21  # WindowLabel::ReferendumTerms
    seen = {}
    for politics in (False, True):
        pool = zvtes_py.GamePool(
            8, ["net:0"] * 4, seed=3, decks=political, keep_logs=True, politics=politics
        )
        rng = np.random.default_rng(3)
        rows, done = 0, []
        while len(done) < 8:
            obs = pool.observe()
            rows += int((obs["window"] == terms).sum())
            done += pool.act(random_actions(obs, rng))
        seen[politics] = rows
        for ep in done:
            assert zvtes_py.replay(ep["seed"], ep["decks"], ep["log"], ep["time_limit"]) == ep["half_vps"]
    assert seen[False] > 0 and seen[True] == 0


OWNER_GAME = "data/replays/zero-vtes-replay-2026-10-04T12-09-51-567Z.json"


def test_habit_seats_and_the_learners_habits():
    """`net:k+habits`: a network seat playing with the punishing habits
    (`zvtes::habits`), named apart in episodes; `habits()` counts the
    learner's offered and taken habits, then resets."""
    table = ["net:0", "net:1+habits", "net:1", "net:1+habits"]
    pool = zvtes_py.GamePool(8, table, seed=5)
    rng = np.random.default_rng(5)
    done = []
    while len(done) < 8:
        obs = pool.observe()
        assert set(obs["agent"]) <= {0, 1}
        done += pool.act(random_actions(obs, rng))
    assert all(ep["seats"] == table for ep in done)
    counts = pool.habits()
    assert set(counts) == {"rescue", "burn", "diablerie", "keep", "reaction"}
    assert all(0 <= taken <= offered for offered, taken in counts.values())
    assert sum(offered for offered, _ in counts.values()) > 0
    assert all(v == (0, 0) for v in pool.habits().values())
    with pytest.raises(ValueError):
        pool.queue([{"seats": ["net:0", "net:x+habits", "net:1", "net:1"]}])


def test_a_pool_without_refill_ends_with_its_queue():
    """`refill=False`: every queued table played once, no game after; the
    pool's slots leave as their games end."""
    tables = [{"seats": ["net:0", "aggro", "wall", "aggro"]}] * 6
    pool = zvtes_py.GamePool(6, tables[0]["seats"], seed=3, tables=tables, refill=False)
    rng = np.random.default_rng(3)
    done = []
    while pool.n_games:
        obs = pool.observe()
        assert len(obs["opt_n"]) == pool.n_games
        done += pool.act(random_actions(obs, rng))
    assert sorted(ep["game"] for ep in done) == list(range(6))


def test_each_network_reads_its_own_observation():
    """`obs=3` with `net_obs`: the learner reads `V3` (its deck rows apart),
    a frozen network the version it was trained on (`V2`: deck tokens)."""
    pool = zvtes_py.GamePool(8, ["net:0", "net:1", "net:0", "net:1"], seed=4, obs=3, net_obs=[(1, 2)])
    rng = np.random.default_rng(4)
    seen = {0: 0, 1: 0}
    for _ in range(200):
        obs = pool.observe()
        deck_tokens = (obs["tok_ids"][..., 0] == 7).sum(1)
        for i, agent in enumerate(obs["agent"]):
            if agent == 0:
                assert deck_tokens[i] == 0 and obs["deck_n"][i] > 0
            else:
                assert deck_tokens[i] > 0 and obs["deck_n"][i] == 0
            seen[int(agent)] += 1
        pool.act(random_actions(obs, rng))
    assert all(seen.values())
    with pytest.raises(ValueError):
        zvtes_py.GamePool(1, ["net:0"] * 4, obs=4)


def test_tables_start_from_an_owner_game():
    """A queued `start`: the game replayed to one of its decisions (past the
    opening), every seat played from there; episodes say which."""
    text = open(OWNER_GAME).read()
    pool = zvtes_py.GamePool(4, ["policy"] * 4, seed=9, replays=[text])
    assert pool.starts == [369]
    pool.queue([{"seats": ["aggro", "net:0", "wall", "aggro"], "start": 0}] * 8)
    rng = np.random.default_rng(9)
    done = []
    while len(done) < 8:
        done += pool.act(random_actions(pool.observe(), rng))
    started = [ep for ep in done if "start" in ep]
    assert started and all(ep["start"][0] == 0 and 369 // 5 <= ep["start"][1] < 369 for ep in started)
    assert all(ep["decks"] == [] for ep in started)
    with pytest.raises(ValueError):
        pool.queue([{"seats": ["net:0"] * 5, "start": 0}])
    with pytest.raises(ValueError):
        pool.queue([{"seats": ["net:0"] * 4, "start": 1}])


def test_nudges_bias_their_options_only():
    """`nudges`: `opt_bias` holds the rescue nudge on every network seat's
    rescue of its own vampire, the pass nudge on passing its discard phase,
    minus the reaction nudge on cycling a reaction, minus the bleed nudge
    on a bleed for nothing, minus the idle nudge on ending the influence
    phase with a crypt draw offered; 0 elsewhere (and everywhere at 0)."""
    pool = zvtes_py.GamePool(16, ["policy"] * 4, seed=11)
    assert pool.nudges == {"rescue": 0.0, "pass": 0.0, "reaction": 0.0, "bleed": 0.0, "idle": 0.0}
    rng = np.random.default_rng(11)
    obs = pool.observe()
    assert (obs["opt_bias"] == 0).all() and obs["opt_bias"].shape == obs["opt_f"].shape[:2]
    pool.nudges = {"rescue": 2.0, "pass": 0.5}
    pool.nudges = {"reaction": 0.25, "bleed": 0.125, "idle": 1.0}
    assert pool.nudges == {"rescue": 2.0, "pass": 0.5, "reaction": 0.25, "bleed": 0.125, "idle": 1.0}
    with pytest.raises(ValueError):
        pool.nudges = {"rush": 1.0}
    seen = {2.0: 0, 0.5: 0, -0.25: 0, -0.125: 0, -1.0: 0}
    for _ in range(8000):
        obs = pool.observe()
        assert set(np.unique(obs["opt_bias"])) <= {0.0, *seen}
        for v in seen:
            seen[v] += int((obs["opt_bias"] == v).sum())
        if all(seen.values()):
            break
        pool.act(random_actions(obs, rng))
    assert all(seen.values()), seen
    counts = pool.habits()
    (phases, passed), (held, cycled) = counts["keep"], counts["reaction"]
    assert 0 < phases and passed <= phases and cycled <= held <= phases


def test_a_seat_given_a_deck_keeps_it():
    """`decks` with None entries: the seats given one keep it, the others
    are drawn, never sharing much of a crypt with it."""
    pool = zvtes_py.GamePool(4, ["policy"] * 4, seed=5, decks=list(range(14)))
    pool.queue([{"seats": ["net:0", "random", "random", "random"], "decks": [None, 12, None, None]}] * 8)
    rng = np.random.default_rng(5)
    done = []
    while len(done) < 4:
        # The pool's first games (dealt as it is made) are its default table.
        done += [ep for ep in pool.act(random_actions(pool.observe(), rng)) if ep["seats"][1] == "random"]
    for ep in done:
        assert ep["decks"][1] == 12 and ep["decks"].count(12) == 1
    assert len({ep["decks"][0] for ep in done}) > 1, "the others drawn"
