"""The seats' context as an input (the training board's "The seats' context
as an input"; DESIGN §5): every observation carries it, a network saved
before it loads with a zero layer and plays as it did, and training
moves the layer."""

import json

import numpy as np
import torch
import zvtes_py

from train.model import Critic, Policy, remap_vocab
from train.ppo import Config, Trainer, batch_of, forward


def test_every_observation_carries_the_context():
    schema = json.loads(zvtes_py.schema())
    pool = zvtes_py.GamePool(4, ["policy"] * 4, seed=2, obs=3)
    rng = np.random.default_rng(2)
    for _ in range(50):
        obs = pool.observe()
        assert obs["ctx"].shape == (4, schema["ctx"]) and np.isfinite(obs["ctx"]).all()
        pool.act([int(rng.integers(n)) for n in obs["opt_n"]])


def resumed(kind, schema, vocab):
    """A trained network saved before the context layer, loaded now."""
    trained = kind(schema, d=32, layers=1).eval()
    for p in trained.parameters():
        p.add_(0.1 * torch.randn_like(p))
    old = {k: v for k, v in trained.state_dict().items() if k != "ctx.weight"}
    net = kind(schema, d=32, layers=1).eval()
    net.load_state_dict(remap_vocab(old, vocab, vocab, net.state_dict()))
    return net


@torch.no_grad()
def test_a_network_saved_before_it_plays_unchanged():
    torch.manual_seed(0)
    schema = json.loads(zvtes_py.schema())
    vocab = zvtes_py.vocab_keys()
    policy, critic = resumed(Policy, schema, vocab), resumed(Critic, schema, vocab)
    assert not policy.ctx.weight.any() and not critic.ctx.weight.any()
    pool = zvtes_py.GamePool(4, ["policy"] * 4, seed=3, obs=3, critic=True)
    rng = np.random.default_rng(3)
    for _ in range(20):
        obs = pool.observe()
        batch = batch_of(obs, "cpu")
        seen = forward(policy, batch)
        batch["ctx"] = None
        blind = forward(policy, batch)
        assert torch.equal(seen[0], blind[0]) and torch.equal(seen[1], blind[1])
        args = [torch.from_numpy(obs[f"critic_{a}_{b}"]) for a in ("tok", "deck") for b in ("ids", "f", "n")]
        args = [a.float() if a.is_floating_point() else a.long() for a in args]
        ctx = torch.from_numpy(obs["ctx"]).float()
        assert torch.equal(critic(*args, ctx), critic(*args))
        pool.act([int(rng.integers(n)) for n in obs["opt_n"]])


def test_training_moves_the_context_layer(tmp_path):
    cfg = Config(
        games=8, steps=24, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", out=str(tmp_path), critic_warmup=0,
    )
    trainer = Trainer(cfg)
    assert not trainer.model.ctx.weight.any() and not trainer.critic.ctx.weight.any()
    trainer.run()
    assert trainer.model.ctx.weight.any() and trainer.critic.ctx.weight.any()


def test_an_exploiter_saved_before_it_resumes(tmp_path):
    small = dict(
        games=8, steps=16, iterations=1, epochs=1, minibatch=64, d=32, layers=1,
        eval_every=1, eval_games=4, device="cpu", critic_warmup=0, league=True, exploiter=True,
    )
    Trainer(Config(**small, out=str(tmp_path / "old"))).run()
    saved = torch.load(tmp_path / "old" / "model.pt")
    for key in ("model", "critic", "exploiter"):
        saved[key] = {k: v for k, v in saved[key].items() if k != "ctx.weight"}
    torch.save(saved, tmp_path / "old.pt")
    resumed = Trainer(Config(**small, out=str(tmp_path / "new"), resume=str(tmp_path / "old.pt")))
    assert not resumed.exploiter.model.ctx.weight.any() and not resumed.model.ctx.weight.any()
