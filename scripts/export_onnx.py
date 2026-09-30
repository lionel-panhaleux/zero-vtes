"""Export the policy network to ONNX (docs/human-play.md, slice 3).

    uv run --with onnx --with onnxscript --with onnxruntime \
        python scripts/export_onnx.py [runs/p4-critic/model.pt] --out /tmp/policy.onnx

Inputs `tok_ids [B,T,4] i64, tok_f [B,T,64] f32, tok_n [B] i64,
opt_ids [B,O,5] i64, opt_f [B,O,24] f32, opt_n [B] i64`; outputs
`logits [B,O]` (padding at -1e9) and `value [B]`. B, T and O are dynamic.

`Policy.forward` scores only the offered options (`nonzero`, then an indexed
scatter): shapes that depend on data, which static-shape runtimes (tract)
cannot plan. `Dense` is the same function scoring every padded option row
and masking after; it is checked against `Policy` here, then the ONNX file
against it (onnxruntime) on real observations of several shapes.

`--original` exports `Policy.forward` as is (to see which operators it
needs); `--dump DIR` writes the test cases as raw little-endian files for other
runtimes (`DIR/cases.txt`: one `name T O` line per case).
"""

import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np
import torch
from torch import nn

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import zvtes_py

from train.model import Policy, gather, remap_vocab

NAMES = ("tok_ids", "tok_f", "tok_n", "opt_ids", "opt_f", "opt_n")


class Dense(nn.Module):
    """`Policy.forward` without data-dependent shapes."""

    def __init__(self, policy):
        super().__init__()
        self.p = policy

    def forward(self, tok_ids, tok_f, tok_n, opt_ids, opt_f, opt_n):
        p = self.p
        t, o = tok_ids.shape[1], opt_ids.shape[1]
        tok_pad = torch.arange(t)[None, :] >= tok_n[:, None]
        opt_pad = torch.arange(o)[None, :] >= opt_n[:, None]

        base = p.vocab(tok_ids[..., 1]) + p.token_type(tok_ids[..., 0]) + p.tok_f(tok_f)
        base = base + p.tok_ptr(
            gather(base, tok_ids[..., 2]) + gather(base, tok_ids[..., 3])
        )
        x = p.norm(p.trunk(base, src_key_padding_mask=tok_pad))

        last = (tok_n - 1).clamp(min=0)
        ctx = x.gather(1, last[:, None, None].expand(-1, 1, x.shape[-1])).squeeze(1)
        keep = (~tok_pad).unsqueeze(-1).float()
        mean = (x * keep).sum(1) / keep.sum(1).clamp(min=1.0)
        glob = torch.cat([ctx, mean], dim=-1)

        opt = p.opt_kind(opt_ids[..., 0]) + p.vocab(opt_ids[..., 1]) + p.opt_f(opt_f)
        for k, lin in enumerate(p.opt_ptr):
            opt = opt + lin(gather(x, opt_ids[..., 2 + k]))
        pair = torch.cat([opt, ctx[:, None, :].expand(-1, o, -1)], dim=-1)
        score = p.score(pair).squeeze(-1)
        logits = torch.where(opt_pad, torch.full_like(score, -1e9), score)
        return logits, p.value(glob).squeeze(-1)


def load(path):
    schema = json.loads(zvtes_py.schema())
    saved = torch.load(path, map_location="cpu")
    cfg = saved.get("config", {})
    policy = Policy(schema, d=cfg.get("d", 128), layers=cfg.get("layers", 3))
    vocab = zvtes_py.vocab_keys()
    state = remap_vocab(
        saved["model"], saved.get("vocab", vocab), vocab, policy.state_dict()
    )
    policy.load_state_dict(state)
    return policy.eval()


def observations(n, seed):
    """`n` single-decision batches of distinct (tokens, options) shapes,
    from games played at random."""
    pool = zvtes_py.GamePool(8, ["net:0"] * 5, seed=seed)
    rng = np.random.default_rng(seed)
    seen, out = set(), []
    for _ in range(400):
        obs = pool.observe()
        for i in range(len(obs["tok_n"])):
            t, o = int(obs["tok_n"][i]), int(obs["opt_n"][i])
            if (t, o) in seen or o < 2:
                continue
            seen.add((t, o))
            out.append(
                (
                    obs["tok_ids"][i : i + 1, :t].astype(np.int64),
                    obs["tok_f"][i : i + 1, :t].astype(np.float32),
                    np.array([t], np.int64),
                    obs["opt_ids"][i : i + 1, :o].astype(np.int64),
                    obs["opt_f"][i : i + 1, :o].astype(np.float32),
                    np.array([o], np.int64),
                )
            )
        pool.act([int(rng.integers(n)) for n in obs["opt_n"]])
    # Spread over the option counts: the small and the large decisions.
    out.sort(key=lambda c: (c[5][0], c[2][0]))
    return [
        out[i]
        for i in sorted(set(np.linspace(0, len(out) - 1, n).astype(int).tolist()))
    ]


def padded(cases):
    """The cases as one padded batch."""
    b = len(cases)
    t, o = max(int(c[2][0]) for c in cases), max(int(c[5][0]) for c in cases)
    out = [
        np.zeros((b, t, cases[0][0].shape[2]), np.int64),
        np.zeros((b, t, cases[0][1].shape[2]), np.float32),
        np.zeros(b, np.int64),
        np.zeros((b, o, cases[0][3].shape[2]), np.int64),
        np.zeros((b, o, cases[0][4].shape[2]), np.float32),
        np.zeros(b, np.int64),
    ]
    for i, c in enumerate(cases):
        tn, on = int(c[2][0]), int(c[5][0])
        out[0][i, :tn], out[1][i, :tn], out[2][i] = c[0][0], c[1][0], tn
        out[3][i, :on], out[4][i, :on], out[5][i] = c[3][0], c[4][0], on
    return tuple(out)


def run(module, case):
    with torch.no_grad():
        logits, value = module(*(torch.from_numpy(a) for a in case))
    return logits.numpy(), value.numpy()


def diff(a, b):
    return max(float(np.abs(a[0] - b[0]).max()), float(np.abs(a[1] - b[1]).max()))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("checkpoint", nargs="?", default="runs/p4-critic/model.pt")
    ap.add_argument("--out", required=True)
    ap.add_argument("--opset", type=int, default=17)
    ap.add_argument("--original", action="store_true")
    ap.add_argument("--dump", default=None)
    ap.add_argument("--cases", type=int, default=12)
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()

    # The encoder's fused inference path does not trace; dropout is 0, so
    # the plain path computes the same.
    torch.backends.mha.set_fastpath_enabled(False)
    policy = load(args.checkpoint)
    dense = Dense(policy).eval()
    cases = observations(args.cases, args.seed)
    batch = padded(cases)
    print("cases (T, O):", [(int(c[2][0]), int(c[5][0])) for c in cases])

    worst = max(diff(run(policy, c), run(dense, c)) for c in cases + [batch])
    print(f"Dense vs Policy (PyTorch): max abs diff {worst:.3g}")

    module = policy if args.original else dense
    example = tuple(torch.from_numpy(a) for a in batch)
    # The TorchScript exporter (`dynamo=False`) bakes the example's T into
    # the attention reshapes: only the dynamo exporter keeps T and O dynamic.
    b, t, o = torch.export.Dim("B"), torch.export.Dim("T"), torch.export.Dim("O")
    shapes = ({0: b, 1: t}, {0: b, 1: t}, {0: b}, {0: b, 1: o}, {0: b, 1: o}, {0: b})
    torch.onnx.export(
        module,
        example,
        args.out,
        input_names=list(NAMES),
        output_names=["logits", "value"],
        opset_version=args.opset,
        dynamo=True,
        external_data=False,
        dynamic_shapes=shapes,
    )

    import onnx

    # `ConstantOfShape` without a value is float32 zeros by the ONNX
    # specification; tract types it f64: spell the default out.
    proto = onnx.load(args.out)
    for node in proto.graph.node:
        if node.op_type == "ConstantOfShape" and not node.attribute:
            zero = onnx.helper.make_tensor("value", onnx.TensorProto.FLOAT, [1], [0.0])
            node.attribute.append(onnx.helper.make_attribute("value", zero))
    onnx.save(proto, args.out)
    graph = proto.graph
    ops = {}
    for node in graph.node:
        ops[node.op_type] = ops.get(node.op_type, 0) + 1
    print(
        f"{args.out}: {Path(args.out).stat().st_size / 1e6:.2f} MB, {len(graph.node)} nodes"
    )
    print("ops:", dict(sorted(ops.items())))

    try:
        import onnxruntime as ort
    except ImportError:
        print("onnxruntime not installed: no runtime check")
    else:
        sess = ort.InferenceSession(args.out, providers=["CPUExecutionProvider"])
        worst, mismatch = 0.0, 0
        for c in cases + [batch]:
            ref = run(module, c)
            got = sess.run(None, dict(zip(NAMES, c)))
            worst = max(worst, diff(ref, got))
            mismatch += int((ref[0].argmax(-1) != got[0].argmax(-1)).sum())
        print(
            f"onnxruntime vs PyTorch: max abs diff {worst:.3g}, argmax mismatches {mismatch}"
        )
        c = cases[len(cases) // 2]
        feed = dict(zip(NAMES, c))
        start = time.perf_counter()
        for _ in range(200):
            sess.run(None, feed)
        ms = (time.perf_counter() - start) / 200 * 1e3
        print(
            f"onnxruntime: {ms:.2f} ms per decision (T={int(c[2][0])}, O={int(c[5][0])})"
        )

    if args.dump:
        out = Path(args.dump)
        out.mkdir(parents=True, exist_ok=True)
        lines = []
        for i, c in enumerate(cases):
            logits, value = run(module, c)
            for name, a in zip(NAMES + ("logits", "value"), c + (logits, value)):
                a.astype("<i8" if a.dtype == np.int64 else "<f4").tofile(
                    out / f"{i}.{name}.bin"
                )
            lines.append(f"{i} {int(c[2][0])} {int(c[5][0])}")
        (out / "cases.txt").write_text("\n".join(lines) + "\n")
        print(f"{len(cases)} cases in {out}")


if __name__ == "__main__":
    main()
