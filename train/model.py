"""The policy (DESIGN §5 "Encoding sketch"): entity tokens through a
transformer trunk; each offered option is scored against the tokens it
points at (pointer-network style), so new cards and options change data, not
the network's shape. A value head reads the context token.

Inputs are `zvtes_py.GamePool.observe()` batches (see `zvtes::obs`): ids
`[kind/type, vocabulary, pointer, pointer(, pointer)]` with pointers as
token index + 1 (0 = none), plus float features.
"""

import torch
from torch import nn


def gather(x, ptr):
    """Rows of `x` [B, T, D] at pointers `ptr` [B, K] (index + 1, 0 = none:
    a zero row)."""
    b, t, d = x.shape
    padded = torch.cat([x.new_zeros(b, 1, d), x], dim=1)
    return padded.gather(1, ptr.long().clamp(0, t).unsqueeze(-1).expand(-1, -1, d))


class Policy(nn.Module):
    def __init__(self, schema, d=128, layers=3, heads=4):
        super().__init__()
        self.vocab = nn.Embedding(schema["vocab"], d, padding_idx=0)
        self.token_type = nn.Embedding(schema["token_types"], d, padding_idx=0)
        self.tok_f = nn.Linear(schema["tok_f"], d)
        self.tok_ptr = nn.Linear(d, d, bias=False)
        layer = nn.TransformerEncoderLayer(
            d, heads, dim_feedforward=4 * d, dropout=0.0, batch_first=True, norm_first=True
        )
        self.trunk = nn.TransformerEncoder(layer, layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(d)
        self.opt_kind = nn.Embedding(schema["choice_kinds"], d, padding_idx=0)
        self.opt_f = nn.Linear(schema["opt_f"], d)
        self.opt_ptr = nn.ModuleList(nn.Linear(d, d, bias=False) for _ in range(3))
        self.score = nn.Sequential(nn.Linear(2 * d, d), nn.GELU(), nn.Linear(d, 1))
        self.value = nn.Sequential(nn.Linear(2 * d, d), nn.GELU(), nn.Linear(d, 1))

    def forward(self, tok_ids, tok_f, tok_n, opt_ids, opt_f, opt_n):
        """Option logits [B, O] (padding at -1e9) and values [B]."""
        b, t, _ = tok_ids.shape
        o = opt_ids.shape[1]
        ar_t = torch.arange(t, device=tok_ids.device)
        ar_o = torch.arange(o, device=tok_ids.device)
        tok_pad = ar_t[None, :] >= tok_n[:, None]
        opt_pad = ar_o[None, :] >= opt_n[:, None]

        base = self.vocab(tok_ids[..., 1]) + self.token_type(tok_ids[..., 0]) + self.tok_f(tok_f)
        # A card's host, the context's minions: the pointed token's base.
        base = base + self.tok_ptr(gather(base, tok_ids[..., 2]) + gather(base, tok_ids[..., 3]))
        x = self.norm(self.trunk(base, src_key_padding_mask=tok_pad))

        # The context token is the last one; with the mean of all tokens.
        last = (tok_n - 1).clamp(min=0)
        ctx = x[torch.arange(b, device=x.device), last]
        keep = (~tok_pad).unsqueeze(-1).float()
        mean = (x * keep).sum(1) / keep.sum(1).clamp(min=1.0)
        glob = torch.cat([ctx, mean], dim=-1)

        # Only the offered options are scored (ragged: a batch's option
        # counts vary from 1 to 100+), then scattered into [B, O].
        bi, oi = (~opt_pad).nonzero(as_tuple=True)
        ids, f = opt_ids[bi, oi], opt_f[bi, oi]
        opt = self.opt_kind(ids[:, 0]) + self.vocab(ids[:, 1]) + self.opt_f(f)
        # Pointers into the flattened tokens: row 0 is the zero row (none).
        flat = torch.cat([x.new_zeros(1, x.shape[-1]), x.reshape(b * t, -1)])
        for k, lin in enumerate(self.opt_ptr):
            ptr = ids[:, 2 + k].long()
            opt = opt + lin(flat[torch.where(ptr > 0, bi * t + ptr.clamp(max=t), 0)])
        score = self.score(torch.cat([opt, ctx[bi]], dim=-1)).squeeze(-1).float()
        logits = torch.full((b, o), -1e9, device=x.device)
        logits[bi, oi] = score
        return logits, self.value(glob).squeeze(-1).float()


class Critic(nn.Module):
    """The omniscient value network (DESIGN §5 "asymmetric critic"): the
    policy's token encoder and value head, reading `encode_critic`'s
    tokens (every hand, every uncontrolled vampire). Its parameter names
    match the policy's, so it starts as a copy of a trained policy's
    encoder and value head (`load_state_dict(policy, strict=False)`)."""

    def __init__(self, schema, d=128, layers=3, heads=4):
        super().__init__()
        self.vocab = nn.Embedding(schema["vocab"], d, padding_idx=0)
        self.token_type = nn.Embedding(schema["token_types"], d, padding_idx=0)
        self.tok_f = nn.Linear(schema["tok_f"], d)
        self.tok_ptr = nn.Linear(d, d, bias=False)
        layer = nn.TransformerEncoderLayer(
            d, heads, dim_feedforward=4 * d, dropout=0.0, batch_first=True, norm_first=True
        )
        self.trunk = nn.TransformerEncoder(layer, layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(d)
        self.value = nn.Sequential(nn.Linear(2 * d, d), nn.GELU(), nn.Linear(d, 1))

    def forward(self, tok_ids, tok_f, tok_n):
        """Values [B]."""
        b, t, _ = tok_ids.shape
        tok_pad = torch.arange(t, device=tok_ids.device)[None, :] >= tok_n[:, None]
        base = self.vocab(tok_ids[..., 1]) + self.token_type(tok_ids[..., 0]) + self.tok_f(tok_f)
        base = base + self.tok_ptr(gather(base, tok_ids[..., 2]) + gather(base, tok_ids[..., 3]))
        x = self.norm(self.trunk(base, src_key_padding_mask=tok_pad))
        last = (tok_n - 1).clamp(min=0)
        ctx = x[torch.arange(b, device=x.device), last]
        keep = (~tok_pad).unsqueeze(-1).float()
        mean = (x * keep).sum(1) / keep.sum(1).clamp(min=1.0)
        return self.value(torch.cat([ctx, mean], dim=-1)).squeeze(-1).float()


def remap_vocab(state, old_keys, new_keys, fresh):
    """A state dict saved under vocabulary `old_keys`, for a network built
    on `new_keys` (the pool grew, ids shifted): each embedding row follows
    its key; rows of new keys come from `fresh` (a new network's state)."""
    if list(old_keys) == list(new_keys):
        return state
    old = {k: i for i, k in enumerate(old_keys)}
    out = dict(state)
    rows = fresh["vocab.weight"].clone()
    for i, k in enumerate(new_keys):
        if k in old:
            rows[i] = state["vocab.weight"][old[k]]
    out["vocab.weight"] = rows
    return out
