//! Option descriptors (docs/human-play.md): every `Choice` as structured
//! data (what clients highlight and match gestures against) and as text.
//!
//! Two uses: the options offered to a seat, described as that seat sees
//! them; and a decision taken, narrated to a viewer — the cards it names
//! from hand are shown only when the decision makes them public (a card
//! played, discarded or burned for a vote), never a card moved to the
//! library's bottom or searched out.

use serde::{Deserialize, Serialize};
use zvtes::cards::{AbilitySource, Target};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, WindowLabel, Zone};
use zvtes::game::{Contest, Engine};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{MinionKind, HIDDEN};

use crate::view::{TableView, TargetView};

/// A card an option names: its KRCG id when the viewer may know it, its
/// copy when it is on the table, its arena index when in play.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardRef {
    pub id: Option<u32>,
    pub copy: Option<u32>,
    pub card_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionView {
    pub index: usize,
    /// The `Choice` variant, snake case.
    pub kind: String,
    pub text: String,
    /// The card played, used, taken or named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<CardRef>,
    /// The minion the option is about (blocker, voter, the vampire paying).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minion: Option<u32>,
    /// The minion playing the card or using the ability.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetView>,
    /// The action a minion takes (`take_action`): "bleed", "hunt", …
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favor: Option<bool>,
}

pub fn action_kind(k: ActionKind) -> &'static str {
    match k {
        ActionKind::Bleed => "bleed",
        ActionKind::Hunt => "hunt",
        ActionKind::LeaveTorpor => "leave torpor",
        ActionKind::Rescue => "rescue",
        ActionKind::Diablerie => "diablerie",
        ActionKind::Employ => "employ",
        ActionKind::Recruit => "recruit",
        ActionKind::Political => "political action",
        ActionKind::Card => "action",
        ActionKind::BurnCard => "burn card",
        ActionKind::StealCard => "steal card",
        ActionKind::Equip => "equip",
        ActionKind::EquipFrom => "equip from a minion",
        ActionKind::EquipFound => "equip from the library",
        ActionKind::Ability => "action",
    }
}

/// A window, as a client names it.
pub fn window_text(w: WindowLabel) -> &'static str {
    use WindowLabel::*;
    match w {
        UnlockAbilities => "Unlock phase",
        MasterPhase => "Master phase",
        PlayTerms => "Terms of the play",
        RiderTerms => "Paying an effect",
        PhaseImpulse => "End of phase",
        CardAsPlayed => "As the card is played",
        MinionPhase => "Minion phase",
        ActionAsAnnounced => "As the action is announced",
        ActionBlock => "Block attempts",
        ActionAfterResolution => "After resolution",
        ActionWouldBeSuccessful => "Action would be successful",
        ActionBlocksDeclined => "Blocks declined",
        ActionBlocked => "Action blocked",
        ActionAfterCombat => "After combat",
        TorporBlock => "Blocked leave torpor",
        RescueCost => "Rescue cost",
        EntryEffect => "Entering play",
        ResolutionChoice => "Resolving an effect",
        BlockToll => "Block toll",
        Distribution => "Distributing blood",
        BloodHuntPolling => "Blood hunt",
        ReferendumTerms => "Referendum terms",
        ReferendumPolling => "Referendum",
        DiscardDown => "Discard down to hand size",
        ActionTerms => "Terms of the action",
        LibrarySearch => "Library search",
        ActionExchange => "Exchange",
        Draw => "Draw",
        BottomCards => "Cards to the library's bottom",
        InfluenceTransfers => "Influence phase",
        DiscardPhase => "Discard phase",
        CombatBeforeRange => "Combat: before range",
        CombatDetermineRange => "Combat: range",
        CombatBeforeStrikes => "Combat: before strikes",
        CombatStrike => "Combat: strike",
        CombatBeforeResolution => "Combat: before resolution",
        CombatPrevention => "Combat: damage prevention",
        CombatPreventionAllocation => "Combat: aggravated damage prevented",
        CombatAdditionalStrikes => "Combat: additional strikes",
        CombatPress => "Combat: press",
        CombatEndOfRound => "Combat: end of round",
    }
}

/// What `Pass` means in a window.
fn pass_text(w: WindowLabel) -> &'static str {
    use WindowLabel::*;
    match w {
        MasterPhase => "End the master phase",
        MinionPhase => "End the minion phase",
        InfluenceTransfers => "End the influence phase",
        DiscardPhase => "End the discard phase",
        ActionBlock => "Pass (no block, no play)",
        ReferendumPolling | BloodHuntPolling => "Pass (no vote now)",
        UnlockAbilities => "Pass",
        _ => "Pass",
    }
}

/// Who describes, for whom.
pub struct Describer<'a> {
    pub e: &'a Engine,
    /// The deciding seat.
    pub seat: SeatId,
    pub window: WindowLabel,
    /// The viewer's table (its copies, what it may know).
    pub view: &'a TableView,
    pub names: &'a [String],
    /// Narrating a decision taken: the hand cards it makes public shown.
    pub narrate: bool,
    /// The request's options: what a pass declines.
    pub offered: &'a [Choice],
}

impl Describer<'_> {
    fn seat_name(&self, s: SeatId) -> String {
        self.names
            .get(s.index())
            .cloned()
            .unwrap_or_else(|| format!("Seat {}", s.0 + 1))
    }

    fn minion_name(&self, m: MinionId) -> String {
        let mn = &self.e.state.minions[m.index()];
        match mn.kind {
            MinionKind::Token { .. } => "ghoul token".into(),
            _ => mn.name().into(),
        }
    }

    fn in_play(&self, c: u32) -> CardRef {
        let card = self.e.state.cards[c as usize].card;
        let copy = self
            .view
            .seats
            .iter()
            .flat_map(|s| {
                s.in_play
                    .iter()
                    .chain(s.ready.iter().chain(&s.torpor).flat_map(|m| &m.cards))
            })
            .find(|p| p.card_id == c)
            .map(|p| p.card.copy);
        CardRef {
            id: Some(LIBRARY[card as usize].id),
            copy,
            card_id: Some(c),
        }
    }

    fn in_play_name(&self, c: u32) -> &'static str {
        LIBRARY[self.e.state.cards[c as usize].card as usize].name
    }

    /// A card the deciding seat may play by index (hand, then Fleshforge
    /// Chamber's cards): its name, if the viewer may know it, and ref.
    fn hand(&self, i: u8, public: bool) -> (Option<&'static str>, CardRef) {
        let vh = self.e.virtual_hand(self.seat);
        let (card, from) = vh[i as usize];
        let seat = &self.view.seats[self.seat.index()];
        let shown = match from {
            None => seat.hand.get(i as usize).copied(),
            Some(loc) => seat
                .in_play
                .iter()
                .find(|p| p.card_id == loc.0)
                .and_then(|p| {
                    let k = vh[..i as usize]
                        .iter()
                        .filter(|&&(_, f)| f == Some(loc))
                        .count();
                    p.stored.get(k).copied()
                }),
        };
        let known = shown.is_some_and(|c| c.id.is_some()) || (self.narrate && public);
        let id = known.then_some(LIBRARY[card as usize].id);
        let r = CardRef {
            id,
            copy: shown.map(|c| c.copy),
            card_id: None,
        };
        (known.then_some(LIBRARY[card as usize].name), r)
    }

    fn slot(&self, i: u8) -> (String, CardRef) {
        let seat = &self.view.seats[self.seat.index()];
        let shown = seat.uncontrolled.get(i as usize).map(|s| s.card);
        let known = self.e.state.seats[self.seat.index()]
            .uncontrolled
            .get(i as usize)
            .map(|u| u.card)
            .filter(|_| shown.is_some_and(|c| c.id.is_some()));
        let name = match known {
            Some(c) if c != HIDDEN => CRYPT[c as usize].name.to_string(),
            _ => format!("uncontrolled vampire #{}", i + 1),
        };
        let r = CardRef {
            id: shown.and_then(|c| c.id),
            copy: shown.map(|c| c.copy),
            card_id: None,
        };
        (name, r)
    }

    fn target_text(&self, t: Target) -> Option<String> {
        match t {
            Target::None => None,
            Target::Minion(m) => Some(self.minion_name(m)),
            Target::Seat(s) => Some(self.seat_name(s)),
            Target::Uncontrolled(i) => Some(self.slot(i).0),
            Target::Card(c) => Some(self.in_play_name(c.0).to_string()),
        }
    }

    fn contest(&self, c: Contest) -> String {
        match c {
            Contest::Card(m) => self.minion_name(m),
            Contest::Title(m) => format!("the title of {}", self.minion_name(m)),
            Contest::Library(id) => self.in_play_name(id.0).to_string(),
        }
    }

    fn votes(favor: bool) -> &'static str {
        if favor {
            "for"
        } else {
            "against"
        }
    }

    /// One option.
    pub fn describe(&self, index: usize, c: &Choice) -> OptionView {
        let mut o = OptionView {
            index,
            kind: kind(c).into(),
            text: String::new(),
            card: None,
            minion: None,
            actor: None,
            target: None,
            action: None,
            amount: None,
            favor: None,
        };
        let card_named = |name: Option<&str>| name.unwrap_or("a card").to_string();
        o.text = match *c {
            // A distribution walks the recipients: the pass skips this one.
            Choice::Pass if self.window == WindowLabel::Distribution => {
                match self.offered.iter().find_map(|c| match c {
                    Choice::ChooseMinion(m) => Some(*m),
                    _ => None,
                }) {
                    Some(m) => format!("No blood to {}", self.minion_name(m)),
                    None => "Pass".into(),
                }
            }
            Choice::Pass => pass_text(self.window).into(),
            Choice::BurnEdge => "Burn the Edge for 1 pool".into(),
            Choice::PayContest(x) => format!("Pay to keep contesting {}", self.contest(x)),
            Choice::YieldContest(x) => format!("Yield {}", self.contest(x)),
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                let (name, r) = self.hand(card, true);
                o.card = Some(r);
                o.actor = actor.map(|m| m.0);
                o.target = TargetView::of(target);
                let played = self.e.virtual_hand(self.seat)[card as usize].0;
                let mut s = format!("Play {}", card_named(name));
                if name.is_some() {
                    s += &mode_text(played, mode);
                }
                if let Some(m) = actor {
                    s += &format!(" with {}", self.minion_name(m));
                }
                if let Some(t) = self.target_text(target) {
                    s += &format!(" on {t}");
                }
                s
            }
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                o.actor = actor.map(|m| m.0);
                o.target = TargetView::of(target);
                let name = match source {
                    AbilitySource::Card(id) => {
                        o.card = Some(self.in_play(id.0));
                        self.in_play_name(id.0).to_string()
                    }
                    AbilitySource::Vampire(m) => {
                        o.minion = Some(m.0);
                        self.minion_name(m)
                    }
                };
                let mut s = format!("Use {name}");
                if ability > 0 {
                    s += &format!(" (ability {})", ability + 1);
                }
                if let Some(m) = actor {
                    s += &format!(" with {}", self.minion_name(m));
                }
                if let Some(t) = self.target_text(target) {
                    s += &format!(" on {t}");
                }
                s
            }
            Choice::ResolveRider { card } => {
                o.card = Some(self.in_play(card));
                format!("Resolve {}", self.in_play_name(card))
            }
            Choice::ResolvePrinted { minion } => {
                o.minion = Some(minion.0);
                format!("Resolve {}'s effect", self.minion_name(minion))
            }
            Choice::BurnInPlay { card } => {
                o.card = Some(self.in_play(card));
                format!("Burn {}", self.in_play_name(card))
            }
            Choice::BurnPool => "Burn 1 pool".into(),
            Choice::BurnBloodFrom { minion } => {
                o.minion = Some(minion.0);
                format!("Burn 1 blood from {}", self.minion_name(minion))
            }
            Choice::SetX(x) => {
                o.amount = Some(x);
                format!("X = {x}")
            }
            Choice::Discard(i) => {
                let (name, r) = self.hand(i, true);
                o.card = Some(r);
                format!("Discard {}", card_named(name))
            }
            Choice::FindCard(card) => {
                let public = !self.narrate;
                o.card = Some(CardRef {
                    id: public.then_some(LIBRARY[card as usize].id),
                    copy: None,
                    card_id: None,
                });
                if public {
                    format!("Take {}", LIBRARY[card as usize].name)
                } else {
                    "Take a card".into()
                }
            }
            Choice::TakeFrom { zone, card } => {
                let public = !self.narrate || zone == Zone::AshHeap;
                o.card = Some(CardRef {
                    id: public.then_some(LIBRARY[card as usize].id),
                    copy: None,
                    card_id: None,
                });
                let from = match zone {
                    Zone::Library => "the library",
                    Zone::Hand => "the hand",
                    Zone::AshHeap => "the ash heap",
                };
                let name = if public {
                    LIBRARY[card as usize].name
                } else {
                    "a card"
                };
                format!("Take {name} from {from}")
            }
            Choice::Slot(i) => {
                let (name, r) = self.slot(i);
                o.card = Some(r);
                format!("Choose {name}")
            }
            Choice::AshVampire(card) => {
                o.card = Some(CardRef {
                    id: Some(CRYPT[card as usize].id),
                    copy: None,
                    card_id: None,
                });
                format!("Choose {} in the ash heap", CRYPT[card as usize].name)
            }
            Choice::DrawLibrary => "Draw from the library".into(),
            Choice::DrawStored { card, stored } => {
                o.card = Some(self.in_play(card));
                let public = !self.narrate;
                let name = if public {
                    LIBRARY[stored as usize].name
                } else {
                    "a card"
                };
                format!("Draw {name} from {}", self.in_play_name(card))
            }
            Choice::Bottom(i) => {
                let (name, r) = self.hand(i, false);
                o.card = Some(r);
                format!("Put {} on the bottom of the library", card_named(name))
            }
            Choice::TakeEquipment { card } => {
                o.card = Some(self.in_play(card));
                format!("Take {}", self.in_play_name(card))
            }
            Choice::TakeAction {
                minion,
                action,
                target,
            } => {
                o.minion = Some(minion.0);
                o.action = Some(action_kind(action).into());
                o.target = target.map(|m| TargetView::Minion { minion: m.0 });
                let mut s = format!(
                    "{} with {}",
                    capitalize(action_kind(action)),
                    self.minion_name(minion)
                );
                if let Some(t) = target {
                    s += &format!(" ({})", self.minion_name(t));
                }
                s
            }
            Choice::DeclareBlock { minion } => {
                o.minion = Some(minion.0);
                format!("Block with {}", self.minion_name(minion))
            }
            Choice::PayRescue { from_acting } => {
                o.amount = Some(from_acting);
                format!("The rescuer pays {from_acting} blood")
            }
            Choice::CastVotes { minion, favor } => {
                o.minion = Some(minion.0);
                o.favor = Some(favor);
                format!(
                    "{}'s votes {}",
                    self.minion_name(minion),
                    Self::votes(favor)
                )
            }
            Choice::CastEdge { favor } => {
                o.favor = Some(favor);
                format!("Burn the Edge: 1 vote {}", Self::votes(favor))
            }
            Choice::CastCardVote { favor } => {
                o.favor = Some(favor);
                format!("The political action card's vote {}", Self::votes(favor))
            }
            Choice::BurnForVote { card, favor } => {
                let (name, r) = self.hand(card, true);
                o.card = Some(r);
                o.favor = Some(favor);
                format!(
                    "Burn {} for 1 vote {}",
                    card_named(name),
                    Self::votes(favor)
                )
            }
            Choice::AllocateTo { seat, n } => {
                o.target = Some(TargetView::Seat { seat: seat.0 });
                o.amount = Some(n);
                if self.window == WindowLabel::Distribution {
                    format!("{n} to your pool")
                } else {
                    // Who is allocated to next: the last one gets the rest.
                    let next: Vec<SeatId> = self
                        .e
                        .referendum()
                        .map(|r| r.terms.pending.iter().skip(1).copied().collect())
                        .unwrap_or_default();
                    let rest = match next.as_slice() {
                        [] => String::new(),
                        [last] => format!(" — the rest goes to {}", self.seat_name(*last)),
                        more => format!(
                            " — then {}",
                            more.iter()
                                .map(|s| self.seat_name(*s))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    };
                    format!("Allocate {n} to {}{rest}", self.seat_name(seat))
                }
            }
            Choice::ChooseSeat(s) => {
                o.target = Some(TargetView::Seat { seat: s.0 });
                format!("Choose {}", self.seat_name(s))
            }
            Choice::ChooseMinion(m) => {
                o.target = Some(TargetView::Minion { minion: m.0 });
                if self.window == WindowLabel::Distribution {
                    format!("1 blood to {}", self.minion_name(m))
                } else {
                    format!("Choose {}", self.minion_name(m))
                }
            }
            Choice::ChooseClan(clan) => format!("Choose {clan}"),
            Choice::ChooseCard(card) => {
                o.card = Some(self.in_play(card));
                o.target = Some(TargetView::Card { card_id: card });
                format!("Choose {}", self.in_play_name(card))
            }
            Choice::AllocateToCard { card, n } => {
                o.card = Some(self.in_play(card));
                o.amount = Some(n);
                format!("Allocate {n} to {}", self.in_play_name(card))
            }
            Choice::Transfer { slot } => {
                let (name, r) = self.slot(slot);
                o.card = Some(r);
                format!("Transfer 1 pool to {name}")
            }
            Choice::Withdraw { slot } => {
                let (name, r) = self.slot(slot);
                o.card = Some(r);
                format!("Move 1 blood from {name} back to pool")
            }
            Choice::DrawCrypt => "Draw a crypt card (4 transfers)".into(),
            Choice::InfluenceOut { slot } => {
                let (name, r) = self.slot(slot);
                o.card = Some(r);
                format!("Influence out {name}")
            }
            Choice::HandStrike => "Strike: hand strike".into(),
            Choice::UseManeuver => "Use a maneuver".into(),
            Choice::UsePress => "Use a press".into(),
            Choice::UsePrevent => "Prevent 1 damage".into(),
            Choice::PayBlockToll => "Pay the block toll".into(),
            Choice::PayToCancel => "Pay to cancel the card".into(),
            Choice::StrikeCombatEnds => "Strike: combat ends".into(),
            Choice::RoundStrike { at } => match at {
                Some(id) => {
                    o.card = Some(self.in_play(id.0));
                    format!("Strike at {}", self.in_play_name(id.0))
                }
                None => "Strike (the round's strike)".into(),
            },
            Choice::TakeDamageAfterResolution => "Take the damage".into(),
            Choice::CommittedStrike => "Strike: the committed strike".into(),
            Choice::BurnWeaponStrike { weapon } => {
                o.card = Some(self.in_play(weapon));
                format!("Strike: burn {}", self.in_play_name(weapon))
            }
            Choice::EmployFromHand { card, mode } => {
                o.card = Some(CardRef {
                    id: Some(LIBRARY[card as usize].id),
                    copy: None,
                    card_id: None,
                });
                format!(
                    "Employ {}{}",
                    LIBRARY[card as usize].name,
                    mode_text(card, mode)
                )
            }
            Choice::BurnBloodForIntercept { minion } => {
                o.minion = Some(minion.0);
                format!(
                    "{} burns 1 blood for +1 intercept",
                    self.minion_name(minion)
                )
            }
            Choice::UnlockForBlood { minion } => {
                o.minion = Some(minion.0);
                format!("{} burns blood to unlock", self.minion_name(minion))
            }
            Choice::ContinueAction => "Continue the action as if unblocked".into(),
            Choice::GainStrength => "Gain the strength".into(),
            Choice::BurnBloodToAvoid => "Burn blood to avoid the damage".into(),
            Choice::BurnCorruptionToFail { minion } => {
                o.minion = Some(minion.0);
                format!(
                    "Burn a corruption counter from {}: its block fails",
                    self.minion_name(minion)
                )
            }
            Choice::BurnCorruptionToUnlock { minion } => {
                o.minion = Some(minion.0);
                format!(
                    "Burn 2 corruption counters from {} to unlock",
                    self.minion_name(minion)
                )
            }
            Choice::Diablerise => "Diablerize the acting vampire".into(),
        };
        o
    }
}

/// The mode a card is played in: the level and Disciplines of its printed
/// line ("at superior [DOM]"), and its rank when the definition splits that
/// line into several modes (by effect or by target).
fn mode_text(card: u16, mode: u8) -> String {
    let lib = &LIBRARY[card as usize];
    let mode = mode as usize;
    let Some(def) = zvtes::cards::lookup(card) else {
        return String::new();
    };
    if mode >= def.modes.len() {
        return String::new();
    }
    let printed = zvtes::cards::library_mode(card, mode);
    let tags = lib.modes.get(printed).map_or(&[][..], |m| m.tags);
    let mut s = String::new();
    if !tags.is_empty() {
        let upper = |t: &&str| !t.chars().any(|c| c.is_lowercase());
        let level = if tags.iter().all(upper) {
            "superior "
        } else if !tags.iter().any(upper) {
            "inferior "
        } else {
            ""
        };
        s = format!(" at {level}[{}]", tags.join("]["));
    }
    let siblings: Vec<usize> = (0..def.modes.len())
        .filter(|&i| zvtes::cards::library_mode(card, i) == printed)
        .collect();
    if siblings.len() > 1 {
        let rank = siblings.iter().position(|&i| i == mode).unwrap_or(0);
        s += &format!(" (option {})", rank + 1);
    }
    s
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// The variant's name, snake case.
pub fn kind(c: &Choice) -> &'static str {
    match c {
        Choice::Pass => "pass",
        Choice::BurnEdge => "burn_edge",
        Choice::PayContest(_) => "pay_contest",
        Choice::YieldContest(_) => "yield_contest",
        Choice::Play { .. } => "play",
        Choice::UseAbility { .. } => "use_ability",
        Choice::ResolveRider { .. } => "resolve_rider",
        Choice::ResolvePrinted { .. } => "resolve_printed",
        Choice::BurnInPlay { .. } => "burn_in_play",
        Choice::BurnPool => "burn_pool",
        Choice::BurnBloodFrom { .. } => "burn_blood_from",
        Choice::SetX(_) => "set_x",
        Choice::Discard(_) => "discard",
        Choice::FindCard(_) => "find_card",
        Choice::TakeFrom { .. } => "take_from",
        Choice::Slot(_) => "slot",
        Choice::AshVampire(_) => "ash_vampire",
        Choice::DrawLibrary => "draw_library",
        Choice::DrawStored { .. } => "draw_stored",
        Choice::Bottom(_) => "bottom",
        Choice::TakeEquipment { .. } => "take_equipment",
        Choice::TakeAction { .. } => "take_action",
        Choice::DeclareBlock { .. } => "declare_block",
        Choice::PayRescue { .. } => "pay_rescue",
        Choice::CastVotes { .. } => "cast_votes",
        Choice::CastEdge { .. } => "cast_edge",
        Choice::CastCardVote { .. } => "cast_card_vote",
        Choice::BurnForVote { .. } => "burn_for_vote",
        Choice::AllocateTo { .. } => "allocate_to",
        Choice::ChooseSeat(_) => "choose_seat",
        Choice::ChooseMinion(_) => "choose_minion",
        Choice::ChooseClan(_) => "choose_clan",
        Choice::ChooseCard(_) => "choose_card",
        Choice::AllocateToCard { .. } => "allocate_to_card",
        Choice::Transfer { .. } => "transfer",
        Choice::Withdraw { .. } => "withdraw",
        Choice::DrawCrypt => "draw_crypt",
        Choice::InfluenceOut { .. } => "influence_out",
        Choice::HandStrike => "hand_strike",
        Choice::UseManeuver => "use_maneuver",
        Choice::UsePress => "use_press",
        Choice::UsePrevent => "use_prevent",
        Choice::PayBlockToll => "pay_block_toll",
        Choice::PayToCancel => "pay_to_cancel",
        Choice::StrikeCombatEnds => "strike_combat_ends",
        Choice::RoundStrike { .. } => "round_strike",
        Choice::TakeDamageAfterResolution => "take_damage_after_resolution",
        Choice::CommittedStrike => "committed_strike",
        Choice::BurnWeaponStrike { .. } => "burn_weapon_strike",
        Choice::EmployFromHand { .. } => "employ_from_hand",
        Choice::BurnBloodForIntercept { .. } => "burn_blood_for_intercept",
        Choice::UnlockForBlood { .. } => "unlock_for_blood",
        Choice::ContinueAction => "continue_action",
        Choice::GainStrength => "gain_strength",
        Choice::BurnBloodToAvoid => "burn_blood_to_avoid",
        Choice::BurnCorruptionToFail { .. } => "burn_corruption_to_fail",
        Choice::BurnCorruptionToUnlock { .. } => "burn_corruption_to_unlock",
        Choice::Diablerise => "diablerise",
    }
}
