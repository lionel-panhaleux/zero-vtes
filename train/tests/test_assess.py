"""A replay read back as samples (`zvtes_py.replay_decisions`, the
diagnostic's input): one per logged decision plus the one the log ends
on, each with its options, its choice and encodings that fit them."""

import json

import numpy as np
import zvtes_py

from train.assess import evaluate
from train.model import Critic, Policy


def spec(seed):
    seats = [
        {
            "name": f"Seat {s}",
            "deck": {str(i): n for i, n in zvtes_py.deck_cards(d)},
            "player": {"kind": "human"} if s == 1 else {"kind": "bot", "style": "network"},
        }
        for s, d in enumerate([0, 1, 2, 3])
    ]
    return {"seed": seed, "seats": seats}


def test_replay_decisions():
    rng = np.random.default_rng(7)
    log = []
    for _ in range(40):
        game = zvtes_py.replay_decisions(json.dumps({"spec": spec(11), "log": log}), 1)
        decisions = game["decisions"]
        assert [d["decision"] for d in decisions] == list(range(len(log) + 1))
        assert game["humans"] == [1]
        for d, chosen in zip(decisions, log):
            assert d["chosen"] == chosen < len(d["options"]) == len(d["cards"])
            assert d["opt_ids"].shape[0] == len(d["options"])
            assert d["tok_ids"].shape[0] == d["tok_f"].shape[0] > 0
            assert d["critic_tok_ids"].shape[0] >= d["tok_ids"].shape[0]
        pending = decisions[-1]
        assert pending["chosen"] is None
        log.append(int(rng.integers(len(pending["options"]))))
    schema = json.loads(zvtes_py.schema())
    policy, critic = Policy(schema, d=32, layers=1).eval(), Critic(schema, d=32, layers=1).eval()
    probs, values = evaluate(policy, critic, decisions[:-1])
    assert [len(p) for p in probs] == [len(d["options"]) for d in decisions[:-1]]
    assert all(abs(p.sum() - 1) < 1e-4 for p in probs) and len(values) == len(probs)


def test_log_refused():
    bad = {"spec": spec(11), "log": [999]}
    game = zvtes_py.replay_decisions(json.dumps(bad), 1)
    assert "refused" in game["stopped"], "an impossible option index stops the game"
    assert [d["chosen"] for d in game["decisions"]] == [None] and "over" not in game
