"""One tiny PPO iteration runs end to end, evaluation included, and
changes the policy."""

import torch

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
