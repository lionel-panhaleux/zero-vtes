//! Options in English (docs/protocol.md §7): from the option's key and
//! effects, the window it is offered in and the names the client knows.
//! `None`: a kind not rendered yet (A7 ratchets kind by kind).

use zvtes_api::ids::{ActionKind, OptionKind as K, Purpose, Window};
use zvtes_api::option::{EffectKind, Mode, OptionView};
use zvtes_api::view::Target;

use crate::effect::{label, parts};
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
            let known = k.card.as_ref().is_some_and(|c| c.id.is_some());
            let mut s = format!("Play {}", card());
            if let Some(m) = k.mode.as_ref().filter(|_| known) {
                s += &mode(m);
                if let Some(l) = split(o, offered, n) {
                    s += &format!(" ({l})");
                }
            }
            if let Some(a) = k.actor {
                s += &format!(" with {}", n.minion_bare(a));
            }
            if k.target.is_some() {
                s += &format!(" on {}", target());
            } else if known && declares(o, ActionKind::Bleed) {
                // A bleed's target: the prey, unless a card says otherwise.
                s += &format!(" → {}", n.seat(n.prey(seat)?));
            }
            s
        }
        K::UseAbility => {
            let (name, host) = match (&k.card, k.minion) {
                (Some(c), _) => {
                    let copy = c.copy?;
                    (n.copy_bare(copy), n.host(copy))
                }
                // A vampire's own: its actor named even when itself.
                (None, Some(m)) => (n.minion_bare(m), None),
                (None, None) => return None,
            };
            let mut s = format!("Use {name}");
            if let Some(a) = k.actor.filter(|&a| Some(a) != host) {
                s += &format!(" with {}", n.minion_bare(a));
            }
            if k.target.is_some() {
                s += &format!(" on {}", target());
            }
            s += &ability_what(o, offered, n)?;
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

/// A play's mode as its printed line's tags read: " at superior [DOM]",
/// " at [cel][COMBAT]" (a card of several types: the type it is played
/// as, in capitals).
fn mode(m: &Mode) -> String {
    let mut tags: Vec<String> = m
        .disciplines
        .iter()
        .map(|d| match d.superior {
            true => d.discipline.to_uppercase(),
            false => d.discipline.clone(),
        })
        .collect();
    if let Some(t) = &m.card_type {
        tags.push(match t.as_str() {
            "action_modifier" => "ACTIONMODIFIER".into(),
            "political_action" => "POLITICAL".into(),
            t => t.to_uppercase(),
        });
    }
    if tags.is_empty() {
        return String::new();
    }
    let upper = |t: &String| !t.chars().any(|c| c.is_lowercase());
    let level = if tags.iter().all(upper) {
        "superior "
    } else if !tags.iter().any(upper) {
        "inferior "
    } else {
        ""
    };
    format!(" at {level}[{}]", tags.join("]["))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// The option declares an action of this kind.
fn declares(o: &OptionView, kind: ActionKind) -> bool {
    o.effects
        .iter()
        .any(|e| e.kind == EffectKind::Action && e.detail.is_none() && e.action == Some(kind))
}

/// What sets a play's mode apart from the other modes of its printed
/// line offered with it ("gain 3 blood" against "gain 2 blood and
/// unlock"); `None` when none is offered or nothing does.
fn split(o: &OptionView, offered: &[OptionView], n: &Names) -> Option<String> {
    let (m, id) = (o.key.mode.as_ref()?, o.key.card.as_ref()?.id);
    let siblings: Vec<&OptionView> = offered
        .iter()
        .filter(|s| s.key.kind == K::Play && s.key.card.as_ref().map(|c| c.id) == Some(id))
        .filter(|s| {
            s.key
                .mode
                .as_ref()
                .is_some_and(|sm| sm.line == m.line && sm.index != m.index)
        })
        .collect();
    if siblings.is_empty() {
        return None;
    }
    let own = parts(&o.effects, n);
    let apart: Vec<String> = own
        .into_iter()
        .filter(|p| siblings.iter().any(|s| !parts(&s.effects, n).contains(p)))
        .collect();
    (!apart.is_empty()).then(|| apart.join(", "))
}

/// What an ability does, after who and on what: blood moved to or from
/// the pool; which of the source's abilities (what sets it apart from
/// the others offered); a lone ability's action.
fn ability_what(o: &OptionView, offered: &[OptionView], n: &Names) -> Option<String> {
    let moved = o.effects.iter().find(|e| {
        e.kind == EffectKind::Blood
            && matches!(e.detail.as_deref(), Some("to_pool" | "from_pool"))
            && e.on == Some(zvtes_api::option::On::Host)
    });
    if let Some(e) = moved {
        let v = e.n.unwrap_or(0);
        return Some(if v < 0 {
            format!(": {} blood to pool", -v)
        } else {
            format!(": {v} pool to blood")
        });
    }
    let ability = o.key.ability.unwrap_or(0);
    let same = |s: &OptionView| {
        s.key.kind == K::UseAbility
            && s.key.card.as_ref().map(|c| c.copy) == o.key.card.as_ref().map(|c| c.copy)
            && s.key.minion == o.key.minion
    };
    // Each ability of the source offered: its parts.
    let mut abilities: Vec<(u8, Vec<String>)> = Vec::new();
    for s in offered.iter().filter(|s| same(s)) {
        let i = s.key.ability.unwrap_or(0);
        if !abilities.iter().any(|(j, _)| *j == i) {
            abilities.push((i, parts(&s.effects, n)));
        }
    }
    let several = o.abilities.unwrap_or(1) > 1;
    if several || abilities.len() > 1 || ability > 0 {
        // Offered alone among several: what it does, whole.
        if abilities.len() < 2 {
            let l = label(&o.effects, n);
            return Some(match l.is_empty() {
                true => format!(" (ability {})", ability + 1),
                false => format!(": {l}"),
            });
        }
        let told = |own: &Vec<String>| {
            let apart: Vec<&String> = own
                .iter()
                .filter(|p| !abilities.iter().all(|(_, o)| o.contains(p)))
                .collect();
            match apart.is_empty() {
                true => own.join(", "),
                false => apart
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            }
        };
        let own = abilities.iter().find(|(i, _)| *i == ability)?.1.clone();
        let l = told(&own);
        let clash = abilities
            .iter()
            .any(|(i, p)| *i != ability && *p != own && told(p) == l);
        return Some(if l.is_empty() || clash {
            format!(" (ability {})", ability + 1)
        } else {
            format!(": {l}")
        });
    }
    // A lone ability that is an action: which.
    let action = o
        .effects
        .iter()
        .any(|e| e.kind == EffectKind::Action && e.detail.is_none());
    let l = label(&o.effects, n);
    Some(if action && !l.is_empty() {
        format!(": {l}")
    } else {
        String::new()
    })
}
