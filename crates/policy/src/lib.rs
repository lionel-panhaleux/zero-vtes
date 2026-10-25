//! The trained policy's inference (docs/online-plan.md "The bots"): the forward
//! pass of `train/model.py`'s `Policy`, written by hand for one decision —
//! no padding, no batch — and an [`Agent`] sampling from it. The network is
//! small and fixed in form (embeddings, pre-norm transformer layers with a
//! ReLU feed-forward, GELU heads, pointer-scored options), so this costs no
//! dependency and runs wherever the engine does; the price is keeping it in
//! step with `train/model.py` (`train/tests/test_export.py` compares the
//! two on real observations).
//!
//! Weights come as a `.zvp` file (`python -m train.export`): a text header
//! — `zvp 1`, `d`, `layers`, `heads`, optionally `obs <version>` (the
//! observation it reads, `obs::Version`: 1 when absent, every file written
//! before 2026-10-03), one `vocab <key>` line per vocabulary
//! row, one `tensor <name> <dims…>` line per tensor — a blank line, then
//! the tensors' f32 values, little-endian, in header order. The vocabulary
//! is re-keyed on load (as `remap_vocab` does): a checkpoint trained on an
//! older pool still plays, its unknown cards as zero rows.

use std::collections::HashMap;

pub mod chores;
pub mod profiles;

use zvtes::agent::Agent;
use zvtes::cards::Stat;
use zvtes::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use zvtes::game::Engine;
use zvtes::ids::SeatId;
use zvtes::library::{CardType, LIBRARY};
use zvtes::rng::Rng;
use zvtes_env::habits::{self, Habit};
use zvtes_env::obs::{self, Obs};
use zvtes_env::politics;

struct Layer {
    norm1: (Vec<f32>, Vec<f32>),
    in_proj: (Vec<f32>, Vec<f32>),
    out_proj: (Vec<f32>, Vec<f32>),
    norm2: (Vec<f32>, Vec<f32>),
    linear1: (Vec<f32>, Vec<f32>),
    linear2: (Vec<f32>, Vec<f32>),
}

/// A two-layer head: Linear(2d → d), GELU, Linear(d → 1).
struct Head {
    hidden: (Vec<f32>, Vec<f32>),
    out: (Vec<f32>, Vec<f32>),
}

pub struct Net {
    /// The encoding the network was trained on.
    pub version: obs::Version,
    d: usize,
    heads: usize,
    zero: Vec<f32>,
    vocab: Vec<f32>,
    token_type: Vec<f32>,
    tok_f: (Vec<f32>, Vec<f32>),
    tok_ptr: Vec<f32>,
    layers: Vec<Layer>,
    norm: (Vec<f32>, Vec<f32>),
    opt_kind: Vec<f32>,
    opt_f: (Vec<f32>, Vec<f32>),
    opt_ptr: [Vec<f32>; 3],
    score: Head,
    value: Head,
    /// The seats' context into the context token (`ctx`, [d, LEN]); none
    /// in a network trained before it.
    ctx: Option<Vec<f32>>,
}

/// `out = W · x (+ b)`, `W` row-major with `x.len()` columns.
fn linear(w: &[f32], b: Option<&[f32]>, x: &[f32], out: &mut [f32]) {
    let cols = x.len();
    for (r, o) in out.iter_mut().enumerate() {
        *o = dot(&w[r * cols..(r + 1) * cols], x) + b.map_or(0.0, |b| b[r]);
    }
}

/// Eight accumulators: the compiler vectorizes them.
fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    let ((ca, ta), (cb, tb)) = (a.as_chunks::<8>(), b.as_chunks::<8>());
    for (x, y) in ca.iter().zip(cb) {
        for k in 0..8 {
            acc[k] += x[k] * y[k];
        }
    }
    let tail: f32 = ta.iter().zip(tb).map(|(x, y)| x * y).sum();
    acc.iter().sum::<f32>() + tail
}

fn layer_norm(p: &(Vec<f32>, Vec<f32>), x: &[f32], out: &mut [f32]) {
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
    let inv = 1.0 / (var + 1e-5).sqrt();
    for i in 0..x.len() {
        out[i] = (x[i] - mean) * inv * p.0[i] + p.1[i];
    }
}

/// The error function (Abramowitz & Stegun 7.1.26, |error| ≤ 1.5e-7) —
/// `f32` has none, and the heads' GELU is the exact one.
fn erf(x: f32) -> f32 {
    let (sign, x) = (x.signum(), x.abs() as f64);
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = ((((1.061405429 * t - 1.453152027) * t + 1.421413741) * t - 0.284496736) * t
        + 0.254829592)
        * t;
    sign * (1.0 - poly * (-x * x).exp()) as f32
}

fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + erf(x * std::f32::consts::FRAC_1_SQRT_2))
}

impl Head {
    fn apply(&self, input: &[f32]) -> f32 {
        let mut hid = vec![0f32; self.hidden.1.len()];
        linear(&self.hidden.0, Some(&self.hidden.1), input, &mut hid);
        hid.iter_mut().for_each(|v| *v = gelu(*v));
        let mut y = [0f32];
        linear(&self.out.0, Some(&self.out.1), &hid, &mut y);
        y[0]
    }
}

impl Net {
    /// A network from a `.zvp` file, its vocabulary re-keyed to the
    /// engine's (`obs::vocab_keys`).
    pub fn load(bytes: &[u8]) -> Result<Net, String> {
        let split = bytes
            .windows(2)
            .position(|w| w == b"\n\n")
            .ok_or("no header")?;
        let header = std::str::from_utf8(&bytes[..split]).map_err(|e| e.to_string())?;
        let mut data = &bytes[split + 2..];
        let mut lines = header.lines();
        if lines.next() != Some("zvp 1") {
            return Err("not a zvp 1 file".into());
        }
        let mut meta: HashMap<&str, usize> = HashMap::new();
        let mut keys: Vec<&str> = Vec::new();
        let mut tensors: HashMap<&str, (Vec<usize>, Vec<f32>)> = HashMap::new();
        for line in lines {
            let (kind, rest) = line
                .split_once(' ')
                .ok_or_else(|| format!("bad line: {line}"))?;
            match kind {
                "d" | "layers" | "heads" | "obs" => {
                    meta.insert(kind, rest.parse().map_err(|_| format!("bad line: {line}"))?);
                }
                "vocab" => keys.push(rest),
                "tensor" => {
                    let mut it = rest.split(' ');
                    let name = it.next().ok_or("unnamed tensor")?;
                    let shape: Vec<usize> = it
                        .map(|v| v.parse().map_err(|_| format!("bad line: {line}")))
                        .collect::<Result<_, _>>()?;
                    let n: usize = shape.iter().product();
                    if data.len() < 4 * n {
                        return Err(format!("{name}: truncated"));
                    }
                    let values = data[..4 * n]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| f32::from_le_bytes(*c))
                        .collect();
                    data = &data[4 * n..];
                    tensors.insert(name, (shape, values));
                }
                _ => return Err(format!("bad line: {line}")),
            }
        }
        let get = |k: &str| meta.get(k).copied().ok_or_else(|| format!("no {k}"));
        let (d, n_layers, heads) = (get("d")?, get("layers")?, get("heads")?);
        let version = match meta.get("obs").copied().unwrap_or(1) {
            1 => obs::Version::V1,
            2 => obs::Version::V2,
            3 => obs::Version::V3,
            v => return Err(format!("unknown obs version {v}")),
        };
        // The seats' context layer, if the network has one: [d, its LEN],
        // features appended since read as zero columns.
        let ctx_saved = tensors.remove("ctx.weight");
        let mut take = |name: &str, shape: &[usize]| -> Result<Vec<f32>, String> {
            let (s, v) = tensors
                .remove(name)
                .ok_or_else(|| format!("no tensor {name}"))?;
            if !shape.is_empty() && s != shape {
                return Err(format!("{name}: shape {s:?}, expected {shape:?}"));
            }
            Ok(v)
        };
        let mut pair =
            |name: &str, rows: usize, cols: usize| -> Result<(Vec<f32>, Vec<f32>), String> {
                Ok((
                    take(&format!("{name}.weight"), &[rows, cols])?,
                    take(&format!("{name}.bias"), &[rows])?,
                ))
            };

        let tok_f = pair("tok_f", d, obs::TOK_F)?;
        let opt_f = pair("opt_f", d, obs::OPT_F)?;
        let norm = (take("norm.weight", &[d])?, take("norm.bias", &[d])?);
        let mut layers = Vec::new();
        for l in 0..n_layers {
            let n = |s: &str| format!("trunk.layers.{l}.{s}");
            layers.push(Layer {
                norm1: (
                    take(&n("norm1.weight"), &[d])?,
                    take(&n("norm1.bias"), &[d])?,
                ),
                in_proj: (
                    take(&n("self_attn.in_proj_weight"), &[3 * d, d])?,
                    take(&n("self_attn.in_proj_bias"), &[3 * d])?,
                ),
                out_proj: (
                    take(&n("self_attn.out_proj.weight"), &[d, d])?,
                    take(&n("self_attn.out_proj.bias"), &[d])?,
                ),
                norm2: (
                    take(&n("norm2.weight"), &[d])?,
                    take(&n("norm2.bias"), &[d])?,
                ),
                linear1: (
                    take(&n("linear1.weight"), &[4 * d, d])?,
                    take(&n("linear1.bias"), &[4 * d])?,
                ),
                linear2: (
                    take(&n("linear2.weight"), &[d, 4 * d])?,
                    take(&n("linear2.bias"), &[d])?,
                ),
            });
        }
        let mut head = |name: &str| -> Result<Head, String> {
            Ok(Head {
                hidden: (
                    take(&format!("{name}.0.weight"), &[d, 2 * d])?,
                    take(&format!("{name}.0.bias"), &[d])?,
                ),
                out: (
                    take(&format!("{name}.2.weight"), &[1, d])?,
                    take(&format!("{name}.2.bias"), &[1])?,
                ),
            })
        };
        let (score, value) = (head("score")?, head("value")?);
        let opt_ptr = [
            take("opt_ptr.0.weight", &[d, d])?,
            take("opt_ptr.1.weight", &[d, d])?,
            take("opt_ptr.2.weight", &[d, d])?,
        ];
        let tok_ptr = take("tok_ptr.weight", &[d, d])?;
        let opt_kind = take("opt_kind.weight", &[])?;

        // Embedding rows follow their key; a key the file lacks is a zero
        // row. Token types are appended over time: missing ones are zero
        // rows too (`row`).
        let saved = take("vocab.weight", &[keys.len(), d])?;
        let at: HashMap<&str, usize> = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        let mut vocab = Vec::new();
        for key in obs::vocab_keys() {
            match at.get(key.as_str()) {
                Some(&i) => vocab.extend_from_slice(&saved[i * d..(i + 1) * d]),
                None => vocab.extend(std::iter::repeat_n(0.0, d)),
            }
        }
        let token_type = take("token_type.weight", &[])?;
        let ctx = match ctx_saved {
            Some((s, v)) => {
                let len = zvtes_env::context::LEN;
                if s.len() != 2 || s[0] != d || s[1] > len {
                    return Err(format!("ctx.weight: shape {s:?}, expected [{d}, ≤{len}]"));
                }
                let mut w = vec![0.0; d * len];
                for r in 0..d {
                    w[r * len..r * len + s[1]].copy_from_slice(&v[r * s[1]..(r + 1) * s[1]]);
                }
                Some(w)
            }
            None => None,
        };

        Ok(Net {
            version,
            d,
            heads,
            zero: vec![0.0; d],
            vocab,
            token_type,
            tok_f,
            tok_ptr,
            layers,
            norm,
            opt_kind,
            opt_f,
            opt_ptr,
            score,
            value,
            ctx,
        })
    }

    /// An embedding row; a zero row past the table (an id the checkpoint
    /// predates).
    fn row<'a>(&'a self, table: &'a [f32], i: i32) -> &'a [f32] {
        let i = i.max(0) as usize;
        table
            .get(i * self.d..(i + 1) * self.d)
            .unwrap_or(&self.zero)
    }

    /// The options' logits and the value of one observation.
    pub fn forward(&self, o: &Obs) -> (Vec<f32>, f32) {
        let d = self.d;
        let t = o.tok_ids.len();
        let mut tmp = vec![0f32; d];
        let mut out = vec![0f32; d];

        // Tokens: vocabulary + type + features, then the pointed tokens'
        // bases (a card's host, the context's minions).
        let mut base = vec![0f32; t * d];
        for i in 0..t {
            let x = &mut base[i * d..(i + 1) * d];
            linear(&self.tok_f.0, Some(&self.tok_f.1), &o.tok_f[i], x);
            let ids = &o.tok_ids[i];
            let (a, b) = (
                self.row(&self.token_type, ids[0]),
                self.row(&self.vocab, ids[1]),
            );
            for k in 0..d {
                x[k] += a[k] + b[k];
            }
        }
        // `V3`'s deck rows: embedded alike, their mean added to the
        // context token (the last), out of attention.
        if !o.deck_ids.is_empty() && t > 0 {
            let mut pooled = vec![0f32; d];
            for (ids, f) in o.deck_ids.iter().zip(&o.deck_f) {
                linear(&self.tok_f.0, Some(&self.tok_f.1), f, &mut tmp);
                let (a, b) = (
                    self.row(&self.token_type, ids[0]),
                    self.row(&self.vocab, ids[1]),
                );
                for k in 0..d {
                    pooled[k] += tmp[k] + a[k] + b[k];
                }
            }
            let n = o.deck_ids.len() as f32;
            for k in 0..d {
                base[(t - 1) * d + k] += pooled[k] / n;
            }
        }
        // The seats' context, added to the context token alike.
        if let Some(w) = &self.ctx {
            debug_assert_eq!(o.ctx.len(), zvtes_env::context::LEN, "the seats' context");
            if t > 0 {
                linear(w, None, &o.ctx, &mut tmp);
                for k in 0..d {
                    base[(t - 1) * d + k] += tmp[k];
                }
            }
        }
        let mut x = base.clone();
        for i in 0..t {
            tmp.fill(0.0);
            for k in 2..4 {
                let ptr = o.tok_ids[i][k].clamp(0, t as i32) as usize;
                if ptr > 0 {
                    for j in 0..d {
                        tmp[j] += base[(ptr - 1) * d + j];
                    }
                }
            }
            linear(&self.tok_ptr, None, &tmp, &mut out);
            for j in 0..d {
                x[i * d + j] += out[j];
            }
        }

        // The trunk: pre-norm self-attention and feed-forward.
        let (h, hd) = (self.heads, d / self.heads);
        let scale = 1.0 / (hd as f32).sqrt();
        let mut qkv = vec![0f32; t * 3 * d];
        let mut att = vec![0f32; t];
        let mut mix = vec![0f32; d];
        let mut ff = vec![0f32; 4 * d];
        let mut delta = vec![0f32; t * d];
        for layer in &self.layers {
            for i in 0..t {
                layer_norm(&layer.norm1, &x[i * d..(i + 1) * d], &mut tmp);
                linear(
                    &layer.in_proj.0,
                    Some(&layer.in_proj.1),
                    &tmp,
                    &mut qkv[i * 3 * d..(i + 1) * 3 * d],
                );
            }
            for i in 0..t {
                mix.fill(0.0);
                for a in 0..h {
                    let q = &qkv[i * 3 * d + a * hd..i * 3 * d + (a + 1) * hd];
                    let mut max = f32::MIN;
                    for j in 0..t {
                        let k = &qkv[j * 3 * d + d + a * hd..j * 3 * d + d + (a + 1) * hd];
                        att[j] = dot(q, k) * scale;
                        max = max.max(att[j]);
                    }
                    let mut sum = 0f32;
                    for v in att.iter_mut() {
                        *v = (*v - max).exp();
                        sum += *v;
                    }
                    for j in 0..t {
                        let v = &qkv[j * 3 * d + 2 * d + a * hd..j * 3 * d + 2 * d + (a + 1) * hd];
                        let w = att[j] / sum;
                        for c in 0..hd {
                            mix[a * hd + c] += w * v[c];
                        }
                    }
                }
                linear(
                    &layer.out_proj.0,
                    Some(&layer.out_proj.1),
                    &mix,
                    &mut delta[i * d..(i + 1) * d],
                );
            }
            for (v, dv) in x.iter_mut().zip(&delta) {
                *v += dv;
            }
            for i in 0..t {
                layer_norm(&layer.norm2, &x[i * d..(i + 1) * d], &mut tmp);
                linear(&layer.linear1.0, Some(&layer.linear1.1), &tmp, &mut ff);
                ff.iter_mut().for_each(|v| *v = v.max(0.0));
                linear(&layer.linear2.0, Some(&layer.linear2.1), &ff, &mut out);
                for j in 0..d {
                    x[i * d + j] += out[j];
                }
            }
        }
        let mut mean = vec![0f32; d];
        for i in 0..t {
            layer_norm(&self.norm, &x[i * d..(i + 1) * d], &mut tmp);
            x[i * d..(i + 1) * d].copy_from_slice(&tmp);
            for j in 0..d {
                mean[j] += tmp[j] / t as f32;
            }
        }

        // The context token is the last one. Each option: its kind, its
        // vocabulary entry, its features and the tokens it points at,
        // scored with the context.
        let ctx = x[(t - 1) * d..].to_vec();
        let mut pair = vec![0f32; 2 * d];
        pair[d..].copy_from_slice(&ctx);
        let mut logits = Vec::with_capacity(o.opt_ids.len());
        for (ids, f) in o.opt_ids.iter().zip(&o.opt_f) {
            linear(&self.opt_f.0, Some(&self.opt_f.1), f, &mut pair[..d]);
            let (a, b) = (
                self.row(&self.opt_kind, ids[0]),
                self.row(&self.vocab, ids[1]),
            );
            for j in 0..d {
                pair[j] += a[j] + b[j];
            }
            for (k, w) in self.opt_ptr.iter().enumerate() {
                let ptr = ids[2 + k].clamp(0, t as i32) as usize;
                if ptr > 0 {
                    linear(w, None, &x[(ptr - 1) * d..ptr * d], &mut out);
                    for j in 0..d {
                        pair[j] += out[j];
                    }
                }
            }
            logits.push(self.score.apply(&pair));
        }
        pair[..d].copy_from_slice(&ctx);
        pair[d..].copy_from_slice(&mean);
        (logits, self.value.apply(&pair))
    }
}

/// Nudges on the network's logits (owner, 2026-10-05: "a guiding harness
/// like we do for the votes", "until it learns by itself"): added before
/// sampling, inside the distribution training learns from (`opt_bias` in
/// `crates/py`), faded out over a run. Every option stays offered and
/// legal (rules-qa 17.4): a nudge only makes one likelier.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Nudges {
    /// Added to a rescue of one's own vampire (`habits::Habit::Rescue`).
    pub rescue: f32,
    /// Added to passing one's discard phase when a card could be cycled:
    /// the hand kept (the network cycles in 95% of its discard phases, the
    /// owner in 53%).
    pub pass: f32,
    /// Taken off cycling a reaction card in one's discard phase (the
    /// owner keeps them: 31% of the hand, 23% of the cards cycled; the
    /// network cycles them as often as it holds them).
    pub reaction: f32,
    /// Taken off a bleed for nothing (`empty_bleed`; owner, 2026-10-05:
    /// "it bleeds with Aggressive Corpse and that minion has zero bleed …
    /// doing an action with no effect should not be favored").
    pub bleed: f32,
    /// Taken off ending one's influence phase while a crypt draw is still
    /// offered (4 transfers unspent) with 5 pool or more (owner,
    /// 2026-10-05: "it has a tendency to stay idle with all in torpor when
    /// it could draw a crypt card or try to influence one more vampire";
    /// the network ends 69% of its influence phases with none made, pool
    /// 11.7 on average, the owner 25%).
    pub idle: f32,
    /// Added to a crypt draw (4 transfers) with 5 pool or more (owner,
    /// 2026-10-10: "the bot never pays 4 transfers to draw from the
    /// crypt": offered ~40 times a game, never taken — p8 gave it 6e-8 —
    /// the owner 11% of the times it is offered).
    pub draw: f32,
    /// Taken off a transfer onto a vampire that cannot be influenced out
    /// (`stuck_transfer`; owner, 2026-10-10: a Toreador bot's pool spent
    /// on a second copy of its unique vampire — the network makes it in
    /// 35% of the influence decisions offering one, the owner never).
    pub stuck: f32,
    /// Taken off moving a nightmare counter onto one's own minion
    /// (`own_nightmare`; owner, 2026-10-10: "never a good move").
    pub nightmare: f32,
}

impl Nudges {
    pub fn is_zero(&self) -> bool {
        *self == Nudges::default()
    }

    /// Each nudge by name, as a record keeps them (a new field fails to
    /// compile here until it is named).
    pub fn named(&self) -> [(&'static str, f32); 8] {
        let Nudges {
            rescue,
            pass,
            reaction,
            bleed,
            idle,
            draw,
            stuck,
            nightmare,
        } = *self;
        [
            ("rescue", rescue),
            ("pass", pass),
            ("reaction", reaction),
            ("bleed", bleed),
            ("idle", idle),
            ("draw", draw),
            ("stuck", stuck),
            ("nightmare", nightmare),
        ]
    }

    /// The nudge on option `o` of `req`. Reads only the deciding seat's
    /// legal view.
    pub fn bias(&self, e: &Engine, req: &DecisionRequest, o: &Choice) -> f32 {
        let mut b = 0.0;
        if self.rescue != 0.0 && habits::habit(e, req.seat, o) == Some(Habit::Rescue) {
            b += self.rescue;
        }
        if self.bleed != 0.0 && empty_bleed(e, o) {
            b -= self.bleed;
        }
        if self.idle != 0.0 && *o == Choice::Pass && idle_influence(e, req) {
            b -= self.idle;
        }
        if self.draw != 0.0 && *o == Choice::DrawCrypt && idle_influence(e, req) {
            b += self.draw;
        }
        if self.stuck != 0.0 && stuck_transfer(e, req.seat, o) {
            b -= self.stuck;
        }
        if self.nightmare != 0.0 && own_nightmare(e, req.seat, o) == Some(true) {
            b -= self.nightmare;
        }
        if (self.pass != 0.0 || self.reaction != 0.0) && cycling(e, req) {
            let hand = &e.state.seats[req.seat.index()].hand;
            match *o {
                Choice::Pass => b += self.pass,
                Choice::Discard(i)
                    if LIBRARY[hand[i as usize] as usize]
                        .types
                        .contains(&CardType::Reaction) =>
                {
                    b -= self.reaction
                }
                _ => {}
            }
        }
        b
    }
}

/// `o` is a rulebook bleed by a minion that bleeds for 0 before any
/// action modifier: its base bleed (an ally's printed one: Aggressive
/// Corpse) and the cards it bears. An action card's bleed is not one.
pub fn empty_bleed(e: &Engine, o: &Choice) -> bool {
    let st = &e.state;
    matches!(*o, Choice::TakeAction { minion, action: ActionKind::Bleed, .. }
        if st.base_bleed(minion) as i32 + st.bearer_mod(minion, Stat::Bleed, ActionKind::Bleed) <= 0)
}

/// `o` is a transfer onto a copy of a unique vampire `seat` already has
/// in play: it is not offered out while that copy stays (no voluntary
/// self-contest, `zvtes::influence`), the pool on it stuck.
pub fn stuck_transfer(e: &Engine, seat: SeatId, o: &Choice) -> bool {
    let Choice::Transfer { slot } = *o else {
        return false;
    };
    let card = e.state.seats[seat.index()].uncontrolled[slot as usize].card;
    card != zvtes::state::CREATED
        && zvtes::crypt::CRYPT[card as usize].name != "Anarch Convert"
        && e.state.controls_copy(seat, card)
}

/// `o` moves a nightmare counter (Week of Nightmares): onto a minion of
/// `seat`'s? `None` when it moves none.
pub fn own_nightmare(e: &Engine, seat: SeatId, o: &Choice) -> Option<bool> {
    let Choice::UseAbility {
        source: zvtes::cards::AbilitySource::Card(c),
        ability,
        target: zvtes::cards::Target::Minion(t),
        ..
    } = *o
    else {
        return None;
    };
    let def = zvtes::cards::lookup(e.state.cards[c.index()].card)?;
    def.abilities
        .get(ability as usize)?
        .effects
        .iter()
        .any(|f| matches!(f, zvtes::cards::Effect::CounterToTarget(_)))
        .then(|| e.state.minions[t.index()].owner == seat)
}

/// `req` is a seat's influence phase with a crypt draw still offered (4
/// transfers left) and 5 pool or more: passing it leaves them unused.
pub fn idle_influence(e: &Engine, req: &DecisionRequest) -> bool {
    req.window == WindowLabel::InfluenceTransfers
        && e.state.seats[req.seat.index()].pool >= 5
        && req.options.contains(&Choice::DrawCrypt)
}

/// The choices no run has trained with (their observation folded into
/// another kind, `obs::option`), never a network's: a withdrawal, a path
/// followed. `None` when none is offered.
pub fn untrained(req: &DecisionRequest) -> Option<Vec<bool>> {
    let mask: Vec<bool> = req
        .options
        .iter()
        .map(|o| !matches!(o, Choice::WithdrawFromGame | Choice::FollowPath { .. }))
        .collect();
    mask.contains(&false).then_some(mask)
}

/// The options a network seat may take in a block window (owner,
/// 2026-10-15: "mask in the bot layer, train and serve"): the engine
/// offers every legal attempt (rules-qa 73.8), those that cannot reach the
/// action's stealth (`Engine::block_can_reach`) too; a network never makes
/// one, and a window of futile attempts only leaves its pass. `None` when
/// no futile attempt is offered. A failed attempt costs nothing, so no
/// reward would ever teach this. The outermost mask, in training
/// (`crates/py`) and on the server alike.
pub fn reachable_blocks(e: &Engine, req: &DecisionRequest) -> Option<Vec<bool>> {
    let mask: Vec<bool> = req
        .options
        .iter()
        .map(|o| !matches!(*o, Choice::DeclareBlock { minion } if !e.block_can_reach(req.seat, minion)))
        .collect();
    mask.contains(&false).then_some(mask)
}

/// Two masks (`None`: no opinion) as one: the options both leave, or the
/// first's alone when they would leave none.
pub fn combine(a: Option<Vec<bool>>, b: Option<Vec<bool>>) -> Option<Vec<bool>> {
    match (a, b) {
        (Some(a), Some(b)) if a.iter().zip(&b).any(|(x, y)| *x && *y) => {
            Some(a.iter().zip(&b).map(|(x, y)| *x && *y).collect())
        }
        (Some(a), _) => Some(a),
        (None, b) => b,
    }
}

/// `req` is a seat's own discard phase with a card to cycle.
pub fn cycling(e: &Engine, req: &DecisionRequest) -> bool {
    req.window == WindowLabel::DiscardPhase
        && e.state.active == req.seat
        && req.options.iter().any(|o| matches!(o, Choice::Discard(_)))
}

/// The options a network seat may take (None: all): blocks it can
/// reach, the kinds it was trained on, the default politics, and the
/// chores never declined when `chores`.
pub fn mask(e: &Engine, req: &DecisionRequest, chores: bool) -> Option<Vec<bool>> {
    combine(
        combine(reachable_blocks(e, req), untrained(req)),
        combine(
            politics::mask(e, req),
            chores.then(|| chores::mask(e, req)).flatten(),
        ),
    )
}

/// The policy as a player: samples an option from the network's
/// distribution, as training does (`train/ppo.py` `act`), with the nudges
/// training plays with (`nudges`), sharpened by a temperature under 1
/// (`temperature`: the human-play bot).
#[derive(Clone)]
pub struct NetAgent {
    net: std::sync::Arc<Net>,
    rng: Rng,
    temperature: f32,
    nudges: Nudges,
    chores: bool,
}

impl NetAgent {
    pub fn new(net: std::sync::Arc<Net>, seed: u64) -> NetAgent {
        NetAgent {
            net,
            rng: Rng::new(seed),
            temperature: 1.0,
            nudges: Nudges::default(),
            chores: true,
        }
    }

    /// The logits divided by `t` before sampling: 1 samples as training
    /// does, under 1 picks the network's favourites more often (owner,
    /// 2026-10-05: "it has a tendency to play cards for the sake of it" —
    /// a card it rates 30% is played 30% of the time).
    pub fn temperature(mut self, t: f32) -> NetAgent {
        assert!(t > 0.0, "a positive temperature");
        self.temperature = t;
        self
    }

    /// Whether the chores are never declined (`chores`; on by default, as
    /// in training).
    pub fn chores(mut self, on: bool) -> NetAgent {
        self.chores = on;
        self
    }

    /// The nudges added to the logits (before the temperature), as the
    /// training pool's do.
    pub fn nudges(mut self, n: Nudges) -> NetAgent {
        self.nudges = n;
        self
    }
}

impl Agent for NetAgent {
    fn decide(&mut self, engine: &Engine, req: &DecisionRequest) -> usize {
        // Referendum terms and votes by rule (`zvtes_env::politics`).
        if let Some(i) = politics::terms_choice(engine, req) {
            return i;
        }
        // Options score independently (each against the context): the
        // masked ones change nothing of the others' logits.
        let mask = mask(engine, req, self.chores);
        if let Some(m) = &mask {
            if let [only] = (0..m.len()).filter(|&i| m[i]).collect::<Vec<_>>()[..] {
                return only;
            }
        }
        let (mut logits, _) = self
            .net
            .forward(&obs::encode(engine, req, self.net.version));
        for (l, o) in logits.iter_mut().zip(&req.options) {
            *l += self.nudges.bias(engine, req, o);
            *l /= self.temperature;
        }
        let allowed = |i: usize| mask.as_ref().is_none_or(|m| m[i]);
        let max = (0..logits.len())
            .filter(|&i| allowed(i))
            .map(|i| logits[i])
            .fold(f32::MIN, f32::max);
        let weights: Vec<f64> = logits
            .iter()
            .enumerate()
            .map(|(i, &l)| {
                if allowed(i) {
                    ((l - max) as f64).exp()
                } else {
                    0.0
                }
            })
            .collect();
        let total: f64 = weights.iter().sum();
        // A uniform draw in [0, total), from 53 random bits.
        let mut u = ((self.rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64) * total;
        for (i, w) in weights.iter().enumerate() {
            if u < *w {
                return i;
            }
            u -= w;
        }
        (0..weights.len())
            .rev()
            .find(|&i| weights[i] > 0.0)
            .unwrap_or(0)
    }

    fn fork(&self) -> Box<dyn Agent> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use zvtes::decks::Deck;
    use zvtes::game::Step;
    use zvtes::ids::MinionId;

    /// A network of zeros (no layer): every option scores alike, so the
    /// agent draws uniformly among those its masks leave.
    pub(crate) fn uniform_net() -> Net {
        let d = 4;
        let shapes: [(&str, &[usize]); 20] = [
            ("tok_f.weight", &[d, obs::TOK_F]),
            ("tok_f.bias", &[d]),
            ("opt_f.weight", &[d, obs::OPT_F]),
            ("opt_f.bias", &[d]),
            ("norm.weight", &[d]),
            ("norm.bias", &[d]),
            ("score.0.weight", &[d, 2 * d]),
            ("score.0.bias", &[d]),
            ("score.2.weight", &[1, d]),
            ("score.2.bias", &[1]),
            ("value.0.weight", &[d, 2 * d]),
            ("value.0.bias", &[d]),
            ("value.2.weight", &[1, d]),
            ("value.2.bias", &[1]),
            ("opt_ptr.0.weight", &[d, d]),
            ("opt_ptr.1.weight", &[d, d]),
            ("opt_ptr.2.weight", &[d, d]),
            ("tok_ptr.weight", &[d, d]),
            ("opt_kind.weight", &[1, d]),
            ("token_type.weight", &[1, d]),
        ];
        let mut header =
            format!("zvp 1\nd {d}\nlayers 0\nheads 1\nobs 1\ntensor vocab.weight 0 {d}\n");
        let mut n = 0;
        for (name, shape) in shapes {
            let dims: Vec<String> = shape.iter().map(|x| x.to_string()).collect();
            header += &format!("tensor {name} {}\n", dims.join(" "));
            n += shape.iter().product::<usize>();
        }
        let mut bytes = header.into_bytes();
        bytes.push(b'\n');
        bytes.resize(bytes.len() + 4 * n, 0);
        Net::load(&bytes).expect("a zero network")
    }

    /// Owner, 2026-10-15 ("mask in the bot layer, train and serve"): on
    /// random games of the E25 engine, which offers futile block attempts,
    /// the network agent never makes one, and a window of futile attempts
    /// only is passed.
    #[test]
    fn the_network_agent_never_attempts_a_futile_block() {
        let net = std::sync::Arc::new(uniform_net());
        let (mut offered, mut only) = (0, 0);
        for seed in 0..12u64 {
            let decks: Vec<Deck> = (0..4)
                .map(|s| Deck::precon(((seed * 3 + s * 5) % 14) as usize))
                .collect();
            let mut e = Engine::with_decks(seed, &decks);
            e.time_limit = 40;
            let mut agents: Vec<NetAgent> = (0..4)
                .map(|s| NetAgent::new(net.clone(), seed * 10 + s))
                .collect();
            while let Step::Decision(req) = e.run() {
                let i = agents[req.seat.index()].decide(&e, &req);
                let futile = |o: &Choice| matches!(*o, Choice::DeclareBlock { minion } if !e.block_can_reach(req.seat, minion));
                if req.options.iter().any(futile) {
                    offered += 1;
                    assert!(
                        !futile(&req.options[i]),
                        "seed {seed}: a futile attempt taken"
                    );
                }
                if e.futile_blocks(&req) {
                    only += 1;
                    assert_eq!(
                        req.options[i],
                        Choice::Pass,
                        "seed {seed}: a futile window not passed"
                    );
                }
                e.choose(i);
                if e.decisions > 20_000 {
                    break;
                }
            }
        }
        assert!(offered > 0 && only > 0, "the games met no futile attempt");
    }

    /// `rush` names the options entering combat with their target: on
    /// random games of precons with rush cards (Nosferatu, Banu Haqim,
    /// Brujah, Gangrel), each rush declared is followed, before its action
    /// ends, by a combat of the acting minion with the target or a blocker.
    #[test]
    fn a_rush_is_an_action_entering_combat_with_its_target() {
        use zvtes::combat::CombatNote;
        use zvtes::notes::Note;
        let net = std::sync::Arc::new(uniform_net());
        let (mut rushes, mut with_target) = (0, 0);
        for seed in 0..200u64 {
            let decks: Vec<Deck> = [3, 7, 8, 9].map(Deck::precon).to_vec();
            let mut e = Engine::with_decks(seed, &decks);
            e.time_limit = 40;
            e.notes = Some(Vec::new());
            let mut agents: Vec<NetAgent> = (0..4)
                .map(|s| NetAgent::new(net.clone(), seed * 10 + s))
                .collect();
            // The rush under way: its action (turn, acting minion), its
            // target, the minion its actor fought.
            let mut open: Option<((u32, MinionId), MinionId, Option<MinionId>)> = None;
            loop {
                let step = e.run();
                for n in e.take_notes().unwrap_or_default() {
                    if let (Some((k, _, seen)), Note::Combat(CombatNote::Begins { minions })) =
                        (&mut open, n)
                    {
                        if minions.contains(&k.1) {
                            *seen = minions.into_iter().find(|&m| m != k.1);
                        }
                    }
                }
                let Step::Decision(req) = step else { break };
                let key = e.action().map(|a| (e.state.turn, a.acting));
                // A rush card's as-played window comes before its action
                // (a wake there, rules-qa 96.4): not its end.
                let before_action = req.window == WindowLabel::CardAsPlayed && key.is_none();
                if !before_action && open.is_some_and(|(k, _, _)| key != Some(k)) {
                    let (k, t, seen) = open.take().unwrap();
                    assert!(
                        seen.is_some(),
                        "seed {seed}: the rush of {k:?} on {t:?} fought no one"
                    );
                    with_target += (seen == Some(t)) as u32;
                }
                let i = agents[req.seat.index()].decide(&e, &req);
                if let Some(t) = zvtes::profile::rush(&e, req.seat, &req.options[i]) {
                    let actor = match req.options[i] {
                        Choice::Play { actor, .. } | Choice::UseAbility { actor, .. } => actor,
                        Choice::TakeAction { minion, .. } => Some(minion),
                        _ => None,
                    };
                    rushes += 1;
                    open = Some(((e.state.turn, actor.expect("an acting minion")), t, None));
                }
                e.choose(i);
                if e.decisions > 20_000 {
                    break;
                }
            }
        }
        assert!(
            rushes >= 20 && with_target > 0,
            "{rushes} rushes, {with_target} on their target"
        );
    }
}
