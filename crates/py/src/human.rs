//! A game record (the server's, or the corpus's: `data/records`) read
//! back as samples: each
//! decision's encodings for the networks, what was offered and what was
//! chosen, and what happened in between (DESIGN §11.2, human games as
//! data; `train/assess.py` reads them).

use numpy::{PyArray1, PyArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use zvtes::crypt::CRYPT;
use zvtes::library::LIBRARY;
use zvtes::obs::{encode, encode_critic, Obs, OPT_F, OPT_IDS, TOK_F, TOK_IDS};
use zvtes_api::record::GameRecord;
use zvtes_host::replay::{Read, Replayer, Step};

use crate::{all_decks, version};

/// An observation's arrays into `d` under `prefix`: `tok_ids` [T, TOK_IDS],
/// `tok_f` [T, TOK_F], `deck_ids` [D, TOK_IDS], `deck_f` [D, TOK_F] (`V3`'s
/// deck rows, none before) and, for a policy's, `opt_ids`, `opt_f`.
fn put(py: Python<'_>, d: &Bound<'_, PyDict>, prefix: &str, o: Obs, options: bool) -> PyResult<()> {
    let k = o.deck_ids.len();
    let ids: Vec<i32> = o.deck_ids.into_iter().flatten().collect();
    let f: Vec<f32> = o.deck_f.into_iter().flatten().collect();
    d.set_item(
        format!("{prefix}deck_ids"),
        PyArray1::from_vec(py, ids).reshape([k, TOK_IDS])?,
    )?;
    d.set_item(
        format!("{prefix}deck_f"),
        PyArray1::from_vec(py, f).reshape([k, TOK_F])?,
    )?;
    let t = o.tok_ids.len();
    let ids: Vec<i32> = o.tok_ids.into_iter().flatten().collect();
    let f: Vec<f32> = o.tok_f.into_iter().flatten().collect();
    d.set_item(
        format!("{prefix}tok_ids"),
        PyArray1::from_vec(py, ids).reshape([t, TOK_IDS])?,
    )?;
    d.set_item(
        format!("{prefix}tok_f"),
        PyArray1::from_vec(py, f).reshape([t, TOK_F])?,
    )?;
    if options {
        let n = o.opt_ids.len();
        let ids: Vec<i32> = o.opt_ids.into_iter().flatten().collect();
        let f: Vec<f32> = o.opt_f.into_iter().flatten().collect();
        d.set_item(
            format!("{prefix}opt_ids"),
            PyArray1::from_vec(py, ids).reshape([n, OPT_IDS])?,
        )?;
        d.set_item(
            format!("{prefix}opt_f"),
            PyArray1::from_vec(py, f).reshape([n, OPT_F])?,
        )?;
    }
    Ok(())
}

/// Read `record` (a game record's JSON: the server's, or the corpus's):
/// one dict per decision, in order — `decision`, `seat`, `turn`,
/// `window` (its id), `options` (told), `cards` (the KRCG id each option
/// names, else None), `chosen` (None for the decision the record ends on),
/// `choice` (its text), `entry` (with `chosen`: the index in the record's
/// `entries` of the decision entry it took), `before` (what happened since the previous
/// decision), the deciding seat's policy encoding (`tok_ids`, `tok_f`,
/// `opt_ids`, `opt_f`) and critic encoding (`critic_tok_ids`,
/// `critic_tok_f`), both in observation version `obs`, and `position`
/// (`position_names`' features for the deciding seat). The dict `over`
/// (`vps`, `before`) ends a finished game; `stopped` (why) a game whose
/// record stops reading (a rule change since: the decisions up to it, the
/// last unanswered). Also `names` (seats') and `humans` (the seats a human
/// played).
#[pyfunction]
#[pyo3(signature = (record, obs=1))]
pub fn replay_decisions<'py>(
    py: Python<'py>,
    record: &str,
    obs: u8,
) -> PyResult<Bound<'py, PyDict>> {
    let v = version(obs)?;
    let record: GameRecord =
        serde_json::from_str(record).map_err(|e| PyValueError::new_err(format!("record: {e}")))?;
    let mut r = Replayer::new(&record).map_err(PyValueError::new_err)?;
    let mut samples: Vec<Bound<'py, PyDict>> = Vec::new();
    let out = PyDict::new(py);
    let read = |py: Python<'py>, r: &Replayer, d: &Read| -> PyResult<Bound<'py, PyDict>> {
        let x = PyDict::new(py);
        x.set_item("decision", d.decision)?;
        x.set_item("seat", d.seat)?;
        x.set_item("turn", d.turn)?;
        let window = serde_json::to_value(d.window).expect("ids serialize");
        x.set_item("window", window.as_str().unwrap_or("unknown"))?;
        x.set_item("before", d.before.clone())?;
        x.set_item("options", d.options.clone())?;
        x.set_item("cards", d.cards.clone())?;
        let req = r.request().expect("a decision is pending");
        put(py, &x, "", encode(r.engine(), req, v), true)?;
        put(py, &x, "critic_", encode_critic(r.engine(), req, v), false)?;
        x.set_item(
            "position",
            crate::position::features(&r.engine().state, req.seat),
        )?;
        Ok(x)
    };
    loop {
        match r.step() {
            Step::Decision(d) => {
                let x = read(py, &r, &d)?;
                match d.chosen {
                    None => {
                        x.set_item("chosen", py.None())?;
                        samples.push(x);
                        break;
                    }
                    Some(i) => {
                        x.set_item("chosen", i)?;
                        x.set_item("entry", d.entry)?;
                        x.set_item("choice", d.options.get(i).cloned())?;
                        samples.push(x);
                        r.take();
                    }
                }
            }
            // The engine changed what the game asks: the game so far, and
            // why it stops.
            Step::Stopped { why, last } => {
                let x = read(py, &r, &last)?;
                x.set_item("chosen", py.None())?;
                samples.push(x);
                out.set_item("stopped", why)?;
                break;
            }
            Step::Over { vp_halves, before } => {
                let o = PyDict::new(py);
                o.set_item(
                    "vps",
                    vp_halves
                        .iter()
                        .map(|&h| h as f32 / 2.0)
                        .collect::<Vec<_>>(),
                )?;
                o.set_item("before", before)?;
                out.set_item("over", o)?;
                break;
            }
        }
    }
    out.set_item("decisions", samples)?;
    out.set_item("names", r.names().to_vec())?;
    out.set_item(
        "humans",
        r.humans().iter().map(|&s| s as usize).collect::<Vec<_>>(),
    )?;
    Ok(out)
}

/// Deck `d`'s cards (`deck_names` order), crypt then library: (KRCG id,
/// copies) — what a network trained on that deck has seen, or a seat's
/// deck in a replay's spec.
#[pyfunction]
pub fn deck_cards(d: usize) -> PyResult<Vec<(u32, u8)>> {
    let deck = all_decks()
        .get(d)
        .ok_or_else(|| PyValueError::new_err(format!("no deck {d}")))?;
    let crypt = deck.crypt.iter().map(|&(i, n)| (CRYPT[i as usize].id, n));
    let library = deck
        .library
        .iter()
        .map(|&(i, n)| (LIBRARY[i as usize].id, n));
    Ok(crypt.chain(library).collect())
}
