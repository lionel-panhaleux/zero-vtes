//! A game exported from the Succubus Club fork ("Export replay": the
//! spec by KRCG ids and the decision log) read back as samples: each
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
use zvtes_protocol::describe::OptionView;
use zvtes_protocol::{Out, Player, Replay, Session};

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

/// The KRCG id of the card an option names, if any.
fn card_of(o: &OptionView) -> Option<u32> {
    o.card.as_ref().and_then(|c| c.id)
}

/// Replay `replay` (the fork's JSON): one dict per decision, in order —
/// `decision`, `seat`, `turn`, `window`, `options` (texts), `cards` (the
/// KRCG id each option names, else None), `chosen` (None for the decision
/// the log ends on), `choice` (its text), `before` (what happened since
/// the previous decision), the deciding seat's policy encoding (`tok_ids`,
/// `tok_f`, `opt_ids`, `opt_f`) and critic encoding (`critic_tok_ids`,
/// `critic_tok_f`), both in observation version `obs`. The dict `over`
/// (`vps`, `before`) ends a finished game; `stopped` (why) a game whose
/// log stops replaying (a rule change since: the decisions up to it,
/// the last unanswered). Also `names` (seats') and `humans` (the seats a
/// human played).
#[pyfunction]
#[pyo3(signature = (replay, obs=1))]
pub fn replay_decisions<'py>(
    py: Python<'py>,
    replay: &str,
    obs: u8,
) -> PyResult<Bound<'py, PyDict>> {
    let v = version(obs)?;
    let replay: Replay =
        serde_json::from_str(replay).map_err(|e| PyValueError::new_err(format!("replay: {e}")))?;
    let mut spec = replay.spec.clone();
    let humans: Vec<usize> = (0..spec.seats.len())
        .filter(|&s| spec.seats[s].player == Player::Human)
        .collect();
    let names: Vec<String> = spec.seats.iter().map(|s| s.name.clone()).collect();
    // Every seat answered from the log: the deal depends on the seed alone.
    for s in &mut spec.seats {
        s.player = Player::Human;
    }
    let mut session = Session::new(spec).map_err(|e| PyValueError::new_err(format!("{e:?}")))?;
    let mut reader = replay.reader();
    session.show_all();
    let mut samples: Vec<Bound<'py, PyDict>> = Vec::new();
    let out = PyDict::new(py);
    loop {
        match session.advance() {
            Out::Decision {
                decision,
                seat,
                window,
                options,
                events,
                view,
                ..
            } => {
                let d = PyDict::new(py);
                d.set_item("decision", decision)?;
                d.set_item("seat", seat)?;
                d.set_item("turn", view.turn)?;
                d.set_item("window", window)?;
                d.set_item("before", events)?;
                d.set_item(
                    "options",
                    options.iter().map(|o| o.text.clone()).collect::<Vec<_>>(),
                )?;
                d.set_item("cards", options.iter().map(card_of).collect::<Vec<_>>())?;
                let req = session.request().expect("a decision is pending").clone();
                put(py, &d, "", encode(session.engine(), &req, v), true)?;
                put(
                    py,
                    &d,
                    "critic_",
                    encode_critic(session.engine(), &req, v),
                    false,
                )?;
                let index = match reader.next(session.engine(), &req) {
                    None => {
                        d.set_item("chosen", py.None())?;
                        samples.push(d);
                        break;
                    }
                    // The engine changed what the game asks: the game so
                    // far, and why it stops.
                    Some(Err(e)) => {
                        d.set_item("chosen", py.None())?;
                        samples.push(d);
                        out.set_item(
                            "stopped",
                            format!("decision {decision}: the log's choice is refused: {e}"),
                        )?;
                        break;
                    }
                    Some(Ok(i)) => i,
                };
                d.set_item("chosen", index)?;
                match session.answer(decision, index) {
                    Ok(Out::Decided { choice, .. }) => d.set_item("choice", choice.text)?,
                    Ok(_) => unreachable!("an answer is decided"),
                    Err(e) => {
                        return Err(PyValueError::new_err(format!(
                            "decision {decision}: the log's option {index} is refused: {e:?}"
                        )))
                    }
                }
                samples.push(d);
            }
            Out::Decided { .. } => unreachable!("every seat is answered from the log"),
            Out::Over { vps, events, .. } => {
                let o = PyDict::new(py);
                o.set_item("vps", vps)?;
                o.set_item("before", events)?;
                out.set_item("over", o)?;
                break;
            }
        }
    }
    out.set_item("decisions", samples)?;
    out.set_item("names", names)?;
    out.set_item("humans", humans)?;
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
