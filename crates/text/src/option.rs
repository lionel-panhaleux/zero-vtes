//! Options in English (docs/protocol.md §7): from the option's key and
//! effects, the window it is offered in and the names the client knows.
//! `None`: a kind not rendered yet (A7 ratchets kind by kind).

use zvtes_api::ids::{ActionKind, OptionKind as K, Purpose, Role, Window, Zone};
use zvtes_api::option::{EffectKind, Mode, On, OptionView};
use zvtes_api::view::Target;

use crate::effect::{label, parts};
use crate::event::{action_kind, strike};
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
    let base = base(o, window, seat, offered, n);
    match purpose {
        None => base,
        Some(p) => asked(o, p, base, offered, n),
    }
}

/// An option as it reads anywhere: by its kind and the window.
fn base(
    o: &OptionView,
    window: Window,
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
    Some(match k.kind {
        K::Pass if window == Window::ActionBlock => block_pass(seat, n).into(),
        // A cost's terms: the pass reduces nothing.
        K::Pass
            if window == Window::PlayTerms
                && offered
                    .iter()
                    .flat_map(|o| &o.effects)
                    .any(|e| e.cost || e.kind == EffectKind::Cost) =>
        {
            "No reduction".into()
        }
        K::Pass
            if matches!(
                window,
                Window::PlayTerms
                    | Window::RiderTerms
                    | Window::ActionTerms
                    | Window::ReferendumTerms
            ) =>
        {
            "Pass".into()
        }
        K::Pass => pass(window)?.into(),
        K::EdgePool => "Gain 1 pool (the Edge)".into(),
        K::ResolveRider => format!("Resolve {}", n.own(k.card.as_ref()?.copy?)),
        // A card, a title or a library card contested.
        K::PayContest | K::YieldContest => {
            let what = match (&k.card, k.minion) {
                (Some(c), _) => n.own(c.copy?),
                (None, Some(m)) if k.value.as_deref() == Some("title") => {
                    format!("the title of {}", n.minion_bare(m))
                }
                (None, Some(m)) => n.minion_bare(m),
                (None, None) => return None,
            };
            match k.kind {
                K::PayContest => format!("Pay to keep contesting {what}"),
                _ => format!("Yield {what}"),
            }
        }
        K::BurnInPlay => format!("Burn {}", n.copy_bare(k.card.as_ref()?.copy?)),
        K::Transfer => format!("Transfer 1 pool to {}", target()),
        K::Withdraw => format!("Move 1 blood from {} back to pool", target()),
        K::InfluenceOut => format!("Influence out {}", target()),
        K::Merge => format!("Merge {} (4 transfers and 1 pool)", target()),
        K::DrawCrypt => match (window, n.action()) {
            (Window::InfluenceTransfers, _) => "Draw a crypt card (4 transfers)".into(),
            // In an action: Enthrall's "burn 1 blood to draw 1 card from
            // your crypt".
            (_, Some(_)) => "Burn 1 blood to draw a crypt card".into(),
            _ => "Draw a crypt card (4 transfers)".into(),
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
        K::RoundStrike => match &k.card {
            Some(c) => format!("Strike at {}", n.own(c.copy?)),
            None => match o.effects.iter().find_map(|e| e.strike.as_ref()) {
                Some(s) => format!("Strike: {}", strike(Some(s))),
                None => "Strike: the round's strike".into(),
            },
        },
        K::EmployFromHand => format!("Employ {}{}", card(), mode(k.mode.as_ref()?)),
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
        K::SetX => amount(o, n)?,
        // A referendum's terms: what the chosen gets, when they choose
        // several of a kind.
        K::ChooseSeat | K::ChooseMinion if window == Window::ReferendumTerms => {
            let name = match k.kind {
                K::ChooseSeat => target(),
                _ => minion(k.minion),
            };
            match chosen_role(o) {
                Some(r) => format!("Choose {name} ({r})"),
                None => format!("Choose {name}"),
            }
        }
        K::AllocateTo if window == Window::Distribution => {
            format!("{} to your pool", k.amount?)
        }
        K::AllocateTo => allocate(o, n)?,
        K::CastVotes => {
            let mut s = format!("{}'s votes {}", minion(k.minion), votes(k.favor));
            let cast = |prisci: bool| {
                o.effects.iter().find(|e| {
                    e.kind == EffectKind::Votes
                        && (e.detail.as_deref() == Some("prisci_ballot")) == prisci
                })
            };
            if let Some(v) = cast(false) {
                s += &format!(" ({})", v.n?);
            }
            // A priscus casts its prisci ballot too (rules-qa 52.9).
            match cast(true).and_then(|b| b.n) {
                None => {}
                Some(1) => s += " and a prisci ballot",
                Some(b) => s += &format!(" and {b} prisci ballots"),
            }
            s
        }
        K::CastCardVote => {
            let v = o.effects.iter().find(|e| e.kind == EffectKind::Votes)?;
            match v.on {
                Some(On::Card) => format!("The political action card's vote {}", votes(k.favor)),
                // Business Pressure's round.
                _ => format!("The {} votes {}", v.n?, votes(k.favor)),
            }
        }
        K::ChooseMinion
            if window == Window::PlayTerms
                && o.effects
                    .iter()
                    .any(|e| e.cost && e.kind == EffectKind::Blood) =>
        {
            format!("Choose {} (pays 1 of the cost in blood)", minion(k.minion))
        }
        K::ChooseMinion if window == Window::Distribution => {
            format!("1 blood to {}", minion(k.minion))
        }
        K::ChooseMinion => format!("Choose {}", minion(k.minion)),
        K::ChooseSeat => format!("Choose {}", target()),
        K::ChooseCircle => format!("Choose the {} Circle", n.value(k.value.as_deref()?)),
        K::Slot => {
            let c = k.card.as_ref()?;
            let name = match c.copy {
                Some(copy) => n.copy_bare(copy),
                None => n.card_id(c.id),
            };
            format!("Choose {name}")
        }
        K::AshVampire => {
            let from = match k.zone.as_ref()?.zone {
                Zone::Crypt => "your crypt",
                _ => "the ash heap",
            };
            format!("Choose {} in {from}", card())
        }
        K::DrawStored => {
            let c = k.card.as_ref()?;
            format!("Draw {} from {}", n.card_id(c.id), n.own(c.from?))
        }
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
            // A second vampire chosen after (Toreador Grand Ball): what
            // happens to which (the target locked).
            let second = o.effects.iter().find(|e| e.kind == EffectKind::Role);
            let locks = o
                .effects
                .iter()
                .any(|e| e.kind == EffectKind::Lock && e.on == Some(On::Target));
            if k.target.is_some() {
                match second.is_some() && locks {
                    true => s += &format!(": lock {}", target()),
                    false => s += &format!(" on {}", target()),
                }
            } else if known && declares(o, ActionKind::Bleed) {
                // A bleed's target: the prey, unless a card says otherwise.
                s += &format!(" → {}", n.seat(n.prey(seat)?));
            }
            if let Some(r) = second {
                let clan = r.clan.as_deref().map_or("vampire".into(), |c| n.value(c));
                s += &match r.role? {
                    Role::UnblockableNonBleed => {
                        format!(
                            " (then choose the {clan} whose non-bleed actions cannot be blocked)"
                        )
                    }
                    _ => format!(" (then choose the card's chosen {clan})"),
                };
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

/// An option for the choice an effect asks (`purpose`): its base text,
/// said for that choice where it needs to be.
fn asked(
    o: &OptionView,
    p: Purpose,
    base: Option<String>,
    offered: &[OptionView],
    n: &Names,
) -> Option<String> {
    use Purpose as P;
    let k = &o.key;
    let role = |r: &str| Some(format!("{} ({r})", base.clone()?));
    let verb = |from: &str, to: &str| Some(base.clone()?.replacen(from, to, 1));
    let card = || n.card_id(k.card.as_ref().and_then(|c| c.id));
    let has = |kind| o.effects.iter().find(|e| e.kind == kind);
    // What the other options of the choice do (a pass's meaning).
    let any = |kind| {
        offered
            .iter()
            .flat_map(|o| &o.effects)
            .find(|e| e.kind == kind)
    };
    Some(match (p, k.kind) {
        (P::RemoveUncontrolled, K::Slot) => verb("Choose ", "Remove ")?,
        (P::RemoveFromAsh, K::FindCard) => verb("Take ", "Remove ")?,
        (P::RemoveFromAsh, K::AshVampire) => verb("Choose ", "Remove ")?,
        (P::MinionOf, K::ChooseMinion) => {
            let e = o.effects.first()?;
            role(&match e.kind {
                EffectKind::Lock => "locked".into(),
                EffectKind::Corruption => "gets a corruption counter".into(),
                EffectKind::Unlock => "unlocked".into(),
                EffectKind::Blood if e.n? < 0 => "burns 1 blood".into(),
                EffectKind::Blood => "gains 1 blood".into(),
                EffectKind::Move => format!("gets {}", n.copy_bare(e.copy?)),
                EffectKind::Burn => "burned".into(),
                EffectKind::Damage => format!("takes {} damage", e.n?),
                _ => return None,
            })?
        }
        (P::MinionOf, K::Pass) => match any(EffectKind::Lock).or(any(EffectKind::Unlock)) {
            Some(e) if e.kind == EffectKind::Lock && e.detail.is_none() => "Lock no more".into(),
            Some(e) if e.kind == EffectKind::Unlock => "Unlock none".into(),
            _ => "Pass".into(),
        },
        // Ashur Tablets: to the hand first, then into the library.
        (P::AshurPicks, K::FindCard) => match has(EffectKind::Move)?.to? {
            Zone::Hand => format!("{} to your hand", card()),
            _ => format!("Shuffle {} into your library", card()),
        },
        (P::AshurPicks, K::Pass) => match any(EffectKind::Move).and_then(|e| e.to) {
            Some(Zone::Hand) => "None to your hand".into(),
            _ => "No more".into(),
        },
        (P::ExchangeAnarch, K::Discard) => format!(
            "{} (for {})",
            verb("Discard ", "Exchange ")?,
            n.card_id(has(EffectKind::Move)?.card)
        ),
        (P::DistributeFrom, K::Slot) => role("gets 1 blood")?,
        (P::ChooseForCard, K::ChooseMinion) => {
            let e = has(EffectKind::Role)?;
            let clan = e.clan.as_deref().map_or("vampire".into(), |c| n.value(c));
            role(&match e.role? {
                Role::UnblockableNonBleed => "non-bleed actions cannot be blocked".into(),
                Role::DoesNotUnlock => "does not unlock as normal".into(),
                _ => format!("the card's chosen {clan}"),
            })?
        }
        (P::DistributeFrom, K::Pass) => "Move no more blood".into(),
        (P::BloodOrBurned, K::BurnBloodToAvoid) => {
            let e = has(EffectKind::Blood)?;
            format!("{} burns {} blood", n.minion_bare(e.copy?), -e.n?)
        }
        (P::BloodOrBurned, K::Pass) => {
            let e = has(EffectKind::Burn)?;
            let name = match e.card {
                Some(c) => n.card_id(Some(c)),
                None => n.minion_bare(e.copy?),
            };
            format!("Let {name} burn")
        }
        // Gilbert Duane: the chosen becomes Anarch, or stays; once chosen,
        // a pass gains 1 pool.
        (P::AnarchConvert, K::ChooseMinion) => role("becomes Anarch")?,
        (P::AnarchConvert, K::Pass) if has(EffectKind::Pool).is_some() => "Gain 1 pool".into(),
        (P::AnarchConvert, K::Pass) => "Keep him in play".into(),
        (P::CryptSearch, K::AshVampire) => format!(
            "Move {} from your crypt to your uncontrolled region",
            card()
        ),
        (P::CryptSearch | P::BaseSearch, K::Pass) => "Find none".into(),
        (P::ChooseCircle, K::Pass) => "Keep his own circle".into(),
        (P::StoreSearch | P::DisciplineSearch, K::Pass) => "Stop searching".into(),
        (P::BlockToll, K::Pass) => "Resign the attempt".into(),
        // What legacy says for the choice and is not told from data yet.
        (
            P::GiveBlood
            | P::PayOrBurn
            | P::LockOrDiscard
            | P::AllySearch
            | P::MayDrawExtra
            | P::Redraw
            | P::EntryBurn
            | P::EntryRemoveOrBurn
            | P::MayPutOn
            | P::MayAddCounter
            | P::MayTorture
            | P::MayGainPool
            | P::CancelAttempt
            | P::CorruptionOwner
            | P::AvoidDamage
            | P::DiscardFromHandOf
            | P::PoolForCard
            | P::LockForPool
            | P::StealForCorruption
            | P::MakeAnarch
            | P::BecomesAnarch
            | P::UncontrolledBlood
            | P::LockOrPay
            | P::BaseSearch
            | P::PreySlotToPool
            | P::MayRecycleCanceled
            | P::NameAshPair
            | P::AllyFromAsh
            | P::MoveForBlood
            | P::MoveCardToTitled,
            _,
        )
        | (P::MinionOf | P::RemoveUncontrolled | P::RemoveFromAsh | P::AshurPicks, _)
        | (P::MoveBetween, K::ChooseMinion | K::Pass | K::ChooseCard)
        | (P::ExchangeAnarch, K::Pass) => return None,
        // The rest reads as anywhere.
        _ => base?,
    })
}

/// What a chosen Methuselah or minion gets in a referendum's terms.
fn chosen_role(o: &OptionView) -> Option<String> {
    let e = o.effects.iter().find(|e| e.on == Some(On::Chosen))?;
    Some(match e.kind {
        EffectKind::Pool if e.detail.as_deref() == Some("allocated") => "gives the pool".into(),
        EffectKind::Pool if e.n? > 0 => format!("gains {} pool", e.n?),
        EffectKind::Pool => "burns pool".into(),
        EffectKind::Steal => "takes control of the card".into(),
        EffectKind::Move if e.to == Some(Zone::Removed) => "banished".into(),
        EffectKind::Move => "gets the card".into(),
        EffectKind::Role if e.role == Some(Role::Expelled) => {
            "no reactions, blocks or votes this turn".into()
        }
        EffectKind::Lock => "locked".into(),
        _ => return None,
    })
}

/// Points allocated to a Methuselah in a referendum's terms: what they do,
/// and who is allocated to next (the last gets the rest).
fn allocate(o: &OptionView, n: &Names) -> Option<String> {
    let k = &o.key;
    let a = k.amount?;
    let Some(Target::Seat { seat }) = k.target else {
        return None;
    };
    let pool = |chosen: bool| {
        o.effects
            .iter()
            .find(|e| e.kind == EffectKind::Pool && (e.on == Some(On::Chosen)) == chosen)
    };
    let role = match (pool(false), pool(true)) {
        (Some(_), Some(from)) => {
            let who = from
                .seat
                .map_or_else(|| "the chosen Methuselah".into(), |s| n.seat(s));
            format!(" (gains {a} pool from {who})")
        }
        (Some(_), None) => format!(" (burns {a} pool)"),
        _ => String::new(),
    };
    let next: Vec<String> = n
        .referendum()
        .map(|r| r.terms.pending.iter().skip(1).map(|&s| n.seat(s)).collect())
        .unwrap_or_default();
    let rest = match next.as_slice() {
        [] => String::new(),
        [last] => format!(" — the rest goes to {last}"),
        more => format!(" — then {}", more.join(", ")),
    };
    Some(format!("Allocate {a} to {}{role}{rest}", n.seat(seat)))
}

/// An amount chosen (`set_x`), by what it does.
fn amount(o: &OptionView, n: &Names) -> Option<String> {
    let a = o.key.amount?;
    let has = |k| o.effects.iter().find(|e| e.kind == k);
    // Business Pressure: pool burned for votes.
    if let Some(v) = has(EffectKind::Votes) {
        return Some(match a {
            0 => "Burn no pool".into(),
            _ => format!("Burn {a} pool for {} votes", v.n?),
        });
    }
    // Smiling Jack: cards, the rest in pool.
    if has(EffectKind::Discard).is_some() {
        return Some(format!("{a} card(s), the rest in pool"));
    }
    let Some(e) = o.effects.first() else {
        return Some(format!("X = {a}"));
    };
    if e.x {
        let unit = match e.kind {
            EffectKind::Blood => "blood",
            EffectKind::Pool => "pool",
            EffectKind::Counters => "counters",
            _ => return None,
        };
        return Some(format!("X = {a} ({a} {unit})"));
    }
    Some(match e.kind {
        EffectKind::Blood => match e.copy {
            Some(c) => format!("{a} blood from {}", n.minion_bare(c)),
            None => format!("{a} blood"),
        },
        EffectKind::Pool if e.detail.as_deref() == Some("from_blood") => {
            format!("{a} blood to your pool")
        }
        EffectKind::Pool => format!("Give {a} pool"),
        EffectKind::Counters => format!("{a} counter(s)"),
        EffectKind::Draw => format!("{a} card(s)"),
        _ => return None,
    })
}

/// A pass in the block window, short and explicit: the defender's pass
/// blocks only while its minion's attempt meets the stealth; anyone
/// else's is plain.
fn block_pass(seat: u8, n: &Names) -> &'static str {
    use zvtes_api::ids::ActionStep;
    match n.action() {
        Some(a) if a.seat != seat => match (a.step, a.blocker) {
            (ActionStep::Attempt, Some(b)) if n.controller(b) == Some(seat) => {
                if a.intercept.unwrap_or(0) >= a.stealth {
                    "Pass (blocking)"
                } else {
                    "Pass (no block)"
                }
            }
            (ActionStep::Attempt, _) => "Pass",
            _ => "Pass (no block)",
        },
        _ => "Pass",
    }
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
        // A distribution: what the pass does depends on more; in terms, on
        // the terms (a cost's: "No reduction").
        Window::Distribution
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
