"""The exported policy plays as the trained one: `zvtes-policy`'s forward
pass (Rust, by hand) gives `Policy.forward`'s logits and values on real
observations, from a `.zvp` file — also when the file's vocabulary is an
older pool's."""

import json

import numpy as np
import torch
import zvtes_py

from train.export import dump
from train.model import Policy
from train.ppo import batch_of, forward


def observations(n, seed):
    pool = zvtes_py.GamePool(4, ["policy", "aggro", "wall", "policy"], seed=seed, rotate=True)
    rng = np.random.default_rng(seed)
    for _ in range(n):
        obs = pool.observe()
        yield obs
        pool.act([int(rng.integers(k)) for k in obs["opt_n"]])


def compare(model, data, obs):
    logits, values = forward(model, batch_of(obs, "cpu"))
    for i in range(len(obs["tok_n"])):
        t, o = int(obs["tok_n"][i]), int(obs["opt_n"][i])
        got, value = zvtes_py.policy_forward(
            data,
            obs["tok_ids"][i, :t].tolist(),
            obs["tok_f"][i, :t].tolist(),
            obs["opt_ids"][i, :o].tolist(),
            obs["opt_f"][i, :o].tolist(),
        )
        np.testing.assert_allclose(got, logits[i, :o].numpy(), atol=2e-4)
        assert abs(value - float(values[i])) < 2e-4


@torch.no_grad()
def test_exported_policy_matches():
    torch.manual_seed(0)
    schema = json.loads(zvtes_py.schema())
    model = Policy(schema, d=32, layers=2).eval()
    # Untrained biases and norms are 0 and 1: randomize them, to see them read.
    for p in model.parameters():
        p.add_(0.1 * torch.randn_like(p))
    data = dump(model.state_dict(), zvtes_py.vocab_keys(), 32, 2)
    for obs in observations(40, seed=3):
        compare(model, data, obs)


@torch.no_grad()
def test_an_older_vocabulary_is_rekeyed():
    torch.manual_seed(1)
    schema = json.loads(zvtes_py.schema())
    model = Policy(schema, d=32, layers=1).eval()
    keys = zvtes_py.vocab_keys()
    # The file's vocabulary: the same keys in another order (ids shifted).
    order = np.random.default_rng(1).permutation(np.arange(1, len(keys)))
    order = np.concatenate([[0], order])
    state = dict(model.state_dict())
    state["vocab.weight"] = state["vocab.weight"][order]
    data = dump(state, [keys[i] for i in order], 32, 1)
    for obs in observations(10, seed=5):
        compare(model, data, obs)
