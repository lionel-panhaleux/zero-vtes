//! Effects in English (docs/protocol.md §7): short parts joined ("1
//! transfer, burn it, gain 2 pool"), as option buttons read.

use zvtes_api::ids::{ActionKind, Range};
use zvtes_api::option::{Effect, EffectKind as K, On, When};

use crate::event::{action_kind, strike};
use crate::Names;

fn plural(n: i32, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// What effects do, joined; empty when none says.
pub fn label(effects: &[Effect], n: &Names) -> String {
    parts(effects, n).join(", ")
}

/// Each effect told, the ones a successful bleed brings as one part.
pub fn parts(effects: &[Effect], n: &Names) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < effects.len() {
        if effects[i].when == Some(When::BleedSuccessful) {
            let j = (i..effects.len())
                .find(|&j| effects[j].when != Some(When::BleedSuccessful))
                .unwrap_or(effects.len());
            let then: Vec<String> = effects[i..j].iter().filter_map(|e| part(e, n)).collect();
            out.push(format!("if the bleed is successful, {}", then.join(", ")));
            i = j;
            continue;
        }
        out.extend(part(&effects[i], n));
        i += 1;
    }
    // The card targeted and the card itself (War Ghoul's location burn).
    if let Some(j) = out.iter().position(|p| p == "burn the card") {
        if let Some(k) = out.iter().position(|p| p == "burn it") {
            out[j] = "burn it and itself".into();
            out.remove(k);
        }
    }
    out
}

/// One effect told; `None` for one an option does not tell.
pub fn part(e: &Effect, names: &Names) -> Option<String> {
    let n = e.n.unwrap_or(0);
    let a = n.abs();
    let d = e.detail.as_deref();
    let on = e.on;
    Some(match e.kind {
        K::Action => match d {
            Some("discard_phase") => "+1 discard phase action".into(),
            Some("master_phase") => "master phase action".into(),
            _ => match e.action {
                None | Some(ActionKind::Card | ActionKind::Ability | ActionKind::Unknown) => {
                    "action".into()
                }
                Some(k) => match action_kind(k) {
                    t if t.ends_with("action") => t.into(),
                    t => format!("{t} action"),
                },
            },
        },
        K::Transfers if n < 0 => plural(a, "transfer", "transfers"),
        K::Transfers => format!("+{}", plural(n, "transfer", "transfers")),
        K::Pool => match d {
            Some("from_counter") => "1 blood to pool".into(),
            Some("all_counters") => "all its blood to pool".into(),
            _ if e.cost && e.x => "pay X pool".into(),
            _ if e.cost => format!("pay {a} pool"),
            _ if e.when == Some(When::Successful) => format!("gain {n} pool if successful"),
            _ if n < 0 => format!("burn {a} pool"),
            _ => format!("gain {n} pool"),
        },
        K::Blood => match d {
            Some("to_pool") => format!("{a} blood to pool"),
            Some("from_pool") => format!("{n} pool to blood"),
            Some("from_host") => format!("give {n} blood"),
            Some("from_target") if e.x => "take X blood".into(),
            Some("from_target") => format!("take {n} blood"),
            Some("to_start") => "+blood".into(),
            Some("per_hostage") => "+1 blood per hostage".into(),
            Some("two_oldest") => "+1 blood to the two oldest".into(),
            Some("from_counter") => "1 blood to the vampire".into(),
            Some("ash_ally") => format!("gain {n} blood"),
            Some("ash_ally_unlock") => format!("gain {n} blood and unlock"),
            _ if e.cost => format!("pay {a} blood"),
            _ => match on {
                Some(On::Host) if n < 0 => format!("burn {a} blood"),
                Some(On::Host) => format!("add {n} blood"),
                Some(On::Target) if n < 0 => format!("the target burns {a} blood"),
                Some(On::Target) => format!("{n} blood to the target"),
                Some(On::Uncontrolled) => format!("{n} blood to uncontrolled"),
                Some(On::Card) => format!("{n} blood to the card"),
                _ => format!("+{n} blood"),
            },
        },
        K::Life => match (d, on) {
            (Some("ash_ally"), _) => format!("{n} life to the target"),
            (_, Some(On::Host)) if n < 0 => format!("burn {a} life"),
            (_, Some(On::Target)) if n < 0 => format!("target burns {a}"),
            _ => format!("+{n} life"),
        },
        K::Steal if d == Some("pool") => format!("take {n} pool"),
        K::Steal => format!("steal {n} blood"),
        K::Burn => match (on, d) {
            (Some(On::Card), _) => "burn it".into(),
            (_, Some("card")) => "burn the card".into(),
            _ => "burn the target".into(),
        },
        K::Stealth if d == Some("on_hunts") => format!("+{n} stealth on hunts"),
        K::Stealth if n < 0 => format!("-{a} stealth"),
        K::Stealth => format!("+{n} stealth"),
        K::Intercept => format!("+{n} intercept"),
        K::Strike if d == Some("commit") => "commit its strike".into(),
        K::Strike => format!("strike: {}", strike(e.strike.as_ref())),
        K::Damage if d == Some("weapon_strike") => {
            format!("a melee weapon's strike at +{n} damage")
        }
        K::Damage => format!("{n} damage"),
        K::Strength if d == Some("this_round") => format!("+{n} strength this round"),
        K::Strength => format!("+{n} strength"),
        K::Prevent => match d {
            Some("guns") => format!("prevent {n} (guns)"),
            Some("not_guns") => format!("prevent {n} (not guns)"),
            Some("non_aggravated") => format!("prevent {n} non-agg."),
            Some("strike") => format!("prevent {n} (strike)"),
            Some("strike_all") => "prevent all (strike)".into(),
            Some("strike_non_aggravated") => format!("prevent {n} non-agg. (strike)"),
            _ => format!("prevent {n} damage"),
        },
        K::Immune if d == Some("strike") => "immune: strike".into(),
        K::Immune => "immune: effects".into(),
        K::Press if d == Some("continue") => "press to continue".into(),
        K::Press => "a press".into(),
        K::Maneuver => "a maneuver".into(),
        K::Range => match e.range {
            Some(Range::Long) => "long range".into(),
            _ => "close range".into(),
        },
        K::EndCombat => "end combat".into(),
        K::EnterCombat => "enter combat".into(),
        K::Unlock => match (d, on) {
            (Some("burn_blood"), _) => "burn 1 blood: unlock".into(),
            (_, Some(On::Target)) => "unlock the target".into(),
            (_, Some(On::Acting)) if e.when == Some(When::Successful) => {
                "unlock if successful".into()
            }
            (_, Some(On::Acting)) => "unlock the actor".into(),
            _ => "unlock".into(),
        },
        K::Lock => match on {
            Some(On::Prey) => format!("lock {n} prey vampires"),
            Some(On::Host) => "lock the vampire".into(),
            _ => "lock the target".into(),
        },
        K::Fails if d == Some("referendum") => "referendum fails".into(),
        K::Fails => "the action fails".into(),
        K::Cancel => "cancel the card".into(),
        K::Move => match d {
            Some("leaves_torpor") => "the target leaves torpor".into(),
            Some("give") => "give to prey".into(),
            Some("from_ash_heap") => "take from prey's ash heap".into(),
            Some("political_from_ash_heap") => "a political action card to hand".into(),
            _ => format!("fetch {}", names.card_id(e.card)),
        },
        K::Votes => match d {
            Some("non_camarilla") => format!("non-Camarilla -{a} vote"),
            Some("per_counter") => "+1 vote per counter".into(),
            _ => {
                let side = |f: bool| if f { "for" } else { "against" };
                match (e.x, e.favor) {
                    (true, Some(f)) => format!("+{n}X votes {}", side(f)),
                    (_, Some(f)) => format!("+{} {}", plural(n, "vote", "votes"), side(f)),
                    _ => format!("+{}", plural(n, "vote", "votes")),
                }
            }
        },
        K::Referendum => "call a referendum".into(),
        K::Hunt => "hunt".into(),
        K::HandSize => format!("+{n} hand size this turn"),
        K::Search => "search for equipment".into(),
        K::Draw => match d {
            Some("then_bottom") => "draw, then bottom a card".into(),
            Some("crypt_then_remove") => "crypt draw, then remove one".into(),
            _ => format!("draw {n}, discard down"),
        },
        K::Bleed if e.x => "-X bleed".into(),
        K::Bleed => format!("+{n} bleed"),
        K::Counters => match (d, on) {
            (Some("bank"), _) => format!("bank {n} pool"),
            (Some("from_pool"), _) => format!("{n} pool to the card"),
            _ if e.cost && e.x => "pay with X counters".into(),
            _ if e.x => "burn X counters".into(),
            (_, Some(On::Target)) => "counter on target".into(),
            (_, Some(On::Uncontrolled)) => "counters to uncontrolled".into(),
            _ if n < 0 => format!("burn {}", plural(a, "counter", "counters")),
            _ => format!("add {}", plural(n, "counter", "counters")),
        },
        K::Cost if d == Some("blood") => format!("-{a} blood cost"),
        K::Cost => format!("-{a} cost"),
        _ => return None,
    })
}
