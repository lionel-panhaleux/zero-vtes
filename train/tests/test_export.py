"""The exported policy plays as the trained one: `zvtes-policy`'s forward
pass (Rust, by hand) gives `Policy.forward`'s logits and values on real
observations, from a `.zvp` file — also when the file's vocabulary is an
older pool's."""

import json

import numpy as np
import torch
import zvtes_py

from train.export import dump
from train.model import Policy, remap_vocab
from train.ppo import batch_of, forward


def observations(n, seed, obs=1):
    pool = zvtes_py.GamePool(4, ["policy", "aggro", "wall", "policy"], seed=seed, rotate=True, obs=obs)
    rng = np.random.default_rng(seed)
    for _ in range(n):
        obs = pool.observe()
        yield obs
        pool.act([int(rng.integers(k)) for k in obs["opt_n"]])


def compare(model, data, obs):
    logits, values = forward(model, batch_of(obs, "cpu"))
    for i in range(len(obs["tok_n"])):
        t, o, k = int(obs["tok_n"][i]), int(obs["opt_n"][i]), int(obs["deck_n"][i])
        got, value = zvtes_py.policy_forward(
            data,
            obs["tok_ids"][i, :t].tolist(),
            obs["tok_f"][i, :t].tolist(),
            obs["opt_ids"][i, :o].tolist(),
            obs["opt_f"][i, :o].tolist(),
            obs["deck_ids"][i, :k].tolist(),
            obs["deck_f"][i, :k].tolist(),
            obs["ctx"][i].tolist(),
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
    # `V3`: the deck rows pooled into the context token, alike.
    seen = 0
    for obs in observations(40, seed=4, obs=3):
        assert not (obs["tok_ids"][..., 0] == 7).any()
        seen += int(obs["deck_n"].sum())
        compare(model, data, obs)
    assert seen


@torch.no_grad()
def test_a_policy_without_the_seats_context():
    # A `.zvp` exported before the seats' context (no `ctx.weight`) plays
    # as a network whose context layer is zero: the server keeps playing
    # p14 and p15 as trained.
    torch.manual_seed(1)
    schema = json.loads(zvtes_py.schema())
    model = Policy(schema, d=32, layers=2).eval()
    for name, p in model.named_parameters():
        if name != "ctx.weight":
            p.add_(0.1 * torch.randn_like(p))
    assert not model.ctx.weight.any(), "the context layer starts at zero"
    old = {k: v for k, v in model.state_dict().items() if k != "ctx.weight"}
    data = dump(old, zvtes_py.vocab_keys(), 32, 2, obs=3)
    seen = 0
    for obs in observations(30, seed=5, obs=3):
        seen += int(np.abs(obs["ctx"]).sum() > 0)
        compare(model, data, obs)
    assert seen, "the observations carry a context"


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


@torch.no_grad()
def test_a_v2_policy_matches():
    """A network reading `V2` (ash heaps, events: a token type per choice
    kind) exports with its version and plays as trained."""
    torch.manual_seed(2)
    schema = json.loads(zvtes_py.schema())
    model = Policy(schema, d=32, layers=1).eval()
    for p in model.parameters():
        p.add_(0.1 * torch.randn_like(p))
    data = dump(model.state_dict(), zvtes_py.vocab_keys(), 32, 1, obs=2)
    assert b"\nobs 2\n" in data
    pool = zvtes_py.GamePool(4, ["policy", "aggro", "wall", "policy"], seed=7, rotate=True, obs=2)
    rng = np.random.default_rng(7)
    events = 0
    for _ in range(60):
        obs = pool.observe()
        events += int((obs["tok_ids"][..., 0] >= 10).sum())
        compare(model, data, obs)
        pool.act([int(rng.integers(k)) for k in obs["opt_n"]])
    assert events > 0


@torch.no_grad()
def test_a_narrower_context_layer_widens_with_zeros():
    # A feature appended to the context since a network trained (its
    # columns, one a seat, at the end): the older layer reads it as zero
    # columns, in Rust as in `remap_vocab`.
    torch.manual_seed(2)
    schema = json.loads(zvtes_py.schema())
    model = Policy(schema, d=32, layers=2).eval()
    for p in model.parameters():
        p.add_(0.1 * torch.randn_like(p))
    old = model.state_dict()
    slots = schema["ctx_slots"]
    narrow = {**old, "ctx.weight": old["ctx.weight"][:, :-slots].clone()}
    model.ctx.weight[:, -slots:] = 0
    data = dump(narrow, zvtes_py.vocab_keys(), 32, 2, obs=3)
    for obs in observations(20, seed=6, obs=3):
        compare(model, data, obs)
    widened = remap_vocab(narrow, zvtes_py.vocab_keys(), zvtes_py.vocab_keys(), model.state_dict())
    assert torch.equal(widened["ctx.weight"], model.ctx.weight)
