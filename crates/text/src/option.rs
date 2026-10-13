//! Options in English (docs/protocol.md §7): from the option's key and
//! effects, the window it is offered in and the names the client knows.
//! `None`: a kind not rendered yet (A7 ratchets kind by kind).

use zvtes_api::ids::{OptionKind as K, Purpose, Window};
use zvtes_api::option::{EffectKind, Mode, OptionView};
use zvtes_api::view::Target;

use crate::event::action_kind;
use crate::Names;

/// An option offered at `window`, for `purpose` (among `offered`), to
/// `seat`.
#[allow(clippy::too_many_arguments)]
pub fn option(
    o: &OptionView,
    window: Window,
    purpose: Option<Purpose>,
    seat: u8,
    offered: &[OptionView],
    n: &Names,
) -> Option<String> {
    let k = &o.key;
    let card = || {
        k.card
            .as_ref()
            .map_or_else(|| "a card".into(), |c| n.card_id(c.id))
    };
    let minion = |c: Option<u32>| n.minion_bare(c.unwrap_or(u32::MAX));
    let target = || match k.target {
        Some(Target::Seat { seat }) => n.seat(seat),
        Some(Target::Card { copy }) => n.copy_bare(copy),
        None => String::new(),
    };
    let votes = |f: Option<bool>| if f == Some(true) { "for" } else { "against" };
    let _ = offered;
    // What an effect asks a choice for rewrites its options: not told yet.
    if purpose.is_some() {
        return None;
    }
    Some(match k.kind {
        K::Pass => pass(window)?.into(),
        K::EdgePool => "Gain 1 pool (the Edge)".into(),
        K::Transfer => format!("Transfer 1 pool to {}", target()),
        K::Withdraw => format!("Move 1 blood from {} back to pool", target()),
        K::InfluenceOut => format!("Influence out {}", target()),
        K::Merge => format!("Merge {} (4 transfers and 1 pool)", target()),
        K::DrawCrypt => match window {
            Window::InfluenceTransfers => "Draw a crypt card (4 transfers)".into(),
            _ => return None,
        },
        K::Discard => format!("Discard {}", card()),
        K::Bottom => format!("Put {} on the bottom of the library", card()),
        K::FindCard => format!("Take {}", card()),
        K::DeclareBlock => format!("Block with {}", minion(k.minion)),
        K::TakeAction => {
            let mut s = format!(
                "{} with {}",
                capitalize(action_kind(k.action?)),
                minion(k.actor)
            );
            if let Some(Target::Card { copy }) = k.target {
                s += &format!(" ({})", n.minion_bare(copy));
            }
            s
        }
        K::ChooseClan => format!("Choose {}", n.value(k.value.as_deref()?)),
        K::BurnPool => "Burn 1 pool".into(),
        K::BurnBloodFrom => format!("Burn 1 blood from {}", minion(k.minion)),
        K::BurnForVote => format!("Burn {} for 1 vote {}", card(), votes(k.favor)),
        K::CastEdge => format!("Burn the Edge: 1 vote {}", votes(k.favor)),
        K::HandStrike => "Strike: hand strike".into(),
        K::UseManeuver => "Use a maneuver".into(),
        K::UsePress => "Use a press".into(),
        K::UsePrevent => "Prevent 1 damage".into(),
        K::PayBlockToll => "Pay the block toll".into(),
        K::PayToCancel => "Pay to cancel the card".into(),
        K::StrikeCombatEnds => "Strike: combat ends".into(),
        K::TakeDamageAfterResolution => "Take the damage".into(),
        K::CommittedStrike => "Strike: the committed strike".into(),
        K::DrawLibrary => "Draw from the library".into(),
        K::ResolvePrinted => format!("Resolve {}'s effect", minion(k.minion)),
        K::ContinueAction => "Continue the action as if unblocked".into(),
        K::GainStrength => "Gain the strength".into(),
        K::BurnBloodToAvoid => "Burn blood to avoid the damage".into(),
        K::Diablerise => "Diablerize the acting vampire".into(),
        K::UnlockInfernal => format!("Burn 1 pool to unlock {}", minion(k.minion)),
        K::UnlockForBlood => format!("{} burns blood to unlock", minion(k.minion)),
        K::BurnBloodForIntercept => {
            format!("{} burns 1 blood for +1 intercept", minion(k.minion))
        }
        K::PayRescue => format!("The rescuer pays {} blood", k.amount?),
        // A second vampire chosen after (Toreador Grand Ball): not told yet.
        K::Play if o.effects.iter().any(|e| e.kind == EffectKind::Role) => return None,
        K::Play => {
            let mut s = format!("Play {}", card());
            if let Some(m) = &k.mode {
                s += &mode(m);
            }
            if let Some(a) = k.actor {
                s += &format!(" with {}", n.minion_bare(a));
            }
            match k.target {
                Some(_) => s += &format!(" on {}", target()),
                None => return None,
            }
            let _ = seat;
            s
        }
        _ => return None,
    })
}

/// A pass, by its window; `None` where what it does depends on more.
fn pass(w: Window) -> Option<&'static str> {
    Some(match w {
        Window::MasterPhase => "End the master phase",
        Window::MinionPhase => "End the minion phase",
        Window::InfluenceTransfers => "End the influence phase",
        Window::DiscardPhase => "End the discard phase",
        Window::CombatPress => "Pass (no press)",
        Window::ReferendumPolling | Window::BloodHuntPolling => "Pass (no vote now)",
        // The block, a distribution: what the pass does depends on more;
        // in terms, on the terms (a cost's: "No reduction").
        Window::ActionBlock
        | Window::Distribution
        | Window::PlayTerms
        | Window::RiderTerms
        | Window::ActionTerms
        | Window::ReferendumTerms => return None,
        _ => "Pass",
    })
}

/// A play's mode: " at superior [DOM]".
fn mode(m: &Mode) -> String {
    if m.disciplines.is_empty() {
        return String::new();
    }
    let level = if m.disciplines.iter().all(|d| d.superior) {
        "superior "
    } else if m.disciplines.iter().all(|d| !d.superior) {
        "inferior "
    } else {
        ""
    };
    let tags: Vec<String> = m
        .disciplines
        .iter()
        .map(|d| {
            if d.superior {
                d.discipline.to_uppercase()
            } else {
                d.discipline.clone()
            }
        })
        .collect();
    format!(" at {level}[{}]", tags.join("]["))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}
