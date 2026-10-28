//! Options in English (docs/protocol.md §7): from the option's key and
//! effects, the window it is offered in and the names the client knows.
//! `None`: a kind not rendered, or an option missing what its text needs.

use zvtes_api::ids::{ActionKind, OptionKind as K, Purpose, Role, Window, Zone};
use zvtes_api::message::About;
use zvtes_api::option::{Effect, EffectKind, Mode, On, OptionView};
use zvtes_api::view::Target;

use crate::effect::label;
use crate::event::{action_kind, strike};
use crate::Names;

/// An option offered at `window`, for `purpose` (with its particulars,
/// `about`; among `offered`), to `seat`.
#[allow(clippy::too_many_arguments)]
pub fn option(
    o: &OptionView,
    window: Window,
    purpose: Option<Purpose>,
    about: Option<&About>,
    seat: u8,
    offered: &[OptionView],
    n: &Names,
) -> Option<String> {
    if o.key.kind == K::Pass {
        return pass(o, window, purpose, about, n);
    }
    let base = base(o, window, seat, offered, n);
    match purpose {
        None => base,
        Some(p) => asked(o, p, about, base, n),
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
    // A face down uncontrolled vampire: by its place.
    let back = || {
        let r = k.slot?;
        Some(match r.seat == seat {
            true => format!("uncontrolled vampire #{}", r.slot + 1),
            false => format!("{}'s uncontrolled vampire #{}", n.seat(r.seat), r.slot + 1),
        })
    };
    let target = || match k.target {
        Some(Target::Seat { seat }) => n.seat(seat),
        // A copy never shown: another seat's face down vampire.
        Some(Target::Card { copy }) if !n.knows(copy) => "an uncontrolled vampire".into(),
        Some(Target::Card { copy }) => n.copy_bare(copy),
        None => back().unwrap_or_default(),
    };
    let votes = |f: Option<bool>| if f == Some(true) { "for" } else { "against" };
    // A card in play: as the table names it (`own`: without the minion
    // it is on), else by its own name (a copy the viewer does not number).
    let in_play = |own: bool| {
        let c = k.card.as_ref()?;
        Some(match c.copy {
            Some(copy) if n.knows(copy) && own => n.own(copy),
            Some(copy) if n.knows(copy) => n.copy_bare(copy),
            _ => n.card_id(c.id),
        })
    };
    Some(match k.kind {
        K::EdgePool => "Gain 1 pool (the Edge)".into(),
        K::WithdrawFromGame => "Announce a withdrawal from the game".into(),
        K::FollowPath => format!(
            "Burn 1 blood: {} follows the Path of {}",
            minion(k.minion),
            path_name(k.value.as_deref()?)
        ),
        K::TakeFrom => {
            let from = match k.zone.as_ref()?.zone {
                Zone::Library => "the library",
                Zone::Hand => "the hand",
                _ => "the ash heap",
            };
            format!("Take {} from {from}", card())
        }
        K::TakeEquipment => format!("Take {}", in_play(true)?),
        K::ChooseDiscipline => format!("Choose [{}]", k.value.as_deref()?),
        K::AllocateToCard => {
            let Some(Target::Card { copy }) = k.target else {
                return None;
            };
            let burned = o.effects.iter().any(|e| e.kind == EffectKind::Burn);
            format!(
                "Allocate {} to {}{}",
                k.amount?,
                n.own(copy),
                if burned { " (burned)" } else { "" }
            )
        }
        K::BurnWeaponStrike => format!("Strike: burn {}", in_play(true)?),
        K::BurnCorruptionToFail => format!(
            "Burn a corruption counter from {}: its block fails",
            minion(k.minion)
        ),
        K::BurnCorruptionToUnlock => format!(
            "Burn 2 corruption counters from {} to unlock",
            minion(k.minion)
        ),
        K::ResolveRider => format!("Resolve {}", in_play(true)?),
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
        K::BurnInPlay => format!("Burn {}", in_play(false)?),
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
        // Fleshforge Chamber: put on the card, not discarded.
        K::Discard => match o
            .effects
            .iter()
            .find(|e| e.detail.as_deref() == Some("on_source"))
        {
            Some(e) => format!("Put {} on {}", card(), n.card_id(e.card)),
            None => format!("Discard {}", card()),
        },
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
            } else if k.action == Some(ActionKind::Bleed) {
                // A bleed's target: the prey.
                s += &format!(" → {}", n.seat(n.prey(seat)?));
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
        K::EmployFromHand => {
            let m = k.mode.as_ref()?;
            format!("Employ {}{}{}", card(), mode(m), apart(m, n))
        }
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
        K::CancelCombat => "Cancel the combat".into(),
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
        // A play's second vampire (Toreador Grand Ball's): what it becomes.
        K::ChooseMinion
            if window == Window::PlayTerms
                && o.effects.iter().any(|e| e.kind == EffectKind::Role) =>
        {
            let r = o.effects.iter().find(|e| e.kind == EffectKind::Role)?;
            let clan = r.clan.as_deref().map_or("vampire".into(), |c| n.value(c));
            let role = match r.role? {
                Role::UnblockableNonBleed => "non-bleed actions cannot be blocked".into(),
                _ => format!("the card's chosen {clan}"),
            };
            format!("Choose {} ({role})", minion(k.minion))
        }
        K::ChooseMinion if window == Window::Distribution => {
            format!("1 blood to {}", minion(k.minion))
        }
        K::ChooseMinion => format!("Choose {}", minion(k.minion)),
        K::ChooseSeat => format!("Choose {}", target()),
        K::ChooseCard => {
            let name = in_play(false)?;
            match o.effects.iter().any(|e| e.kind == EffectKind::Steal) {
                // A referendum's terms: the card that changes control.
                true => format!("Choose {name} (changes control)"),
                false => format!("Choose {name}"),
            }
        }
        K::ChooseCircle => format!("Choose the {} Circle", n.value(k.value.as_deref()?)),
        // The Great Beast's Discipline.
        K::Slot if k.value.is_some() => format!("Choose [{}] at superior", k.value.as_deref()?),
        // Another seat's face down: by its place.
        K::Slot if k.card.is_none() => format!("Choose {}", back()?),
        K::Slot => {
            let c = k.card.as_ref()?;
            let name = match (c.copy, c.id) {
                (Some(copy), _) if n.knows(copy) => n.copy_bare(copy),
                _ => n.card_id(c.id),
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
                s += &apart(m, n);
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
                // A card the table does not show (its lasting effect's, the
                // card in the ash heap): named by its host.
                (Some(c), Some(m)) if !n.knows(c.copy?) => (
                    format!("{}'s {}", n.minion_bare(m), n.card_id(c.id)),
                    Some(m),
                ),
                (Some(c), _) => {
                    let copy = c.copy?;
                    (n.copy_bare(copy), n.host(copy).or(k.minion))
                }
                // A vampire's own: its actor named even when itself.
                (None, Some(m)) => (n.minion_bare(m), None),
                (None, None) => return None,
            };
            let mut s = format!("Use {name}");
            // Not the card's own minion (an ally's ability: War Ghoul's rush).
            let own = k.card.as_ref().and_then(|c| c.copy);
            if let Some(a) = k.actor.filter(|&a| Some(a) != host && Some(a) != own) {
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
    about: Option<&About>,
    base: Option<String>,
    n: &Names,
) -> Option<String> {
    use Purpose as P;
    let k = &o.key;
    // The purpose's particulars (`Decision::about`): the minion, the card.
    let whom = || Some(n.minion_bare(about?.minion?));
    let named = || {
        let c = about?.card.as_ref()?;
        Some(match (c.id, c.copy) {
            (Some(id), _) => n.card_id(Some(id)),
            (None, Some(copy)) => n.own(copy),
            (None, None) => return None,
        })
    };
    let role = |r: &str| Some(format!("{} ({r})", base.clone()?));
    let verb = |from: &str, to: &str| Some(base.clone()?.replacen(from, to, 1));
    let card = || n.card_id(k.card.as_ref().and_then(|c| c.id));
    let has = |kind| o.effects.iter().find(|e| e.kind == kind);
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
        // Ashur Tablets: to the hand first, then into the library.
        (P::AshurPicks, K::FindCard) => match has(EffectKind::Move)?.to? {
            Zone::Hand => format!("{} to your hand", card()),
            _ => format!("Shuffle {} into your library", card()),
        },
        (P::ExchangeAnarch, K::Discard) => format!(
            "{} (for {})",
            verb("Discard ", "Exchange ")?,
            n.card_id(has(EffectKind::Move)?.card)
        ),
        (P::DistributeFrom, K::Slot) => role("gets 1 blood")?,
        (P::DiscardFromHandOf, K::FindCard) => format!("Discard {}", card()),
        (P::PayOrBurn, K::PayToCancel) => format!(
            "Burn {} pool to keep {}",
            -has(EffectKind::Pool)?.n?,
            n.own(has(EffectKind::Keep)?.copy?)
        ),
        (P::DiablerieEquipment, K::ChooseCard) => role("moves")?,
        (P::MoveBetween, K::ChooseMinion) => role("receives")?,
        (P::DualFormKept, K::ChooseMinion) => verb("Choose ", "Keep ")?,
        (P::MoveBetween, K::ChooseCard) => role("moves")?,
        (P::ChooseForCard, K::ChooseMinion) => {
            let e = has(EffectKind::Role)?;
            let clan = e.clan.as_deref().map_or("vampire".into(), |c| n.value(c));
            role(&match e.role? {
                Role::UnblockableNonBleed => "non-bleed actions cannot be blocked".into(),
                Role::DoesNotUnlock => "does not unlock as normal".into(),
                _ => format!("the card's chosen {clan}"),
            })?
        }
        (P::BloodOrBurned, K::BurnBloodToAvoid) => {
            let e = has(EffectKind::Blood)?;
            format!("{} burns {} blood", n.minion_bare(e.copy?), -e.n?)
        }
        // a pass gains 1 pool.
        (P::AnarchConvert, K::ChooseMinion) => role("becomes Anarch")?,
        (P::CryptSearch, K::AshVampire) => format!(
            "Move {} from your crypt to your uncontrolled region",
            card()
        ),
        (P::GiveBlood, K::ChooseMinion) => role("receives 1 blood")?,
        (P::MayDrawExtra, K::DrawLibrary) => "Draw an additional card".into(),
        (P::Redraw, K::DrawLibrary) => {
            format!("Put {} on the bottom and draw a new card", named()?)
        }
        (P::EntryBurn, K::ChooseMinion) => role("burned")?,
        (P::PutFoundOn, K::FindCard) => format!("Put {} on it", named()?),
        (P::MayStoreAim, K::ResolveRider) => {
            format!("Move {} onto Light Intensifying Goggles", named()?)
        }
        (P::MayAddCounter, K::ResolveRider) => format!("Add 1 counter to {}", named()?),
        (P::MayTorture, K::ResolveRider) => "Lock The Path of Lilith: a torture counter".into(),
        (P::MayGainPool, K::ResolvePrinted) => format!("Gain {} pool", about?.n?),
        (P::AshToLibraryTop, K::FindCard) => format!("{} on top of your library", card()),
        (P::CutLibrary, K::SetX) => format!("Cut {} cards to the bottom", k.amount?),
        (P::BuyTitle, K::SetX) => {
            let pool = o.effects.iter().find(|e| e.cost)?.n?.unsigned_abs();
            let title = o.effects.iter().find_map(|e| e.detail.as_deref())?;
            format!("Pay {pool} pool: {}", title.replace('_', " "))
        }
        (P::MayGainHandSize, K::ResolvePrinted) => format!("Get +{} hand size", about?.n?),
        (P::MayPreventForBlood, K::ResolvePrinted) => {
            format!("Burn {} blood to prevent the damage", about?.n?)
        }
        (P::MayUnlock, K::ResolvePrinted) => "Unlock".into(),
        (P::ReductionAtResolution, K::ResolvePrinted) => "Pay 1 blood less".into(),
        (P::EntryRemoveOrBurn, K::FindCard) => verb("Take ", "Remove ")?,
        (P::EntryRemoveOrBurn, K::AshVampire) => verb("Choose ", "Remove ")?,
        (P::CancelAttempt, K::ChooseMinion) => "Cancel the attempt".into(),
        (P::CorruptionOwner, K::ChooseSeat) => role("their counter burns")?,
        (P::GiveCardTo, K::ChooseSeat) => role("takes control of it")?,
        (P::AvoidDamage, K::BurnBloodToAvoid) => {
            format!("Burn {} blood to avoid the damage", about?.n?)
        }
        (P::PoolForCard, K::ResolveRider) => {
            format!("Burn {}: gain {} pool", named()?, about?.n?)
        }
        (P::LockForPool, K::ResolveRider) => {
            format!("Lock {}: gain {} pool", named()?, about?.n?)
        }
        (P::StealForCorruption, K::ChooseMinion) => format!("Steal {}", whom()?),
        (P::MakeAnarch, K::ResolveRider) => {
            format!("Burn 1 blood from {}: {} becomes Anarch", named()?, whom()?)
        }
        (P::BecomesAnarch, K::ChooseMinion) => role("becomes Anarch")?,
        (P::UncontrolledBlood, K::Slot) => role(&format!("gets {} blood", about?.n?))?,
        (P::LockOrPay, K::ResolveRider) => format!("Lock {}", named()?),
        (P::LockOrPay, K::BurnPool) => format!("Burn {} pool", about?.n?),
        (P::PoolOrVampire, K::BurnPool) => format!("Burn {} pool", about?.n?),
        (P::PoolOrNoTransfers, K::BurnPool) => format!("Burn {} pool", about?.n?),
        (P::BloodOrCamarilla, K::BurnBloodToAvoid) => "Burn 1 blood".into(),
        (P::PoolOrVampire, K::ChooseMinion) => role("is burned")?,
        (P::BaseSearch, K::AshVampire) => format!("Merge with {}", card()),
        (P::BaseSearch, K::Slot) => "Merge with this vampire".into(),
        (P::PreySlotToPool, K::Slot) => format!(
            "1 blood from an uncontrolled vampire of {} to their pool",
            n.seat(about?.seat?)
        ),
        (P::MayRecycleCanceled, K::ResolvePrinted) => {
            format!("Shuffle {} into your library", named()?)
        }
        (P::NameAshPair, K::FindCard) => format!("Remove two {}", card()),
        (P::AllyFromAsh, K::FindCard) => format!("Move {} to your ready region", card()),
        (P::MoveForBlood, K::ChooseMinion) => format!(
            "{} burns {} blood: move {} to your ready region",
            whom()?,
            about?.n?,
            n.minion_bare(about?.to?)
        ),
        (P::MoveCardToTitled | P::MoveOffBurnedBearer, K::ChooseMinion) => {
            role(&format!("gets {}", named()?))?
        }
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

fn detail(e: &Effect) -> Option<&str> {
    e.detail.as_deref()
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
    // Constant Revolution, The Gate of Acheron: cards, the rest in pool.
    let rest = || {
        o.effects
            .iter()
            .find(|e| e.cost && e.kind == EffectKind::Pool)
            .and_then(|e| e.n)
            .map_or(0, |n| -n)
    };
    match o.effects.first().and_then(detail) {
        Some("hand_at_random") => {
            return Some(format!(
                "Burn {a} card(s) at random from your hand and {} pool",
                rest()
            ))
        }
        Some("ash_at_random") => {
            return Some(format!(
                "Remove {a} card(s) at random from your ash heap and burn {} pool",
                rest()
            ))
        }
        _ => {}
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
        // Jake Washington: put on the vampire.
        EffectKind::Blood if e.n > Some(0) && e.copy.is_some() => {
            format!("{a} blood on {}", n.minion_bare(e.copy?))
        }
        EffectKind::Blood => match e.copy {
            Some(c) => format!("{a} blood from {}", n.minion_bare(c)),
            None => format!("{a} blood"),
        },
        EffectKind::Move if detail(e) == Some("hand_to_bottom") => {
            format!("{a} card(s) to the bottom of the library")
        }
        // Ilonka: from the acting minion's controller's hand.
        EffectKind::Discard if detail(e) == Some("at_random") => match e.seat {
            Some(s) => format!("Discard {a} card(s) at random from {}'s hand", n.seat(s)),
            None => format!("Discard {a} card(s) at random"),
        },
        EffectKind::Pool if e.detail.as_deref() == Some("from_blood") => {
            format!("{a} blood to your pool")
        }
        EffectKind::Pool => format!("Give {a} pool"),
        EffectKind::Counters => format!("{a} counter(s)"),
        EffectKind::Draw if detail(e) == Some("then_bottom") => {
            format!("Draw {a} card(s), then {a} to the bottom of the library")
        }
        EffectKind::Draw => format!("{a} card(s)"),
        _ => return None,
    })
}

/// A pass, from its own effects (what it does, where it does more than
/// decline), its window and its purpose's particulars: never from the
/// options beside it or the view (docs/protocol.md §7). `None`: a pass
/// whose doing is not in its effects.
fn pass(
    o: &OptionView,
    window: Window,
    purpose: Option<Purpose>,
    about: Option<&About>,
    n: &Names,
) -> Option<String> {
    use Purpose as P;
    let has = |kind| o.effects.iter().find(|e| e.kind == kind);
    let named = || {
        let c = about?.card.as_ref()?;
        Some(match (c.id, c.copy) {
            (Some(id), _) => n.card_id(Some(id)),
            (None, Some(copy)) => n.own(copy),
            (None, None) => return None,
        })
    };
    let detail = about.and_then(|a| a.detail.as_deref());
    if let Some(e) = has(EffectKind::EndPhase) {
        return Some(format!("End the {} phase", e.detail.as_deref()?));
    }
    if let Some(e) = has(EffectKind::Block) {
        return Some(
            match e.detail.as_deref() {
                Some("resigned") => "Resign the attempt",
                Some("declines") => "Decline to block",
                _ => "Pass (blocking)",
            }
            .into(),
        );
    }
    let plain = || -> Option<String> {
        Some(
            match window {
                // Its effects say it ends the phase.
                Window::MasterPhase
                | Window::MinionPhase
                | Window::InfluenceTransfers
                | Window::DiscardPhase => return None,
                Window::CombatPress => "Pass (no press)",
                Window::ReferendumPolling | Window::BloodHuntPolling => "Pass (no vote now)",
                // A cost's terms: the pass reduces nothing.
                Window::PlayTerms
                    if matches!(detail, Some("cost_counters" | "cost_reductions")) =>
                {
                    "No reduction"
                }
                _ => "Pass",
            }
            .into(),
        )
    };
    let Some(p) = purpose else {
        return plain();
    };
    Some(match p {
        // What it does.
        P::PayOrBurn => format!("Let {} burn", n.own(has(EffectKind::Burn)?.copy?)),
        P::BloodOrBurned => {
            let e = has(EffectKind::Burn)?;
            let name = match e.card {
                Some(c) => n.card_id(Some(c)),
                None => n.minion_bare(e.copy?),
            };
            format!("Let {name} burn")
        }
        P::EntryRemoveOrBurn => format!("Burn {}", n.minion_bare(has(EffectKind::Burn)?.copy?)),
        P::MoveOffBurnedBearer => {
            has(EffectKind::Burn)?;
            "Let it burn".into()
        }
        P::LockOrDiscard => format!("Lock {}", n.minion_bare(has(EffectKind::Lock)?.copy?)),
        P::AvoidDamage => format!("Take the {} damage", has(EffectKind::Damage)?.n?),
        P::BloodOrCamarilla => {
            has(EffectKind::Role)?;
            "Become Camarilla".into()
        }
        P::PoolOrNoTransfers => {
            has(EffectKind::Transfers)?;
            "Lose all transfers next influence phase".into()
        }
        // Its `block` effect says it resigns.
        P::BlockToll => return None,
        // Gilbert Duane: once chosen, a pass gains 1 pool.
        P::AnarchConvert if has(EffectKind::Pool).is_some() => "Gain 1 pool".into(),
        // What declining does, where a bare pass would not say.
        P::AnarchConvert => "Keep him in play".into(),
        P::MinionOf => match detail {
            Some("lock" | "lock_vampires") => "Lock no more".into(),
            Some("unlock_anarch") => "Unlock none".into(),
            _ => "Choose none".into(),
        },
        // Ashur Tablets: to the hand first, then into the library.
        P::AshurPicks if detail == Some("to_library") => "No more".into(),
        P::AshurPicks => "None to your hand".into(),
        P::CutLibrary => "Do not cut".into(),
        P::XConvictions => "No more".into(),
        // A distribution walks the recipients: the pass skips this one.
        P::MarginBlood => match about.and_then(|a| a.minion) {
            Some(m) => format!("No blood to {}", n.minion_bare(m)),
            None => "Pass".into(),
        },
        P::DiablerieEquipment => "Take nothing more".into(),
        P::MoveBetween | P::CircleBlood => "Move nothing more".into(),
        P::DistributeFrom => "Move no more blood".into(),
        P::CryptSearch | P::BaseSearch | P::LibraryToHand => "Find none".into(),
        P::ChooseCircle => "Keep his own circle".into(),
        P::StoreSearch | P::DisciplineSearch => "Stop searching".into(),
        P::AllySearch => "Find nothing".into(),
        P::MayDrawExtra => "Draw no more".into(),
        P::Redraw => format!("Keep {}", named()?),
        P::MayPutOn | P::MayStoreAim => format!("Leave {} in the ash heap", named()?),
        P::PutFoundOn => format!("Keep {} in your hand", named()?),
        P::MayAddCounter => "Add no counter".into(),
        P::MayTorture => "No torture counter".into(),
        P::MayGainPool => "Gain no pool".into(),
        P::BuyTitle => "Buy no title".into(),
        P::MayGainHandSize => "Keep your hand size".into(),
        P::MayPreventForBlood => "Burn no blood".into(),
        P::MayUnlock => "Stay locked".into(),
        P::ReductionAtResolution => "Let the card fizzle".into(),
        P::CancelAttempt => "Keep the attempt".into(),
        P::PoolForCard | P::LockForPool => "No pool".into(),
        P::StealForCorruption => "Don't steal".into(),
        P::MakeAnarch => "No Anarch".into(),
        P::MoveForBlood => "Don't move it".into(),
        P::PoolToVampires => "Move no more pool".into(),
        P::MayRecycleCanceled => match named() {
            Some(c) => format!("Leave {c} in your ash heap"),
            None => "Leave it in your ash heap".into(),
        },
        P::NameOwnAshCard => "Name none".into(),
        P::PutInPlaySearch => "Put nothing in play".into(),
        P::ReadyTitledLeft => "Play no card for it".into(),
        P::SearchEquipFor => "Find no equipment".into(),
        P::UncontrolledBlood => "Add no blood".into(),
        _ => plain()?,
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

/// What sets a mode apart from the others its printed line splits into,
/// in parentheses.
fn apart(m: &Mode, n: &Names) -> String {
    match label(&m.apart, n) {
        l if l.is_empty() => String::new(),
        l => format!(" ({l})"),
    }
}

/// What an ability does, after who and on what: blood moved to or from
/// the pool; which of the source's abilities (what sets it apart from
/// the others, `OptionView::apart`); a lone ability's action.
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
    // Each ability of the source offered: what sets it apart.
    let mut abilities: Vec<(u8, &[Effect])> = Vec::new();
    for s in offered.iter().filter(|s| same(s)) {
        let i = s.key.ability.unwrap_or(0);
        if !abilities.iter().any(|(j, _)| *j == i) {
            abilities.push((i, &s.apart));
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
        let l = label(&o.apart, n);
        let clash = abilities
            .iter()
            .any(|(i, a)| *i != ability && label(a, n) == l);
        return Some(if l.is_empty() || clash {
            format!(" (ability {})", ability + 1)
        } else {
            format!(": {l}")
        });
    }
    // A lone ability: what it does.
    let l = label(&o.effects, n);
    Some(if l.is_empty() {
        String::new()
    } else {
        format!(": {l}")
    })
}

/// A Sabbat path's name, from its id.
fn path_name(id: &str) -> &str {
    match id {
        "caine" => "Caine",
        "cathari" => "Cathari",
        "death_and_the_soul" => "Death and the Soul",
        "power_and_the_inner_voice" => "Power and the Inner Voice",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use zvtes_api::ids::{ActionKind, ActionStep, KrcgId};
    use zvtes_api::view::ActionView;

    use crate::{Cards, Names};

    struct NoCards;

    impl Cards for NoCards {
        fn name(&self, _: KrcgId) -> Option<&str> {
            None
        }
    }

    fn bare_pass() -> zvtes_api::option::OptionView {
        use zvtes_api::ids::OptionKind;
        use zvtes_api::option::{ChoiceKey, OptionView};
        OptionView {
            index: 0,
            key: ChoiceKey {
                kind: OptionKind::Pass,
                card: None,
                actor: None,
                minion: None,
                target: None,
                action: None,
                mode: None,
                amount: None,
                favor: None,
                blood: None,
                zone: None,
                value: None,
                ability: None,
                slot: None,
            },
            effects: Vec::new(),
            abilities: None,
            apart: Vec::new(),
            user: None,
            text: None,
        }
    }

    // The block window's passes read from their effects alone (rules-qa
    // 1.3, 82.1): the action under way in the view changes nothing.
    #[test]
    fn a_block_window_pass_reads_from_its_effects() {
        use zvtes_api::ids::Window;
        use zvtes_api::option::{Effect, EffectKind};
        let mut n = Names::new(&NoCards);
        n.action = Some(ActionView {
            seat: 0,
            acting: 3,
            kind: ActionKind::Bleed,
            step: ActionStep::Attempt,
            card: None,
            mode: None,
            x: None,
            source: None,
            target: None,
            bleed_target: Some(1),
            bleed: Some(1),
            stealth: 1,
            blocker: Some(7),
            intercept: Some(0),
            declined: vec![],
            declining: true,
        });
        let with = |e: Option<Effect>| {
            let mut o = bare_pass();
            o.effects.extend(e);
            super::option(&o, Window::ActionBlock, None, None, 1, &[], &n)
        };
        let block = |copy, detail: Option<&str>| Effect {
            copy,
            detail: detail.map(Into::into),
            ..Effect::new(EffectKind::Block)
        };
        assert_eq!(with(None).as_deref(), Some("Pass"));
        assert_eq!(
            with(Some(block(None, Some("declines")))).as_deref(),
            Some("Decline to block")
        );
        assert_eq!(
            with(Some(block(Some(7), Some("declines")))).as_deref(),
            Some("Decline to block")
        );
        assert_eq!(
            with(Some(block(Some(7), None))).as_deref(),
            Some("Pass (blocking)")
        );
    }

    /// The purposes whose choice offers a pass (the engine's
    /// `interrupt_request`): each pass says what declining does, not
    /// "Pass".
    #[test]
    fn every_declined_purpose_says_what_declining_does() {
        use zvtes_api::ids::{Purpose as P, Window};
        use zvtes_api::message::About;
        use zvtes_api::option::{CardRef, Effect, EffectKind as E};
        let n = Names::new(&NoCards);
        let about = About {
            n: Some(2),
            m: Some(1),
            cap: Some(3),
            seat: Some(1),
            minion: Some(7),
            to: Some(8),
            card: Some(CardRef {
                id: Some(100001),
                copy: Some(9),
                from: None,
            }),
            clan: Some("brujah".into()),
            detail: None,
        };
        let pass = bare_pass();
        let declined = [
            P::AllySearch,
            P::AnarchConvert,
            P::AshurPicks,
            P::AvoidDamage,
            P::BaseSearch,
            P::BlockToll,
            P::CancelAttempt,
            P::MoveForBlood,
            P::LockForPool,
            P::PoolForCard,
            P::MayStoreAim,
            P::MayTorture,
            P::MayGainHandSize,
            P::MayPreventForBlood,
            P::MayUnlock,
            P::ReductionAtResolution,
            P::BloodOrBurned,
            P::BloodOrCamarilla,
            P::BuyTitle,
            P::ChooseCircle,
            P::CircleBlood,
            P::CryptSearch,
            P::DiablerieEquipment,
            P::DisciplineSearch,
            P::DistributeFrom,
            P::EntryRemoveOrBurn,
            P::LibraryToHand,
            P::LockOrDiscard,
            P::MakeAnarch,
            P::MarginBlood,
            P::MayAddCounter,
            P::MayDrawExtra,
            P::MayGainPool,
            P::MayPutOn,
            P::PutFoundOn,
            P::MayRecycleCanceled,
            P::MinionOf,
            P::MoveBetween,
            P::MoveOffBurnedBearer,
            P::NameOwnAshCard,
            P::PayOrBurn,
            P::PoolOrNoTransfers,
            P::PoolToVampires,
            P::PutInPlaySearch,
            P::ReadyTitledLeft,
            P::Redraw,
            P::SearchEquipFor,
            P::StealForCorruption,
            P::StoreSearch,
            P::UncontrolledBlood,
        ];
        let mut generic = Vec::new();
        for p in declined {
            // What it does, as the host tells a pass doing more than
            // declining: without it, the pass is untold.
            let does = |kind| Effect {
                copy: Some(9),
                n: Some(1),
                ..Effect::new(kind)
            };
            let effect = match p {
                P::BloodOrBurned | P::PayOrBurn | P::EntryRemoveOrBurn | P::MoveOffBurnedBearer => {
                    Some(does(E::Burn))
                }
                P::LockOrDiscard => Some(does(E::Lock)),
                P::AvoidDamage => Some(does(E::Damage)),
                P::BloodOrCamarilla => Some(does(E::Role)),
                P::PoolOrNoTransfers => Some(does(E::Transfers)),
                P::BlockToll => Some(Effect {
                    detail: Some("resigned".into()),
                    ..does(E::Block)
                }),
                _ => None,
            };
            let window = match p {
                P::MarginBlood => Window::Distribution,
                _ => Window::ResolutionChoice,
            };
            let mut pass = pass.clone();
            if effect.is_some() {
                let t = super::option(&pass, window, Some(p), Some(&about), 0, &[], &n);
                assert_eq!(t, None, "{p:?} told without its effect");
            }
            pass.effects.extend(effect);
            let t = super::option(&pass, window, Some(p), Some(&about), 0, &[], &n);
            if t.as_deref().is_none_or(|t| t == "Pass") {
                generic.push((p, t));
            }
        }
        assert!(generic.is_empty(), "declined as a bare pass: {generic:?}");
    }
}
