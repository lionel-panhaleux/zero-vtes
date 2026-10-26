"""A checkpoint's policy as a `.zvp` file: what `zvtes-policy` loads to play
outside Python (the server's bots, docs/online-plan.md "The bots").

`uv run python -m train.export runs/<name>/best.pt policy.zvp`

The file: a text header — `zvp 1`, `d`, `layers`, `heads`, `obs <version>`
when the network reads `zvtes_env::obs::Version::V2` or `V3` (absent: V1), one `vocab <key>`
line per vocabulary row, one `tensor <name> <dims…>` line per tensor — a
blank line, then the tensors' float32 values, little-endian, in header
order. The vocabulary keys let the loader re-key the embedding to its own
pool, as `remap_vocab` does here.
"""

import sys

import numpy as np
import torch


def dump(state, vocab, d, layers, heads=4, obs=1):
    """The `.zvp` bytes of a policy's state dict saved under vocabulary
    `vocab` (keys by id), reading observation version `obs`."""
    assert len(vocab) == state["vocab.weight"].shape[0], "vocabulary and embedding differ"
    header = ["zvp 1", f"d {d}", f"layers {layers}", f"heads {heads}"]
    if obs != 1:
        header.append(f"obs {obs}")
    header += [f"vocab {key}" for key in vocab]
    blobs = []
    for name, tensor in state.items():
        values = tensor.detach().cpu().numpy().astype("<f4")
        header.append(" ".join(["tensor", name, *map(str, values.shape)]))
        blobs.append(values.tobytes())
    return ("\n".join(header) + "\n\n").encode() + b"".join(blobs)


def export(checkpoint, out):
    saved = torch.load(checkpoint, map_location="cpu")
    state = saved["model"]
    cfg = saved.get("config", {})
    d = state["vocab.weight"].shape[1]
    layers = cfg.get("layers") or 1 + max(
        int(k.split(".")[2]) for k in state if k.startswith("trunk.layers.")
    )
    data = dump(state, saved["vocab"], d, layers, obs=cfg.get("obs", 1))
    with open(out, "wb") as f:
        f.write(data)
    return len(data)


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    size = export(sys.argv[1], sys.argv[2])
    print(f"{sys.argv[2]}: {size / 1e6:.2f} MB")


if __name__ == "__main__":
    main()
