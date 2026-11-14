"""The turn-level value trained offline (`train.value`; the training
board's "A turn-level value"): the games are counted as they began, the
decisions carry the seat's result, the fit never ends worse than it
began, and the measure puts it beside the critic by stage."""

import json

import numpy as np
import torch
import zvtes_py

from train.model import Critic, Policy
from train.position import play
from train.value import fit, gen, measure, predict, stage_of

CPU = torch.device("cpu")
CFG = {"obs": 3, "decks": "precons", "gw_weight": 5.0}


def nets():
    schema = json.loads(zvtes_py.schema())
    torch.manual_seed(0)
    return Policy(schema, d=32, layers=1).eval(), Critic(schema, d=32, layers=1).eval()


def held_mse(net, rows):
    y = np.array([x["target"] for x in rows])
    return float(((predict(net, rows, CPU) - y) ** 2).mean())


def test_games_are_counted_as_they_began():
    # Two at a time: the games a refill begins past the third are played,
    # never yielded — counted as they end, a long game still running
    # would give its place to a shorter refill.
    policy, critic = nets()
    ended = [g for g, _, _ in play(policy, critic, CFG, 3, 4, 1, CPU, parallel=2, keep=0.1)]
    assert sorted(ended) == [0, 1, 2]
    assert ended != sorted(ended), "this seed ends a later game first"


def test_gen_measure():
    policy, critic = nets()
    games = list(gen(policy, critic, CFG, 3, 1, 1.0, CPU, parallel=2))
    rows = [x for g in games for x in g]
    for x in rows:
        # (VPs + 5 × win) / 5, at most 5 VPs and the win; the stage a share.
        assert 0 <= x["target"] <= 2 and 0 <= stage_of(x) <= 1.01
    held = games[-1]
    # The checkpoint critic's value kept with each decision is the critic's.
    assert np.allclose([x["critic"] for x in held], predict(critic, held, CPU), atol=1e-4)
    report = measure(critic, critic, CFG, held, [], CPU)
    assert report["self-play (held out)"], "a stage with enough decisions"
    for r in report["self-play (held out)"]:
        assert {"stage", "n", "critic", "value"} <= set(r)


def test_the_fit_never_ends_worse_than_it_began():
    # Trained toward 2 where the held games end at 0, every epoch is worse
    # on them: the starting weights come back.
    policy, critic = nets()
    rows = [x for g in gen(policy, critic, CFG, 2, 1, 0.3, CPU, parallel=2) for x in g]
    train = [x | {"target": 2.0} for x in rows]
    held = [x | {"target": 0.0} for x in rows]
    start, state = held_mse(critic, held), {k: v.clone() for k, v in critic.state_dict().items()}
    net = fit(critic, [lambda: train], held, CPU, epochs=2, lr=1e-2, log=lambda s: None)
    assert held_mse(net, held) == start
    assert all(torch.equal(v, state[k]) for k, v in net.state_dict().items())
