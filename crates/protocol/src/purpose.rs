//! What a pending choice is for, when its window and bare options do not
//! say (owner, 2026-10-04: "cards offering a choice like that need the
//! choice to be very clear"): the choices an effect asks as it resolves
//! (`Engine::interrupt`), and the terms of a play, a cost or an in-play
//! ability. A prompt for the decision, and each option's text rewritten
//! to say what it does — keyed on the engine's variants, never on card
//! names.

use zvtes::cards::{AbilitySource, Effect, Modifier, Target, Terms};
use zvtes::decision::{Choice, WindowLabel};
use zvtes::draws::{Interrupt, MinionPurpose};
use zvtes::ids::MinionId;
use zvtes::library::LIBRARY;

use crate::describe::Describer;

/// A card in play's chosen vampire (`Interrupt::ChooseForCard`), by what
/// its definition does with it: who it is, and its role as an option.
fn chosen_for_card(card: u16, second: bool, clan: Option<&str>) -> (String, String) {
    let effects = zvtes::cards::lookup(card)
        .and_then(|d| d.modes.first())
        .map_or(&[][..], |m| m.effects);
    let has = |m: Modifier| effects.contains(&Effect::Layer(m));
    let clan = clan.unwrap_or("vampire");
    if second && has(Modifier::SecondChosenUnblockable) {
        (
            format!("the {clan} whose non-bleed actions cannot be blocked"),
            "non-bleed actions cannot be blocked".into(),
        )
    } else if !second && has(Modifier::FirstChosenDoesNotUnlock) {
        (
            format!("the {clan} who does not unlock as normal"),
            "does not unlock as normal".into(),
        )
    } else {
        (
            format!("the card's chosen {clan}"),
            format!("the card's chosen {clan}"),
        )
    }
}

impl Describer<'_> {
    fn card_name(&self, c: zvtes::state::CardId) -> &'static str {
        self.in_play_name(c.0)
    }

    fn source_name(&self, source: AbilitySource) -> String {
        match source {
            AbilitySource::Card(c) => self.card_name(c).to_string(),
            AbilitySource::Vampire(m) => self.minion_name(m),
        }
    }

    fn named(&self, m: MinionId) -> String {
        self.minion_name(m)
    }

    /// The prompt of a choice an effect asks as it resolves, or of a play's,
    /// a cost's, an ability's or a rider's terms.
    pub(crate) fn purpose_prompt(&self) -> Option<String> {
        if let Some(i) = self.e.interrupt() {
            return self.interrupt_prompt(i);
        }
        match self.window {
            WindowLabel::PlayTerms => {
                if let Some(p) = self.e.play_terms() {
                    let card = LIBRARY[p.card as usize].name;
                    let what = match zvtes::cards::lookup(p.card)?
                        .modes
                        .get(p.mode as usize)?
                        .terms?
                    {
                        Terms::BloodRange { .. } => "how much blood".to_string(),
                        Terms::UpToTargetBlood => {
                            format!(
                                "how much blood to take from {}",
                                self.named(p.target.minion())
                            )
                        }
                        Terms::XBlood | Terms::XBloodIn { .. } => "X, paid in blood".into(),
                        Terms::UpToMargin(_) => "how much blood moves to your pool".into(),
                        Terms::BoonPool => "how much pool to give".into(),
                        Terms::BurnNamedInPlay(name) => format!("which {name} to burn"),
                        Terms::AnyAshAlly => match p.ash {
                            None => "the ash heap to take an ally from".into(),
                            Some(_) => "the ally".into(),
                        },
                        Terms::EquipFromHand { .. } => match p.named {
                            None => "the equipment".into(),
                            Some(_) => format!(
                                "how much of the cost {} pays in blood",
                                p.actor.map_or("the vampire".into(), |m| self.named(m))
                            ),
                        },
                        Terms::EmployFromHand(_)
                        | Terms::EmployFromHandIgnoring(_)
                        | Terms::RecruitOrEmployFromHand => "the card to put in play".into(),
                        _ => return None,
                    };
                    return Some(format!("{card}: {what}"));
                }
                if let Some(p) = self.e.cost_terms() {
                    let card = LIBRARY[p.card as usize].name;
                    return Some(format!("{card}: the cost reductions to use"));
                }
                let (source, mode, target) = self.e.ability_terms()?;
                let what = match mode.terms? {
                    Terms::Discard { n, .. } => format!("discard {n} card(s) to use it"),
                    Terms::Count { .. } => "how many cards".into(),
                    Terms::XPool => "how much pool to pay".into(),
                    Terms::XCounters => "how many counters to use".into(),
                    Terms::LifeFromTarget => {
                        format!(
                            "how much blood to move from {}",
                            self.named(target.minion())
                        )
                    }
                    Terms::AshCard => "a card in your ash heap".into(),
                    Terms::DirectedHandCard => "a card from the hand".into(),
                    Terms::VampireToHoldTarget => match target {
                        Target::Card(c) => format!("the vampire who gets {}", self.card_name(c)),
                        _ => "the vampire who gets the card".into(),
                    },
                    _ => return None,
                };
                Some(format!("{}: {what}", self.source_name(source)))
            }
            WindowLabel::RiderTerms => {
                let (card, left) = self.e.rider_terms()?;
                Some(format!("{}: {left} still to pay", self.card_name(card)))
            }
            _ => None,
        }
    }

    fn interrupt_prompt(&self, i: Interrupt) -> Option<String> {
        use Interrupt::*;
        Some(match i {
            GiveBlood { from, .. } => {
                format!("Choose the vampire who receives 1 blood from {}", self.named(from))
            }
            PayOrBurn { card, pool, .. } => format!(
                "{} burns unless you burn {pool} pool",
                self.card_name(card)
            ),
            LockOrDiscard { minion, .. } => format!(
                "{} is locked unless you discard a card",
                self.named(minion)
            ),
            StoreSearch { card, left, .. } => format!(
                "Search your library for non-unique equipment to put on {} ({left} more at most)",
                self.card_name(card)
            ),
            EntryBurn { ally, .. } => format!(
                "{} entered play: burn an ally or retainer you control",
                self.named(ally)
            ),
            EntryRemoveOrBurn { ally, .. } => format!(
                "{} burns unless you remove an ally or vampire in your ash heap from the game",
                self.named(ally)
            ),
            BlockToll { minion, .. } => format!(
                "{}'s block attempt has a toll: pay it or resign the attempt",
                self.named(minion)
            ),
            CancelAttempt { minion, .. } => {
                format!("{} can cancel its block attempt", self.named(minion))
            }
            MinionOf { of, purpose, .. } => {
                let of = if of == self.seat {
                    "of yours".to_string()
                } else {
                    format!("of {}", self.seat_name(of))
                };
                match purpose {
                    MinionPurpose::Lock => format!("You can lock a minion {of}"),
                    MinionPurpose::Corrupt => {
                        format!("Choose a minion {of} to get a corruption counter")
                    }
                    MinionPurpose::LockVampire => format!("Lock up to two vampires {of}"),
                    MinionPurpose::UnlockAnarch => "You can unlock a ready Anarch of yours".into(),
                    MinionPurpose::BurnBloodOfClan(c) => {
                        format!("Choose a ready {c} of yours to burn 1 blood")
                    }
                    MinionPurpose::HostCard(card) => {
                        format!("Choose the minion {of} who gets {}", self.card_name(card))
                    }
                    MinionPurpose::LockOldest(c) => {
                        format!("Choose one of the oldest {c} {of} to lock")
                    }
                    MinionPurpose::BurnOfCrowdedClan => {
                        "Choose a vampire to burn, of a clan you control 3 or more of".into()
                    }
                    MinionPurpose::Damage(n) => format!(
                        "Choose a ready minion of yours to take {n} unpreventable damage"
                    ),
                }
            }
            CorruptionOwner { minion, .. } => format!(
                "Choose whose corruption counter on {} burns",
                self.named(minion)
            ),
            AvoidDamage {
                minion,
                blood,
                damage,
                ..
            } => format!(
                "{} takes {damage} unpreventable aggravated damage unless it burns {blood} blood",
                self.named(minion)
            ),
            DiscardFromHandOf { of, .. } => format!(
                "Discard a card from {}'s hand (they replace it)",
                self.seat_name(of)
            ),
            DiscardChosen { left, .. } => format!("Discard {left} card(s) of your choice"),
            RemoveFromAsh { of, left, .. } => format!(
                "Remove {left} card(s) in {}'s ash heap from the game",
                if of == self.seat {
                    "your".into()
                } else {
                    self.seat_name(of)
                }
            ),
            DisciplineSearch { minion, .. } => format!(
                "You can search for a Discipline master card to put on {}",
                self.named(minion)
            ),
            RemoveUncontrolled { .. } => {
                "Remove a crypt card in your uncontrolled region from the game".into()
            }
            ChooseForCard {
                card,
                clan,
                second,
                ..
            } => {
                let id = self.e.state.cards[card.index()].card;
                let (who, _) = chosen_for_card(id, second, clan);
                format!("{}: choose {who}", self.card_name(card))
            }
            ExchangeAnarch { ash: None, .. } => {
                "Choose a card requiring an Anarch in your ash heap, to exchange with a card in your hand"
                    .into()
            }
            ExchangeAnarch { ash: Some(c), .. } => format!(
                "Choose the card in your hand to exchange for {}",
                LIBRARY[c as usize].name
            ),
            PoolForCard { card, n, .. } => {
                format!("You can burn {} to gain {n} pool", self.card_name(card))
            }
            LockForPool { card, n, .. } => {
                format!("You can lock {} to gain {n} pool", self.card_name(card))
            }
            StealForCorruption { minion, .. } => format!(
                "You can burn the corruption counters on {} to steal it",
                self.named(minion)
            ),
            MakeAnarch { card, minion, .. } => format!(
                "You can burn 1 blood from {} to make {} Anarch",
                self.card_name(card),
                self.named(minion)
            ),
            AshurPicks {
                to_library: false, ..
            } => "Choose the card in your ash heap to put in your hand".into(),
            AshurPicks { left, .. } => {
                format!("Choose up to {left} more ash heap cards to shuffle into your library")
            }
            BecomesAnarch { .. } => "Choose a non-titled vampire of yours to become Anarch".into(),
            UncontrolledBlood { clan, n, .. } => {
                format!("Add {n} blood to a {clan} vampire in your uncontrolled region")
            }
            AllyFromAsh { .. } => {
                "Choose the ally to move from your ash heap to your ready region".into()
            }
            MoveForBlood {
                payer, minion, n, ..
            } => format!(
                "{} can burn {n} blood to move {} to your ready region",
                self.named(payer),
                self.named(minion)
            ),
            MoveCardToTitled { card, .. } => format!(
                "Move {} to another justicar or Inner Circle member",
                self.card_name(card)
            ),
            Replacement { .. }
            | DrawUp { .. }
            | Draws { .. }
            | Bottom { .. }
            | MarginPool { .. }
            | MarginBlood { .. } => return None,
        })
    }

    /// An option's text, said for the choice pending (`text`: as it reads
    /// anywhere); `None` keeps it.
    pub(crate) fn purpose_text(&self, c: &Choice, text: &str) -> Option<String> {
        let role = |r: String| Some(format!("{text} ({r})"));
        if let Some(i) = self.e.interrupt() {
            use Interrupt::*;
            return match (i, *c) {
                (GiveBlood { .. }, Choice::ChooseMinion(_)) => role("receives 1 blood".into()),
                (PayOrBurn { card, .. }, Choice::Pass) => {
                    Some(format!("Let {} burn", self.card_name(card)))
                }
                (PayOrBurn { card, pool, .. }, Choice::PayToCancel) => {
                    Some(format!("Burn {pool} pool to keep {}", self.card_name(card)))
                }
                (LockOrDiscard { minion, .. }, Choice::Pass) => {
                    Some(format!("Lock {}", self.named(minion)))
                }
                (StoreSearch { .. } | DisciplineSearch { .. }, Choice::Pass) => {
                    Some("Stop searching".into())
                }
                (EntryBurn { .. }, Choice::ChooseMinion(_)) => role("burned".into()),
                (EntryRemoveOrBurn { ally, .. }, Choice::Pass) => {
                    Some(format!("Burn {}", self.named(ally)))
                }
                (EntryRemoveOrBurn { .. } | RemoveFromAsh { .. }, Choice::FindCard(_))
                | (EntryRemoveOrBurn { .. } | RemoveFromAsh { .. }, Choice::AshVampire(_)) => Some(
                    text.replacen("Take ", "Remove ", 1)
                        .replacen("Choose ", "Remove ", 1),
                ),
                (BlockToll { .. }, Choice::Pass) => Some("Resign the attempt".into()),
                (CancelAttempt { .. }, Choice::ChooseMinion(_)) => {
                    Some("Cancel the attempt".into())
                }
                (CancelAttempt { .. }, Choice::Pass) => Some("Keep the attempt".into()),
                (MinionOf { purpose, .. }, Choice::ChooseMinion(_)) => role(match purpose {
                    MinionPurpose::Lock
                    | MinionPurpose::LockVampire
                    | MinionPurpose::LockOldest(_) => "locked".into(),
                    MinionPurpose::Corrupt => "gets a corruption counter".into(),
                    MinionPurpose::UnlockAnarch => "unlocked".into(),
                    MinionPurpose::BurnBloodOfClan(_) => "burns 1 blood".into(),
                    MinionPurpose::HostCard(card) => format!("gets {}", self.card_name(card)),
                    MinionPurpose::BurnOfCrowdedClan => "burned".into(),
                    MinionPurpose::Damage(n) => format!("takes {n} damage"),
                }),
                (MinionOf { purpose, .. }, Choice::Pass) => Some(
                    match purpose {
                        MinionPurpose::Lock | MinionPurpose::LockVampire => "Lock no more",
                        MinionPurpose::UnlockAnarch => "Unlock none",
                        _ => "Pass",
                    }
                    .into(),
                ),
                (CorruptionOwner { .. }, Choice::ChooseSeat(_)) => {
                    role("their counter burns".into())
                }
                (AvoidDamage { damage, .. }, Choice::Pass) => {
                    Some(format!("Take the {damage} damage"))
                }
                (AvoidDamage { blood, .. }, Choice::BurnBloodToAvoid) => {
                    Some(format!("Burn {blood} blood to avoid the damage"))
                }
                (DiscardFromHandOf { .. }, Choice::FindCard(card)) => {
                    Some(format!("Discard {}", LIBRARY[card as usize].name))
                }
                (RemoveUncontrolled { .. }, Choice::Slot(_)) => {
                    Some(text.replacen("Choose ", "Remove ", 1))
                }
                (
                    ChooseForCard {
                        card, clan, second, ..
                    },
                    Choice::ChooseMinion(_),
                ) => {
                    let id = self.e.state.cards[card.index()].card;
                    role(chosen_for_card(id, second, clan).1)
                }
                (ExchangeAnarch { ash: Some(c), .. }, Choice::Discard(_)) => Some(format!(
                    "{} (for {})",
                    text.replacen("Discard ", "Exchange ", 1),
                    LIBRARY[c as usize].name
                )),
                (PoolForCard { card, n, .. }, Choice::ResolveRider { .. }) => {
                    Some(format!("Burn {}: gain {n} pool", self.card_name(card)))
                }
                (LockForPool { card, n, .. }, Choice::ResolveRider { .. }) => {
                    Some(format!("Lock {}: gain {n} pool", self.card_name(card)))
                }
                (PoolForCard { .. } | LockForPool { .. }, Choice::Pass) => Some("No pool".into()),
                (StealForCorruption { minion, .. }, Choice::ChooseMinion(_)) => {
                    Some(format!("Steal {}", self.named(minion)))
                }
                (StealForCorruption { .. }, Choice::Pass) => Some("Don't steal".into()),
                (MakeAnarch { card, minion, .. }, Choice::ResolveRider { .. }) => Some(format!(
                    "Burn 1 blood from {}: {} becomes Anarch",
                    self.card_name(card),
                    self.named(minion)
                )),
                (MakeAnarch { .. }, Choice::Pass) => Some("No Anarch".into()),
                (
                    AshurPicks {
                        to_library: false, ..
                    },
                    Choice::FindCard(card),
                ) => Some(format!("{} to your hand", LIBRARY[card as usize].name)),
                (
                    AshurPicks {
                        to_library: false, ..
                    },
                    Choice::Pass,
                ) => Some("None to your hand".into()),
                (AshurPicks { .. }, Choice::FindCard(card)) => Some(format!(
                    "Shuffle {} into your library",
                    LIBRARY[card as usize].name
                )),
                (AshurPicks { .. }, Choice::Pass) => Some("No more".into()),
                (BecomesAnarch { .. }, Choice::ChooseMinion(_)) => role("becomes Anarch".into()),
                (UncontrolledBlood { n, .. }, Choice::Slot(_)) => role(format!("gets {n} blood")),
                (AllyFromAsh { .. }, Choice::FindCard(card)) => Some(format!(
                    "Move {} to your ready region",
                    LIBRARY[card as usize].name
                )),
                (
                    MoveForBlood {
                        payer, minion, n, ..
                    },
                    Choice::ChooseMinion(_),
                ) => Some(format!(
                    "{} burns {n} blood: move {} to your ready region",
                    self.named(payer),
                    self.named(minion)
                )),
                (MoveForBlood { .. }, Choice::Pass) => Some("Don't move it".into()),
                (MoveCardToTitled { card, .. }, Choice::ChooseMinion(_)) => {
                    role(format!("gets {}", self.card_name(card)))
                }
                _ => None,
            };
        }
        match (self.window, *c) {
            (WindowLabel::PlayTerms, Choice::SetX(n)) => {
                if let Some(p) = self.e.play_terms() {
                    let terms = zvtes::cards::lookup(p.card)?
                        .modes
                        .get(p.mode as usize)?
                        .terms?;
                    return match terms {
                        Terms::BloodRange { .. } => Some(format!("{n} blood")),
                        Terms::UpToTargetBlood => {
                            Some(format!("{n} blood from {}", self.named(p.target.minion())))
                        }
                        Terms::XBlood | Terms::XBloodIn { .. } => {
                            Some(format!("X = {n} ({n} blood)"))
                        }
                        Terms::UpToMargin(_) => Some(format!("{n} blood to your pool")),
                        Terms::BoonPool => Some(format!("Give {n} pool")),
                        Terms::EquipFromHand { .. } => Some(format!("{n} blood")),
                        _ => None,
                    };
                }
                if self.e.cost_terms().is_some() {
                    return Some(format!("{n} counter(s)"));
                }
                let (_, mode, _) = self.e.ability_terms()?;
                match mode.terms? {
                    Terms::Count { .. } => Some(format!("{n} card(s)")),
                    Terms::XPool => Some(format!("X = {n} ({n} pool)")),
                    Terms::XCounters => Some(format!("X = {n} ({n} counters)")),
                    Terms::LifeFromTarget => Some(format!("{n} blood")),
                    _ => None,
                }
            }
            (WindowLabel::PlayTerms, Choice::ChooseMinion(_)) => {
                if self.e.cost_terms().is_some() {
                    return role("pays 1 of the cost in blood".into());
                }
                let (_, mode, target) = self.e.ability_terms()?;
                match (mode.terms?, target) {
                    (Terms::VampireToHoldTarget, Target::Card(card)) => {
                        role(format!("gets {}", self.card_name(card)))
                    }
                    _ => None,
                }
            }
            (WindowLabel::PlayTerms, Choice::ChooseSeat(_)) => {
                let p = self.e.play_terms()?;
                let terms = zvtes::cards::lookup(p.card)?
                    .modes
                    .get(p.mode as usize)?
                    .terms?;
                matches!(terms, Terms::AnyAshAlly).then(|| format!("{text}'s ash heap"))
            }
            (WindowLabel::PlayTerms, Choice::Pass) if self.e.cost_terms().is_some() => {
                Some("No reduction".into())
            }
            (WindowLabel::RiderTerms, Choice::SetX(n)) => {
                Some(format!("{n} card(s), the rest in pool"))
            }
            _ => None,
        }
    }
}
