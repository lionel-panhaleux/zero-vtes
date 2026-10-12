//! Option descriptors (docs/human-play.md): every `Choice` as structured
//! data (what clients highlight and match gestures against) and as text.
//!
//! Two uses: the options offered to a seat, described as that seat sees
//! them; and a decision taken, narrated to a viewer — the cards it names
//! from hand are shown only when the decision makes them public (a card
//! played, discarded or burned for a vote), never a card moved to the
//! library's bottom or searched out.

use serde::{Deserialize, Serialize};
use zvtes::cards::{AbilitySource, Effect, Modifier, RefEffect, Target, Term, Terms};
use zvtes::crypt::CRYPT;
use zvtes::decision::{ActionKind, Choice, WindowLabel, Zone};
use zvtes::game::{Contest, Engine};
use zvtes::ids::{MinionId, SeatId};
use zvtes::library::LIBRARY;
use zvtes::state::{CardId, MinionKind, HIDDEN};

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
    /// The mode a card is played in (`play`), when the viewer knows the
    /// card: its index and its printed tags (`dom` basic, `DOM` superior,
    /// `COMBAT`…; empty for an untagged card).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ModeView>,
    /// The blood an ability moves onto (+) or off (−) the vampire it is
    /// on (`minion`): Blood Doll's two abilities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blood: Option<i8>,
    /// Where a card chosen by name is taken from, and whose (a search: a
    /// library, an ash heap, a hand): the table shows it there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<ZoneRef>,
}

/// A seat's zone: "library", "ash_heap", "hand".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneRef {
    pub seat: u8,
    pub zone: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeView {
    pub index: u8,
    pub tags: Vec<String>,
    /// The printed line it is played from: one line split into several
    /// modes (`cards::library_mode`) is one choice of Disciplines.
    #[serde(default)]
    pub line: u8,
    /// What sets it apart from the other modes of its line ("gain 3
    /// blood"), when the definition splits the line and its effects say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// What its target is for, when the play chooses another minion after
    /// it ("the Toreador to lock, who does not unlock as normal": Toreador
    /// Grand Ball), so the target clicked on the table is not ambiguous.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
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
        ActionKind::BecomeAnarch => "become Anarch",
        ActionKind::RedListHunt => "enter combat with a Red List minion",
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
        ActionBlock => "Pass (no block)",
        CombatPress => "Pass (no press)",
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
    /// The card whose terms a terms decision chooses: narrated as its
    /// prefix ("Kine Resources Contested: 3 to Bot 2").
    pub about: Option<String>,
}

impl Describer<'_> {
    pub(crate) fn seat_name(&self, s: SeatId) -> String {
        self.names
            .get(s.index())
            .cloned()
            .unwrap_or_else(|| format!("Seat {}", s.0 + 1))
    }

    pub(crate) fn minion_name(&self, m: MinionId) -> String {
        let mn = &self.e.state.minions[m.index()];
        match mn.kind {
            MinionKind::Token { .. } => "ghoul token".into(),
            _ => mn.name().into(),
        }
    }

    pub(crate) fn in_play(&self, c: u32) -> CardRef {
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
            .map(|p| p.card.copy)
            // An ally is on the table as its minion (Screamer's burn).
            .or_else(|| {
                self.view
                    .seats
                    .iter()
                    .flat_map(|s| s.ready.iter().chain(&s.torpor))
                    .find(|m| self.e.state.minions[m.minion as usize].ally_id() == Some(CardId(c)))
                    .map(|m| m.card.copy)
            });
        CardRef {
            id: Some(LIBRARY[card as usize].id),
            copy,
            card_id: Some(c),
        }
    }

    pub(crate) fn in_play_name(&self, c: u32) -> &'static str {
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

    pub(crate) fn slot(&self, i: u8) -> (String, CardRef) {
        self.slot_of(self.seat, i)
    }

    /// Slot `i` of `of`'s uncontrolled region (Solomon Batanea's pick in
    /// his prey's).
    fn slot_of(&self, of: SeatId, i: u8) -> (String, CardRef) {
        let seat = &self.view.seats[of.index()];
        let shown = seat.uncontrolled.get(i as usize).map(|s| s.card);
        let known = self.e.state.seats[of.index()]
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
            Target::Card(c) => Some(self.hosted_name(c.0)),
        }
    }

    /// A card in play with the minion it is on, if any ("Luciano's Feral
    /// Hound"): which of two copies.
    pub(crate) fn hosted_name(&self, c: u32) -> String {
        let name = self.in_play_name(c);
        match self.e.state.cards.get(c as usize).and_then(|k| k.host) {
            Some(m) => format!("{}'s {name}", self.minion_name(m)),
            None => name.to_string(),
        }
    }

    /// Where a card named by `Choice::FindCard` is: a library search, the
    /// ash heap of an exchange (Lenelle) or of the prey (The Erciyes
    /// Fragments), the hand looked at.
    fn found_in(&self) -> Option<ZoneRef> {
        use zvtes::draws::Interrupt;
        let at = |seat: SeatId, zone: &str| {
            Some(ZoneRef {
                seat: seat.0,
                zone: zone.into(),
            })
        };
        match (self.window, self.e.interrupt()) {
            (WindowLabel::LibrarySearch, _) => at(self.seat, "library"),
            (WindowLabel::ActionExchange, _) => at(self.seat, "ash_heap"),
            (_, Some(Interrupt::DiscardFromHandOf { of, .. })) => at(of, "hand"),
            (_, Some(Interrupt::StorePreyAsh { seat, .. })) => {
                at(self.e.state.prey(seat), "ash_heap")
            }
            _ => None,
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

    /// The referendum term being chosen, what the referendum does and the
    /// Methuselah chosen so far (a shortfall re-allocated at resolution:
    /// the last term).
    fn ref_term(&self) -> Option<(Term, RefEffect, Option<SeatId>)> {
        let r = self.e.referendum()?;
        let def = r.def?;
        let term = def.terms.get(r.terms.term).or(def.terms.last())?;
        Some((*term, def.effect, r.terms.chosen))
    }

    /// What choosing `c` does, when the terms choose several things of a
    /// kind: a referendum's ("gains 1 pool", "burns 2 pool"), a play's
    /// second vampire (Toreador Grand Ball's unblockable one).
    fn role(&self, c: &Choice) -> Option<String> {
        if self.window == WindowLabel::PlayTerms {
            let p = self.e.play_terms()?;
            return match c {
                Choice::ChooseMinion(_) => second_chosen(p.card, p.mode).map(|(_, r)| r),
                _ => None,
            };
        }
        if self.window != WindowLabel::ReferendumTerms {
            return None;
        }
        let (term, effect, chosen) = self.ref_term()?;
        let chosen = || chosen.map_or("the chosen Methuselah".into(), |s| self.seat_name(s));
        use RefEffect::*;
        Some(match (*c, term, effect) {
            (Choice::ChooseSeat(_), Term::ChooseSeat(_), BurnAllocatedChosenGains(n)) => {
                format!("gains {n} pool")
            }
            (Choice::ChooseSeat(_), Term::ChooseSeat(_), MoveAllocatedFromChosen) => {
                "gives the pool".into()
            }
            (Choice::ChooseSeat(_), Term::ChooseSeat(_), ChosenTakesChosenCard) => {
                "takes control of the card".into()
            }
            (Choice::ChooseCard(_), _, ChosenTakesChosenCard) => "changes control".into(),
            (Choice::ChooseSeat(_), Term::ChooseSeats { .. }, BurnChosen { .. }) => {
                "burns pool".into()
            }
            (
                Choice::AllocateTo { n, .. },
                _,
                BurnAllocated | BurnAllocatedChosenGains(_) | LockChosenBurnAllocated,
            ) => format!("burns {n} pool"),
            (Choice::AllocateTo { n, .. }, _, MoveAllocatedFromChosen) => {
                format!("gains {n} pool from {}", chosen())
            }
            (Choice::AllocateToCard { n, .. }, _, LockChosenBurnAllocated) if n > 0 => {
                "burned".into()
            }
            (Choice::ChooseMinion(_), _, Banish) => "banished".into(),
            (Choice::ChooseMinion(_), _, Expel) => "no reactions, blocks or votes this turn".into(),
            (Choice::ChooseMinion(_), _, PutOnChosen) => "gets the card".into(),
            (Choice::ChooseMinion(_), _, LockChosenBurnAllocated) => "locked".into(),
            _ => return None,
        })
    }

    /// What the decision is for, when its options alone do not say: a
    /// referendum's term, a play's second vampire — after the card's name
    /// ("Camarilla's Iron Fist: choose the Methuselah who gains 1 pool").
    pub fn prompt(&self) -> Option<String> {
        if self.e.interrupt().is_some() {
            return self.purpose_prompt();
        }
        self.terms_prompt().or_else(|| self.purpose_prompt())
    }

    fn terms_prompt(&self) -> Option<String> {
        let (card, what) = match self.window {
            WindowLabel::PlayTerms => {
                let p = self.e.play_terms()?;
                let (who, _) = second_chosen(p.card, p.mode)?;
                (
                    Some(LIBRARY[p.card as usize].name.to_string()),
                    format!("choose {who}"),
                )
            }
            WindowLabel::ReferendumTerms => {
                let (term, effect, chosen) = self.ref_term()?;
                let chosen = chosen.map_or("the chosen Methuselah".into(), |s| self.seat_name(s));
                use RefEffect::*;
                let what = match (term, effect) {
                    (Term::ChooseSeat(_), BurnAllocatedChosenGains(n)) => {
                        format!("choose the Methuselah who gains {n} pool")
                    }
                    (Term::ChooseSeat(_), MoveAllocatedFromChosen) => {
                        "choose the Methuselah whose pool is given away".into()
                    }
                    (Term::ChooseSeat(_), ChosenTakesChosenCard) => {
                        "choose the Methuselah who takes control of the card".into()
                    }
                    (Term::ChooseCard(_), ChosenTakesChosenCard) => {
                        "choose the card that changes control".into()
                    }
                    (
                        Term::ChooseSeats { .. },
                        BurnChosen {
                            base,
                            young_bonus,
                            young_cap,
                        },
                    ) => format!(
                        "choose the Methuselahs who burn {base} pool ({} if they control a \
                         ready vampire of capacity {young_cap} or less)",
                        base + young_bonus
                    ),
                    (Term::Allocate { .. }, BurnAllocated | BurnAllocatedChosenGains(_)) => {
                        "allocate the points: each Methuselah burns 1 pool per point".into()
                    }
                    (Term::Allocate { .. }, MoveAllocatedFromChosen) => format!(
                        "allocate {chosen}'s pool: each Methuselah gains 1 pool per point, \
                         from {chosen}"
                    ),
                    (Term::AllocatePerChosenWithCards { .. }, LockChosenBurnAllocated) => {
                        "allocate the points: each Methuselah burns 1 pool per point, each \
                         card given a point is burned"
                            .into()
                    }
                    (Term::ChooseMinions { .. }, Banish) => "choose the vampire to banish".into(),
                    (Term::ChooseMinions { max, .. }, Expel) => format!(
                        "choose up to {max} minions that cannot play reaction cards, block or \
                         cast votes this turn"
                    ),
                    (Term::ChooseMinions { .. }, PutOnChosen) => {
                        "choose the vampire who gets the card".into()
                    }
                    (Term::ChooseMinions { .. }, LockChosenBurnAllocated) => {
                        "choose the minions to lock".into()
                    }
                    (Term::ChooseClan, GainPerClanVampire) => {
                        "choose a clan: each Methuselah gains 1 pool per vampire of that clan \
                         they control"
                            .into()
                    }
                    _ => return None,
                };
                (self.about.clone(), what)
            }
            _ => return None,
        };
        Some(match card {
            Some(c) => format!("{c}: {what}"),
            None => capitalize(&what),
        })
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
            mode: None,
            blood: None,
            zone: None,
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
            // In the block window, short and explicit: the defender's pass
            // blocks only while its minion's attempt meets the stealth;
            // anyone else's is plain.
            Choice::Pass if self.window == WindowLabel::ActionBlock => {
                use zvtes::actions::ActionState;
                match self.e.action() {
                    Some(a) if a.seat != self.seat => match a.state {
                        // An attempt short of the stealth fails on a pass.
                        ActionState::Attempt(m)
                            if self.e.state.minions[m.index()].owner == self.seat =>
                        {
                            if a.intercept_of(&self.e.state, m) >= a.stealth(&self.e.state) {
                                "Pass (blocking)".into()
                            } else {
                                "Pass (no block)".into()
                            }
                        }
                        ActionState::Attempt(_) => "Pass".into(),
                        _ => "Pass (no block)".into(),
                    },
                    _ => "Pass".into(),
                }
            }
            Choice::Pass => pass_text(self.window).into(),
            Choice::BurnEdge => "Gain 1 pool (the Edge)".into(),
            // The card or minion contested: its buttons go under it.
            Choice::PayContest(x) | Choice::YieldContest(x) => {
                match x {
                    Contest::Card(m) | Contest::Title(m) => o.minion = Some(m.0),
                    Contest::Library(id) => o.card = Some(self.in_play(id.0)),
                }
                match c {
                    Choice::PayContest(_) => format!("Pay to keep contesting {}", self.contest(x)),
                    _ => format!("Yield {}", self.contest(x)),
                }
            }
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
                    o.mode = Some(mode_view(played, mode));
                }
                if let Some(m) = actor {
                    s += &format!(" with {}", self.minion_name(m));
                }
                let second = name.and(second_chosen(played, mode));
                if let Some(t) = self.target_text(target) {
                    // Two vampires chosen: what happens to which (Toreador
                    // Grand Ball locks its target).
                    if second.is_some() && mode_has(played, mode, Effect::LockTarget) {
                        s += &format!(": lock {t}");
                    } else {
                        s += &format!(" on {t}");
                    }
                } else if bleeds(played, mode) {
                    // A bleed's target: the prey, unless a card says otherwise.
                    s += &format!(" → {}", self.seat_name(self.e.state.prey(self.seat)));
                }
                if let Some((who, _)) = second {
                    s += &format!(" (then choose {who})");
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
                let (name, abilities, host) = match source {
                    AbilitySource::Card(id) => {
                        o.card = Some(self.in_play(id.0));
                        let c = &self.e.state.cards[id.index()];
                        // The vampire it is on: which of two copies.
                        let host = c.host;
                        o.minion = host.map(|m| m.0);
                        let name = match host {
                            Some(m) => {
                                format!("{}'s {}", self.minion_name(m), self.in_play_name(id.0))
                            }
                            None => self.in_play_name(id.0).to_string(),
                        };
                        let abilities =
                            zvtes::cards::lookup(c.card).map_or(&[][..], |d| d.abilities);
                        (name, abilities, host)
                    }
                    AbilitySource::Vampire(m) => {
                        o.minion = Some(m.0);
                        let v = &self.e.state.minions[m.index()];
                        let abilities =
                            zvtes::cards::vampire_of(v).map_or(&[][..], |d| d.abilities);
                        (self.minion_name(m), abilities, None)
                    }
                };
                let mut s = format!("Use {name}");
                // What it does, after who and on what.
                let mut what = String::new();
                let moved = abilities.get(ability as usize).and_then(|a| {
                    a.effects.iter().find_map(|e| match *e {
                        Effect::HostBloodToPool(n) => Some(-(n as i8)),
                        Effect::PoolToHostBlood(n) => Some(n as i8),
                        _ => None,
                    })
                });
                match moved {
                    Some(n) if n < 0 => what = format!(": {} blood to pool", -n),
                    Some(n) => what = format!(": {n} pool to blood"),
                    // Which of several: what it does (Wider View's draw
                    // or pool, Dreams of the Sphinx's hand size or pool).
                    None if abilities.len() > 1 || ability > 0 => {
                        let told = abilities_told(abilities);
                        let label = |i: usize| told.get(i).cloned();
                        // The same label for the same effects: either does
                        // the same (The Louvre, Paris's two locks).
                        let effects = |i: usize| abilities.get(i).map(|a| a.effects);
                        match label(ability as usize).filter(|l| !l.is_empty()) {
                            Some(l)
                                if (0..abilities.len())
                                    .filter(|&i| effects(i) != effects(ability as usize))
                                    .all(|i| label(i).as_deref() != Some(l.as_str())) =>
                            {
                                what = format!(": {l}")
                            }
                            _ => what = format!(" (ability {})", ability + 1),
                        }
                    }
                    // A lone ability that is an action: which (Toreador
                    // Grand Ball's burn).
                    None => {
                        let a = abilities.get(ability as usize);
                        if a.is_some_and(|a| {
                            a.effects
                                .iter()
                                .any(|e| matches!(e, Effect::DeclareAction(_)))
                        }) {
                            let l = a.map(|a| ability_label(a.effects)).unwrap_or_default();
                            if !l.is_empty() {
                                what = format!(": {l}");
                            }
                        }
                    }
                }
                o.blood = moved;
                if let Some(m) = actor.filter(|&m| Some(m) != host) {
                    s += &format!(" with {}", self.minion_name(m));
                }
                if let Some(t) = self.target_text(target) {
                    s += &format!(" on {t}");
                }
                s += &what;
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
                format!("Burn {}", self.hosted_name(card))
            }
            Choice::BurnPool => "Burn 1 pool".into(),
            Choice::BurnBloodFrom { minion } => {
                o.minion = Some(minion.0);
                format!("Burn 1 blood from {}", self.minion_name(minion))
            }
            Choice::SetX(x) => {
                o.amount = Some(x);
                match self.e.referendum().and_then(|r| r.pressure.as_ref()) {
                    // Business Pressure: how much pool to burn.
                    Some(_) if x == 0 => "Burn no pool".to_string(),
                    Some(p) => format!("Burn {x} pool for {} votes", x * p.rate(self.seat)),
                    None => format!("X = {x}"),
                }
            }
            Choice::Discard(i) => {
                let (name, r) = self.hand(i, true);
                o.card = Some(r);
                format!("Discard {}", card_named(name))
            }
            Choice::FindCard(card) => {
                o.zone = self.found_in();
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
                o.zone = Some(ZoneRef {
                    seat: self.seat.0,
                    zone: match zone {
                        Zone::Library => "library",
                        Zone::Hand => "hand",
                        Zone::AshHeap => "ash_heap",
                    }
                    .into(),
                });
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
                let of = match self.e.interrupt() {
                    Some(zvtes::draws::Interrupt::PreySlotToPool { of, .. }) => of,
                    _ => self.seat,
                };
                let (name, r) = self.slot_of(of, i);
                o.card = Some(r);
                format!("Choose {name}")
            }
            Choice::AshVampire(card) => {
                // Disciple of Lazarus's search takes it from the crypt.
                let crypt = matches!(
                    self.e.interrupt(),
                    Some(zvtes::draws::Interrupt::CryptSearch { .. })
                );
                let from = if crypt { "crypt" } else { "ash_heap" };
                o.zone = Some(ZoneRef {
                    seat: self.seat.0,
                    zone: from.into(),
                });
                o.card = Some(CardRef {
                    id: Some(CRYPT[card as usize].id),
                    copy: None,
                    card_id: None,
                });
                let from = if crypt { "your crypt" } else { "the ash heap" };
                format!("Choose {} in {from}", CRYPT[card as usize].name)
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
                // The votes it casts.
                o.amount = self
                    .e
                    .referendum()
                    .map(|r| r.votes_cast(&self.e.state, minion, favor));
                let mut s = format!(
                    "{}'s votes {}",
                    self.minion_name(minion),
                    Self::votes(favor)
                );
                if let Some(n) = o.amount {
                    s += &format!(" ({n})");
                }
                // A priscus casts its prisci ballot too, even with no vote
                // (rules-qa 52.9).
                if self.e.state.title_of(minion) == zvtes::crypt::Title::Priscus {
                    s += " and a prisci ballot";
                }
                s
            }
            Choice::CastEdge { favor } => {
                o.favor = Some(favor);
                format!("Burn the Edge: 1 vote {}", Self::votes(favor))
            }
            Choice::CastCardVote { favor } => {
                o.favor = Some(favor);
                match self.e.referendum().and_then(|r| r.pressure.as_ref()) {
                    // Business Pressure's round.
                    Some(p) => {
                        let n = p.amount.unwrap_or(0) * p.rate(self.seat);
                        format!("The {n} votes {}", Self::votes(favor))
                    }
                    None => format!("The political action card's vote {}", Self::votes(favor)),
                }
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
                    // Who is allocated to next: the last one gets the rest
                    // (for the one allocating, not in a narration).
                    let next: Vec<SeatId> = self
                        .e
                        .referendum()
                        .filter(|_| !self.narrate)
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
                    let role = self.role(c).map_or(String::new(), |r| format!(" ({r})"));
                    format!("Allocate {n} to {}{role}{rest}", self.seat_name(seat))
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
                format!("Choose {}", self.hosted_name(card))
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
            // In an action: Enthrall's "burn 1 blood to draw 1 card from
            // your crypt".
            Choice::DrawCrypt if self.e.action().is_some() => {
                "Burn 1 blood to draw a crypt card".into()
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
                // The strike a card granted for the round (Hunger of
                // Marduk: "steal 2 blood, ranged").
                None => match self.e.combat() {
                    Some(c) => match c.step {
                        zvtes::combat::Step::ChooseStrike(i) => match c.sides[i].round_strike {
                            Some(k) => format!("Strike: {}", crate::narrate::strike_text(Some(k))),
                            None => "Strike: the round's strike".into(),
                        },
                        _ => "Strike: the round's strike".into(),
                    },
                    None => "Strike: the round's strike".into(),
                },
            },
            Choice::TakeDamageAfterResolution => "Take the damage".into(),
            Choice::CommittedStrike => "Strike: the committed strike".into(),
            Choice::BurnWeaponStrike { weapon } => {
                o.card = Some(self.in_play(weapon));
                format!("Strike: burn {}", self.in_play_name(weapon))
            }
            Choice::EmployFromHand { card, mode } => {
                o.zone = Some(ZoneRef {
                    seat: self.seat.0,
                    zone: "hand".into(),
                });
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
            Choice::UnlockInfernal { minion } => {
                o.minion = Some(minion.0);
                format!("Burn 1 pool to unlock {}", self.minion_name(minion))
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
        if matches!(
            c,
            Choice::ChooseSeat(_)
                | Choice::ChooseMinion(_)
                | Choice::ChooseCard(_)
                | Choice::AllocateToCard { .. }
        ) {
            if let Some(r) = self.role(c) {
                o.text += &format!(" ({r})");
            }
        }
        if let Some(t) = self.purpose_text(c, &o.text) {
            o.text = t;
        }
        if let Some(about) = self.about.as_ref().filter(|_| {
            self.narrate
                && matches!(
                    self.window,
                    WindowLabel::PlayTerms
                        | WindowLabel::ActionTerms
                        | WindowLabel::ReferendumTerms
                )
        }) {
            o.text = format!("{about}: {}", o.text);
        }
        o
    }
}

/// The mode a card is played in: the level and Disciplines of its printed
/// line ("at superior [DOM]"), and what sets it apart when the definition
/// splits that line into several modes ("(gain 3 blood)", `split_label`).
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
    if let Some(label) = split_label(card, mode as u8) {
        s += &format!(" ({label})");
    }
    s
}

/// What sets a mode apart from the other modes its printed line splits
/// into: its effects that not all of them have ("gain 3 blood" against
/// "gain 2 blood and unlock"). None for a line not split, or a mode set
/// apart by its target or its window alone (the target names it).
fn split_label(card: u16, mode: u8) -> Option<String> {
    let def = zvtes::cards::lookup(card)?;
    let mode = mode as usize;
    let own = def.modes.get(mode)?;
    let printed = zvtes::cards::library_mode(card, mode);
    let siblings: Vec<_> = (0..def.modes.len())
        .filter(|&i| i != mode && zvtes::cards::library_mode(card, i) == printed)
        .map(|i| &def.modes[i])
        .collect();
    if siblings.is_empty() {
        return None;
    }
    let apart: Vec<Effect> = own
        .effects
        .iter()
        .filter(|e| siblings.iter().any(|m| !m.effects.contains(e)))
        .copied()
        .collect();
    let label = ability_label(&apart);
    (!label.is_empty()).then_some(label)
}

/// What an ability does, by its effects ("4 transfers, burn it, gain 2
/// pool"); empty when none says.
fn ability_label(effects: &[Effect]) -> String {
    ability_parts(effects).join(", ")
}

/// Each ability's label among a card's or vampire's several, without what
/// they all do (Dreams of the Sphinx's counter on each lock; Michael
/// Harris's pool): short buttons (owner: "keep the wording as short as
/// possible"). The whole label when that leaves nothing.
pub(crate) fn abilities_told(abilities: &[zvtes::cards::ModeDef]) -> Vec<String> {
    let parts: Vec<Vec<String>> = abilities.iter().map(|a| ability_parts(a.effects)).collect();
    parts
        .iter()
        .map(|own| {
            let apart: Vec<&String> = own
                .iter()
                .filter(|p| abilities.len() < 2 || !parts.iter().all(|o| o.contains(p)))
                .collect();
            if apart.is_empty() {
                own.join(", ")
            } else {
                apart
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        })
        .collect()
}

fn ability_parts(effects: &[Effect]) -> Vec<String> {
    let plural = |n: u8, one: &str, many: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {many}")
        }
    };
    effects
        .iter()
        .filter_map(|e| {
            Some(match *e {
                Effect::DeclareAction(ActionKind::Card | ActionKind::Ability) => "action".into(),
                // "political action", not "political action action".
                Effect::DeclareAction(k) => match action_kind(k) {
                    t if t.ends_with("action") => t.into(),
                    t => format!("{t} action"),
                },
                Effect::SpendTransfers(n) => plural(n, "transfer", "transfers"),
                Effect::PayPool(n) => format!("pay {n} pool"),
                Effect::PayBlood(n) => format!("pay {n} blood"),
                Effect::HostBurnBlood(n) => format!("burn {n} blood"),
                Effect::BurnOwnLife(n) => format!("burn {n} life"),
                Effect::BurnPool(n) => format!("burn {n} pool"),
                Effect::BurnSelf | Effect::BurnSource => "burn it".into(),
                Effect::AddStealth(n) => format!("+{n} stealth"),
                Effect::ReduceStealth(n) => format!("-{n} stealth"),
                Effect::AddIntercept(n) => format!("+{n} intercept"),
                Effect::AttemptingIntercept(n) => format!("+{n} intercept"),
                Effect::Strike(k) | Effect::CommittedStrike(k) => {
                    format!("strike: {}", crate::narrate::strike_text(Some(k)))
                }
                Effect::WeaponStrikePlus(n) | Effect::CommittedWeaponStrikePlus(n) => {
                    format!("a melee weapon's strike at +{n} damage")
                }
                Effect::StrengthThisRound(n) => format!("+{n} strength this round"),
                Effect::StrengthThisCombat(n) => format!("+{n} strength"),
                Effect::Prevent(n) => format!("prevent {n} damage"),
                Effect::Press => "a press".into(),
                Effect::Maneuver => "a maneuver".into(),
                Effect::EndCombat => "end combat".into(),
                Effect::EnterCombatWithTarget => "enter combat".into(),
                Effect::DamageTarget(n) => format!("{n} damage"),
                Effect::UnlockActor | Effect::UnlockHost | Effect::UnlockSelf => "unlock".into(),
                Effect::UnlockTarget => "unlock the target".into(),
                Effect::LockHost => "lock the vampire".into(),
                Effect::LockTarget => "lock the target".into(),
                Effect::ActionFails => "the action fails".into(),
                Effect::CancelPlayed { .. } => "cancel the card".into(),
                Effect::BurnTargetCard => "burn the card".into(),
                Effect::BurnTargetMinion => "burn the target".into(),
                Effect::AddHostBlood(n) => format!("add {n} blood"),
                Effect::BloodToTarget(n) => format!("{n} blood to the target"),
                Effect::BloodToUncontrolledTarget(n) => {
                    format!("{n} blood to uncontrolled")
                }
                Effect::StealBloodFromTarget(n) => format!("steal {n} blood"),
                Effect::TargetBurnsBlood(n) => format!("the target burns {n} blood"),
                Effect::TargetLeavesTorpor => "the target leaves torpor".into(),
                Effect::GainPool(n) => format!("gain {n} pool"),
                Effect::PoolIfSuccessful(n) => format!("gain {n} pool if successful"),
                Effect::EatAshAlly { blood, unlock } => {
                    format!(
                        "gain {blood} blood{}",
                        if unlock { " and unlock" } else { "" }
                    )
                }
                Effect::EatAshAllyToTarget(n) => format!("{n} life to the target"),
                Effect::IfBleedSuccessful(then) => {
                    format!("if the bleed is successful, {}", ability_label(then))
                }
                Effect::GainTransfers(n) => format!("+{}", plural(n, "transfer", "transfers")),
                Effect::AddVotes(n) => format!("+{}", plural(n, "vote", "votes")),
                Effect::MethuselahVotes { n, favor } => format!(
                    "+{} {}",
                    plural(n, "vote", "votes"),
                    if favor { "for" } else { "against" }
                ),
                Effect::GrantDiscardAction => "+1 discard phase action".into(),
                Effect::CallReferendum(_) => "call a referendum".into(),
                Effect::HandSizeBonusUntilEndOfTurn(n) => format!("+{n} hand size this turn"),
                Effect::HuntAction => "hunt".into(),
                Effect::SearchEquip => "search for equipment".into(),
                Effect::DrawThenBottom => "draw, then bottom a card".into(),
                Effect::DrawCryptThenRemove => "crypt draw, then remove one".into(),
                Effect::DrawThenDiscardDown(n) => {
                    format!("draw {n}, discard down")
                }
                // Which way blood goes (Powerbase: Munich, Sri Sansa).
                Effect::TargetBloodToPool(n) => format!("{n} blood to pool"),
                Effect::PoolToTargetBlood(n) => format!("{n} pool to blood"),
                Effect::HostBloodToTarget(n) => format!("give {n} blood"),
                Effect::TargetBloodToHost(n) => format!("take {n} blood"),
                Effect::StealBloodToHostX => "take X blood".into(),
                Effect::BloodToActing(n) => format!("+{n} blood"),
                Effect::BloodToChosen(n) => format!("+{n} blood"),
                Effect::BloodToTargetToStart(_) => "+blood".into(),
                Effect::BloodPerHostage => "+1 blood per hostage".into(),
                Effect::HuntingGroundBlood(n) => format!("+{n} blood"),
                Effect::HuntingGroundLife(n) => format!("+{n} life"),
                Effect::BloodToCard(n) => format!("{n} blood to the card"),
                Effect::CountersToUncontrolledTarget => "counters to uncontrolled".into(),
                Effect::HostBurnBloodToUnlock => "burn 1 blood: unlock".into(),
                Effect::TargetBurnLife(n) => format!("target burns {n}"),
                Effect::BleedMod { n, .. } => format!("+{n} bleed"),
                Effect::ReduceBleedX => "-X bleed".into(),
                Effect::RemoveCounter(n) => {
                    format!("burn {}", plural(n, "counter", "counters"))
                }
                Effect::RemoveCountersX => "burn X counters".into(),
                Effect::AddCounter(n) => format!("add {}", plural(n, "counter", "counters")),
                Effect::PoolToCounterPlusBank { pool, .. } => {
                    format!("bank {pool} pool")
                }
                Effect::PayActionWithCountersX => "pay with X counters".into(),
                Effect::CounterToTarget(_) => "counter on target".into(),
                Effect::PreventGunStrikeAllowance(n) => {
                    format!("prevent {n} (guns)")
                }
                Effect::PreventNonGun(n) => format!("prevent {n} (not guns)"),
                Effect::PreventNonAggravated(n) => {
                    format!("prevent {n} non-agg.")
                }
                Effect::ImmuneStrikeNonAggravated => "immune: strike".into(),
                Effect::ImmuneEffectNonAggravated => "immune: effects".into(),
                Effect::SetRange(r) => format!("{} range", format!("{r:?}").to_lowercase()),
                Effect::CommitStrike => "commit its strike".into(),
                Effect::OptionalManeuver => "a maneuver".into(),
                Effect::OptionalPress => "a press".into(),
                Effect::ContinuePressForOwnCombatant => "press to continue".into(),
                Effect::ReduceActionBlood(n) => format!("-{n} blood cost"),
                Effect::ReduceActionCost(n) => format!("-{n} cost"),
                Effect::UnlockActing => "unlock the actor".into(),
                Effect::UnlockActingIfSuccessful(_) => "unlock if successful".into(),
                Effect::FailReferendum => "referendum fails".into(),
                Effect::NonCamarillaVotesMinus(n) => {
                    format!("non-Camarilla -{n} vote")
                }
                Effect::TargetVotesPerCounter => "+1 vote per counter".into(),
                Effect::PayPoolX => "pay X pool".into(),
                Effect::MethuselahVotesX { per, favor } => {
                    format!("+{per}X votes {}", if favor { "for" } else { "against" })
                }
                Effect::SpendMasterPhaseAction => "master phase action".into(),
                Effect::PoolToCounter(n) => format!("{n} pool to the card"),
                // Blood kept as counters (Acquired Ventrue Assets, Blood
                // Puppy).
                Effect::CounterToTargetBlood => "1 blood to the vampire".into(),
                Effect::CounterToPool => "1 blood to pool".into(),
                Effect::CountersToPool => "all its blood to pool".into(),
                Effect::GiveToPrey => "give to prey".into(),
                Effect::LockPreyVampires(n) => format!("lock {n} prey vampires"),
                Effect::CryptNamedToUncontrolled(name) => {
                    format!("fetch {name}")
                }
                Effect::StoreFromPreyAsh => "take from prey's ash heap".into(),
                _ => return None,
            })
        })
        .collect()
}

/// The mode's effects include `effect`.
fn mode_has(card: u16, mode: u8, effect: Effect) -> bool {
    zvtes::cards::lookup(card)
        .and_then(|def| def.modes.get(mode as usize))
        .is_some_and(|m| m.effects.contains(&effect))
}

/// A play's second vampire (`Terms::OwnOfClan`), chosen after its target:
/// who it is, and its role as an option ("non-bleed actions cannot be
/// blocked": Toreador Grand Ball).
fn second_chosen(card: u16, mode: u8) -> Option<(String, String)> {
    let m = zvtes::cards::lookup(card)?.modes.get(mode as usize)?;
    let Some(Terms::OwnOfClan { clan, .. }) = m.terms else {
        return None;
    };
    Some(
        if m.effects
            .contains(&Effect::Layer(Modifier::SecondChosenUnblockable))
        {
            (
                format!("the {clan} whose non-bleed actions cannot be blocked"),
                "non-bleed actions cannot be blocked".into(),
            )
        } else {
            (
                format!("the card's chosen {clan}"),
                format!("the card's chosen {clan}"),
            )
        },
    )
}

/// The mode declares a bleed (Govern the Unaligned [dom]).
fn bleeds(card: u16, mode: u8) -> bool {
    zvtes::cards::lookup(card)
        .and_then(|def| def.modes.get(mode as usize))
        .is_some_and(|m| {
            m.effects
                .contains(&zvtes::cards::Effect::DeclareAction(ActionKind::Bleed))
        })
}

/// A play's mode: its index and the printed tags of its line.
pub(crate) fn mode_view(card: u16, mode: u8) -> ModeView {
    let lib = &LIBRARY[card as usize];
    let printed = zvtes::cards::lookup(card)
        .filter(|def| (mode as usize) < def.modes.len())
        .map(|_| zvtes::cards::library_mode(card, mode as usize));
    ModeView {
        index: mode,
        tags: printed
            .and_then(|m| lib.modes.get(m))
            .map_or(Vec::new(), |m| {
                m.tags.iter().map(|t| t.to_string()).collect()
            }),
        line: printed.unwrap_or(mode as usize) as u8,
        label: split_label(card, mode),
        target: target_role(card, mode),
    }
}

/// The role of a play's target when a second vampire is chosen after it
/// (`second_chosen`): by what the mode does to the target.
fn target_role(card: u16, mode: u8) -> Option<String> {
    second_chosen(card, mode)?;
    let m = zvtes::cards::lookup(card)?.modes.get(mode as usize)?;
    let Some(Terms::OwnOfClan { clan, .. }) = m.terms else {
        return None;
    };
    let lock = m.effects.contains(&Effect::LockTarget);
    let stays = m
        .effects
        .contains(&Effect::Layer(Modifier::FirstChosenDoesNotUnlock));
    let put_on = m.effects.contains(&Effect::PutOnTarget);
    Some(match (lock, stays) {
        (true, true) => format!("the {clan} to lock, who does not unlock as normal"),
        (true, false) => format!("the {clan} to lock"),
        _ if put_on => "the minion to put this card on".into(),
        _ => format!("the card's first {clan}"),
    })
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
        Choice::BurnEdge => "edge_pool",
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
        Choice::UnlockInfernal { .. } => "unlock_infernal",
        Choice::ContinueAction => "continue_action",
        Choice::GainStrength => "gain_strength",
        Choice::BurnBloodToAvoid => "burn_blood_to_avoid",
        Choice::BurnCorruptionToFail { .. } => "burn_corruption_to_fail",
        Choice::BurnCorruptionToUnlock { .. } => "burn_corruption_to_unlock",
        Choice::Diablerise => "diablerise",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(name: &str) -> u16 {
        LIBRARY.iter().position(|c| c.name == name).unwrap() as u16
    }

    /// A printed line split into several modes is one choice of
    /// Disciplines, each mode named by what sets it apart.
    #[test]
    fn split_modes_share_their_line_and_say_what_sets_them_apart() {
        let view = |name: &str, mode: u8| {
            let v = mode_view(card(name), mode);
            (v.line, v.label)
        };
        let s = |l: &str| Some(l.to_string());
        assert_eq!(view("Psychophagia", 0), (0, s("gain 3 blood")));
        assert_eq!(view("Psychophagia", 1), (0, s("gain 2 blood and unlock")));
        assert_eq!(view("Putrescent Sustenance", 2), (1, s("gain 3 blood")));
        assert_eq!(
            view("Putrescent Sustenance", 3),
            (1, s("3 life to the target"))
        );
        assert_eq!(view("Feast of the Soul's Secrets", 0), (0, None));
        assert_eq!(
            view("Feast of the Soul's Secrets", 2),
            (1, s("if the bleed is successful, 2 blood to uncontrolled"))
        );
        assert_eq!(
            view("Lam Into", 3).1,
            s("a melee weapon's strike at +2 damage")
        );
        assert!(!mode_text(card("Psychophagia"), 1).contains("option"));
    }

    /// A card or vampire with several abilities says what each does, never
    /// "ability 2" (owner: Powerbase: Munich "with no clear statement about
    /// which is blood to pool or pool to blood"): a label each, distinct
    /// from the others' unless their effects are the same.
    #[test]
    fn every_ability_among_several_is_told_apart() {
        let mut bad = Vec::new();
        let mut check = |name: &str, abilities: &[zvtes::cards::ModeDef]| {
            if abilities.len() < 2 {
                return;
            }
            let labels = abilities_told(abilities);
            for (i, a) in abilities.iter().enumerate() {
                let moved = a
                    .effects
                    .iter()
                    .any(|e| matches!(e, Effect::HostBloodToPool(_) | Effect::PoolToHostBlood(_)));
                let clash = (0..abilities.len())
                    .any(|j| j != i && labels[j] == labels[i] && abilities[j].effects != a.effects);
                if !moved && (labels[i].is_empty() || clash) {
                    bad.push(format!("{name} #{i}: {:?}", a.effects));
                }
            }
        };
        for i in 0..LIBRARY.len() {
            if let Some(d) = zvtes::cards::lookup(i as u16) {
                check(d.name, d.abilities);
            }
        }
        for i in 0..zvtes::crypt::CRYPT.len() {
            if let Some(v) = zvtes::cards::vampire(i as u16) {
                check(v.name, v.abilities);
            }
        }
        assert!(bad.is_empty(), "abilities told apart by number: {bad:#?}");
        let munich = zvtes::cards::lookup(card("Powerbase: Munich")).unwrap();
        assert_eq!(
            abilities_told(munich.abilities)[..2],
            ["1 blood to pool".to_string(), "1 pool to blood".to_string()]
        );
        // What every use does goes unsaid (Dreams of the Sphinx's counter).
        let sphinx = zvtes::cards::lookup(card("Dreams of the Sphinx")).unwrap();
        assert_eq!(
            abilities_told(sphinx.abilities)[..2],
            [
                "+2 hand size this turn".to_string(),
                "gain 1 pool".to_string()
            ]
        );
    }

    /// Toreador Grand Ball's two Toreador: the target says it is the one
    /// locked (owner play feedback), Priority Contract's the one it goes on;
    /// a single-target play says nothing.
    #[test]
    fn a_play_choosing_two_vampires_names_its_targets_role() {
        let ball = card("Toreador Grand Ball");
        assert_eq!(
            mode_view(ball, 0).target.as_deref(),
            Some("the Toreador to lock, who does not unlock as normal")
        );
        assert_eq!(
            second_chosen(ball, 0).unwrap().1,
            "non-bleed actions cannot be blocked"
        );
        assert_eq!(
            mode_view(card("Priority Contract"), 0).target.as_deref(),
            Some("the minion to put this card on")
        );
        assert_eq!(mode_view(card("Psychophagia"), 0).target, None);
    }
}
