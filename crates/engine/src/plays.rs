//! Card plays from hand (offers enumerated from the definitions), the
//! master and discard phases, terms at announcement (rules-qa 3.5), the
//! as-played cancel stack (spec §1.8), resolution, replacement draws and
//! in-play abilities.

use crate::cards::families::Family;
use crate::cards::{
    self, AbilitySource, Actor, Effect, Hook, Layer, Limit, PlayCtx, Requirement, Rider, Scope,
    Target, TargetKind, TargetSpec, Terms, Until, Window,
};
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::draws::Interrupt;
use crate::game::{Engine, Flow, Pending};
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, Label, LIBRARY};
use crate::state::{distinct, ActionId, CardId, CardInPlay, Expiry, OutOfTurn, Release, TimedStat};
use crate::window::OrderedWindow;

/// The once-per-phase key of an in-play ability: the copy, or the copy and
/// mode under `Limit::PerPhaseEach`.
fn phase_key(source: AbilitySource, ability: u8, limit: Option<Limit>) -> (AbilitySource, u8) {
    match limit {
        Some(Limit::PerPhaseEach) => (source, ability),
        _ => (source, 0),
    }
}

/// Who uses an in-play ability (`Engine::ability_user`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AbilityUser {
    /// The Methuselah using it: its terms are theirs to choose, its
    /// effects theirs.
    pub seat: SeatId,
    /// The minion its host predicates read and its host costs are paid
    /// by: the card's bearer, the vampire whose text it is, the vampire
    /// using a card it is not on.
    pub host: Option<MinionId>,
    /// The minion its play predicates read as the actor, and its combat
    /// effects apply to: the host, the minion declaring a granted action,
    /// or the user's minion in combat for an ability used from out of the
    /// combat (Opikun).
    pub actor: Option<MinionId>,
}

/// A record of ability uses a per-copy limit counts in.
#[derive(Clone, Copy, Debug)]
enum Ledger {
    Combat,
    Round,
    Action,
    Referendum,
    Turn,
}

/// Where `limit` counts the uses of `ability` of `source`: the ledger, the
/// key, the uses allowed (`None`: unlimited, or once per phase through
/// `phase_uses`).
fn limit_slot(
    limit: Option<Limit>,
    source: AbilitySource,
    ability: u8,
) -> Option<(Ledger, (AbilitySource, u8), usize)> {
    let used = source.use_key(ability);
    Some(match limit? {
        Limit::PerCombat => (Ledger::Combat, used, 1),
        Limit::TwicePerCombat => (Ledger::Combat, used, 2),
        Limit::PerCombatCard => (Ledger::Combat, source.use_key(0), 1),
        Limit::PerRound => (Ledger::Round, used, 1),
        // Per copy per action (Haqim's Law: Retribution, ANK 20221102-2);
        // once each action or referendum for this ability, whoever uses it
        // ("after …", "during …": rules-qa 16.1, 58.7 — Forward Momentum,
        // Alamut).
        Limit::PerAction => (Ledger::Action, source.action_key(ability), 1),
        // One use each action for the card, whoever the actor (Camarilla
        // Conclave).
        Limit::PerActionCard => (Ledger::Action, source.action_key(0), 1),
        Limit::PerReferendum => (Ledger::Referendum, used, 1),
        Limit::PerTurn => (Ledger::Turn, used, 1),
        Limit::PerPhaseEach => return None,
        Limit::BetweenUnlockPhases | Limit::OnePerAction | Limit::PerCombatThisMode => {
            unreachable!("a card-play limit on an ability")
        }
    })
}

/// The action an ability grants, if it does (`Effect::DeclareAction`).
fn declared_kind(ability: &cards::ModeDef) -> Option<ActionKind> {
    ability.effects.iter().find_map(|e| match e {
        Effect::DeclareAction(k) => Some(*k),
        _ => None,
    })
}

/// How a card put in play as it resolves (`Engine::place_now`) gets its
/// mode's layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModeLayers {
    /// From the mode's own `Effect::Layer`s, which follow the placement in
    /// the effects the play resolves (every definition: none comes first).
    FromEffects,
    /// All at once with the placement: no effect loop runs the mode's
    /// layers (a referendum's card, Pillars Fall after combat).
    Push,
}

/// Who resolves a shared effect (`Engine::apply_shared_effect`): a card
/// played, with the card it put in play if any, or an in-play ability.
#[derive(Clone, Copy, Debug)]
pub(crate) enum By {
    Play(Option<CardId>),
    Ability(AbilitySource),
}

/// Where a hook fires (`Engine::fired_riders`).
pub(crate) struct HookAt<'a> {
    /// The seat a rider's scope is read against: the active Methuselah at a
    /// phase hook, the event's seat at an "own" one; `None` where the scope
    /// does not restrict.
    pub seat: Option<SeatId>,
    /// At a `Hook::reads_action` hook, the seat and minion its riders'
    /// requirements are read for.
    pub acting: Option<(SeatId, MinionId)>,
    /// The cards the hook fires on (the host's, the cards a minion played
    /// …).
    pub on: &'a dyn Fn(&CardInPlay) -> bool,
}

impl HookAt<'static> {
    /// Every card in play's riders, unscoped.
    pub(crate) const ANY: HookAt<'static> = HookAt {
        seat: None,
        acting: None,
        on: &|_| true,
    };

    /// The riders of `seat`'s cards in play (scoped to that seat).
    pub(crate) fn of(seat: SeatId) -> HookAt<'static> {
        HookAt {
            seat: Some(seat),
            ..HookAt::ANY
        }
    }
}

impl<'a> HookAt<'a> {
    /// The riders of the cards `on` names, unscoped.
    pub(crate) fn on(on: &'a dyn Fn(&CardInPlay) -> bool) -> HookAt<'a> {
        HookAt {
            seat: None,
            acting: None,
            on,
        }
    }
}

/// A card play between announcement and resolution: its as-played window is
/// open (other Methuselahs may cancel it), possibly beneath a nested cancel.
#[derive(Clone, Debug)]
pub(crate) struct PendingPlay {
    pub ctx: PlayCtx,
    pub window: OrderedWindow,
    pub canceled: bool,
    /// Canceled "(cost is still paid)" (Meditative Grove): its cost is
    /// paid as it is settled.
    pub cost_paid: bool,
}

/// An action modifier or a reaction: a minion plays the same one once in
/// an action (rulebook "Action Modifier", "Reaction").
fn once_per_action(card: u16) -> bool {
    LIBRARY[card as usize]
        .types
        .iter()
        .any(|t| matches!(t, CardType::ActionModifier | CardType::Reaction))
}

impl Engine {
    /// Draw up to hand size from the library (rulebook "Drawing Cards"):
    /// the cards not replaced yet count against it; an empty library draws
    /// nothing.
    pub(crate) fn draw_up(&mut self, s: SeatId) {
        // A draw alternative applies: each card drawn is a choice (rules-qa
        // 9.4), made before any other decision.
        if self.draws_interrupt(s) {
            self.interrupts.push(Interrupt::DrawUp { seat: s });
            return;
        }
        let size = self.state.hand_size(s);
        while self.state.hand_load(s) < size {
            let seat = &mut self.state.seats[s.index()];
            match seat.library.take_top() {
                Some((c, copy)) => {
                    seat.hand.put(c, copy);
                    self.offer_redraw(s, c);
                }
                None => break,
            }
        }
    }

    /// Replace `n` cards that left `s`'s hand, `except` this card's own
    /// rider (spec §6.8.3; rules-qa 8.3, 9.4; ANK 20200905).
    pub(crate) fn replace(&mut self, s: SeatId, n: usize, except: Option<CardId>) {
        // The Meddling of Semsith: held until the minion phase ends.
        if n > 0 && self.state.in_minion_phase && self.state.meddled(s) {
            let held = &mut self.state.seats[s.index()].held[Release::MinionPhaseEnd];
            *held = held.saturating_add(n as u8);
            return;
        }
        for _ in 0..n {
            if self.state.seats[s.index()].library.is_empty() {
                break;
            }
            let Some(id) = self.replacement_rider(s, except) else {
                continue;
            };
            if self.draws_interrupt(s) {
                self.interrupts
                    .push(Interrupt::Replacement { seat: s, except });
            } else {
                self.apply_replacement_rider(id);
            }
        }
        let drawing = !self.state.seats[s.index()].library.is_empty()
            && self.replacement_rider(s, except).is_none();
        // Infernal Pursuit [CEL]: k more drawn per card replaced, as many
        // as the library has (LSJ 20060620), then discarded down; several
        // replaced at once go one at a time (ANK 20180925-1).
        let k = self.state.seats[s.index()]
            .timed
            .total(TimedStat::ExtraDraws) as u8;
        if drawing && k > 0 && self.combat.is_some() && n > 1 {
            for _ in 0..n {
                self.interrupts
                    .push(Interrupt::PursuitDraw { seat: s, extra: k });
            }
            self.interrupts.push(Interrupt::DrawUp { seat: s });
            return;
        }
        self.draw_up(s);
        if drawing && k > 0 && self.combat.is_some() {
            for _ in 0..n * k as usize {
                let seat = &mut self.state.seats[s.index()];
                match seat.library.take_top() {
                    Some((c, copy)) => seat.hand.put(c, copy),
                    None => break,
                }
            }
        }
    }

    /// Learjet: "you may draw an additional card (discard afterward)",
    /// after the normal replacement is seen (LSJ 20050224-4); none from an
    /// empty library.
    pub(crate) fn offer_extra_draw(&mut self, s: SeatId) {
        if !self.state.seats[s.index()].library.is_empty() {
            self.interrupts.push(Interrupt::MayDrawExtra { seat: s });
        }
    }

    /// A card played by Learjet's bearer as the acting minion (its action's
    /// modifiers, combat cards in its combats; a canceled one too).
    fn learjet_play(&self, ctx: &PlayCtx) -> bool {
        // Ira Rivers: any card of his requiring Auspex.
        let printed = |m: MinionId| {
            let pctx = crate::state::PrintedCtx {
                card: Some((ctx.card, ctx.mode)),
                ..Default::default()
            };
            self.state
                .printed_mod(m, cards::Stat::ExtraDrawOnPlay, pctx)
                > 0
        };
        ctx.actor.is_some_and(|m| {
            printed(m)
                || self.action.as_ref().is_some_and(|a| a.acting == m)
                    && self
                        .state
                        .borne(m)
                        .any(|l| l.modifier == cards::Modifier::ExtraDrawOnReplace)
        })
    }

    /// The end of the action (or of the turn it could not outlive): the
    /// cards held back until then are replaced (Consign to Oblivion [OBL];
    /// spec §1.9.2: after all combats).
    pub(crate) fn release_action_holds(&mut self) {
        self.release_held(Release::EndOfAction);
        self.release_held(Release::Weakens);
    }

    /// Every Methuselah's cards held back until this condition are
    /// replaced ("do not replace until …"; LSJ 20080805: whether or not
    /// the card is still in play).
    pub(crate) fn release_held(&mut self, at: Release) {
        for i in 0..self.state.seats.len() {
            let seat = &mut self.state.seats[i];
            if seat.held[at] > 0 && !seat.ousted {
                self.replace_held(SeatId(i as u8), at, 0, None);
            } else {
                seat.held[at] = 0;
                seat.learjet_held[at] = 0;
            }
        }
    }

    /// Seat `s`'s replacements held until `at`, plus `also`, drawn now;
    /// then Learjet's extra draw for each one its bearer played (PIB
    /// 20110718), waiting with them if the Meddling holds them again
    /// (rules-qa 73.1).
    pub(crate) fn replace_held(
        &mut self,
        s: SeatId,
        at: Release,
        also: usize,
        except: Option<CardId>,
    ) {
        let seat = &mut self.state.seats[s.index()];
        let n = std::mem::take(&mut seat.held[at]);
        let jet = std::mem::take(&mut seat.learjet_held[at]).min(n);
        let meddled = seat.held[Release::MinionPhaseEnd];
        self.replace(s, n as usize + also, except);
        let seat = &mut self.state.seats[s.index()];
        if seat.held[Release::MinionPhaseEnd] > meddled {
            let later = &mut seat.learjet_held[Release::MinionPhaseEnd];
            *later = (*later + jet).min(seat.held[Release::MinionPhaseEnd]);
        } else {
            for _ in 0..jet {
                self.offer_extra_draw(s);
            }
        }
    }

    /// `n` cards of `seat`'s hand to the ash heap at random, one at a time.
    fn discard_at_random(&mut self, seat: SeatId, n: usize) {
        for _ in 0..n {
            let s = &mut self.state.seats[seat.index()];
            let i = self.rng.below(s.hand.len());
            let (c, copy) = s.hand.take(i);
            s.ash.put(c, copy);
        }
    }

    /// Hold a replacement for Blood Weakens — drawn at once, with the ones
    /// held before, if the hand is empty.
    fn hold_weakened(&mut self, s: SeatId, in_play: Option<CardId>) {
        if self.state.seats[s.index()].hand.is_empty() {
            self.replace_held(s, Release::Weakens, 1, in_play);
        } else {
            self.state.seats[s.index()].held[Release::Weakens] += 1;
        }
    }

    /// Is this play's replacement held by Blood Weakens: a minion's card
    /// played using a Discipline (during an action, or declaring one), the
    /// minion no diablerist ignoring it?
    fn weakened_card(&self, ctx: &PlayCtx) -> bool {
        self.state
            .global(cards::Modifier::HoldDisciplineReplacements)
            .is_some()
            && ctx.actor.is_some_and(|m| {
                !self.state.minions[m.index()].diablerie_shield && !self.state.gehenna_immune(m)
            })
            && played_disciplines(ctx.card, ctx.mode)
                .iter()
                .any(|t| cards::discipline_key(t) != "?")
    }

    /// A Gehenna event `id` was played: the others' counters go down
    /// (Wormwood, Fueled by Heart's Blood), and diablerists no longer
    /// ignore Blood Weakens and The Slow Withering.
    fn gehenna_played(&mut self, id: CardId) {
        for m in &mut self.state.minions {
            m.diablerie_shield = false;
        }
        for (c, r) in self.fired_riders(Hook::GehennaPlayed, &HookAt::ANY) {
            if c == id {
                continue;
            }
            for &e in r.effects {
                self.apply_rider_effect(c, e, Hook::GehennaPlayed);
            }
        }
        // Wormwood's ceiling, as it enters or its counters go down.
        self.state.drain_all_excess();
    }

    /// A vampire moves from torpor to the ready region (a leave torpor
    /// action, a rescue, an effect): Torpid Blood's replacement.
    pub(crate) fn leave_torpor(&mut self, m: MinionId) {
        let v = &mut self.state.minions[m.index()];
        if v.in_torpor {
            v.in_torpor = false;
            self.release_held(Release::LeavesTorpor);
        }
    }

    /// Master phase actions for the active seat's master phase (rulebook
    /// "Master Phase"; out-of-turn accounting per rules-qa 3.3).
    pub(crate) fn start_master_phase(&mut self) {
        let s = self.state.active;
        // No minion phase under way (a fixture started in one).
        self.state.in_minion_phase = false;
        // Nightmares upon Nightmares' discards: replaced now.
        if self.state.seats[s.index()].held[Release::MasterPhase] > 0 {
            self.replace_held(s, Release::MasterPhase, 0, None);
        }
        let seat = &mut self.state.seats[s.index()];
        seat.mpa = 1;
        // Anson's second action, read live (`GameState::mpa_left`).
        seat.second_mpa_spent = false;
        seat.trifle_used = false;
        seat.master_played = false;
        seat.mpa += std::mem::take(&mut seat.mpa_next);
        if let Some(OutOfTurn { trifle }) = seat.out_of_turn.take() {
            if trifle {
                seat.trifle_used = true;
            } else {
                self.state.spend_mpa(s);
            }
        }
        self.state.phase_uses.clear();
        // Brujah Debate's lock: due again this phase.
        self.state.seats[s.index()].master_resolved.clear();
        self.flow = Flow::MasterPhase;
    }

    /// The master-phase locks due now for the active Methuselah (Brujah
    /// Debate, per copy, read live; one played this phase applies in it,
    /// rules-qa 33.1), less those resolved this phase.
    pub(crate) fn master_dues(&self) -> Vec<CardId> {
        let st = &self.state;
        let s = st.active;
        self.fired_riders(Hook::MasterPhaseStart, &HookAt::of(s))
            .into_iter()
            .filter(|&(id, r)| {
                !st.seats[s.index()].master_resolved.contains(&id)
                    && r.effects.iter().any(|e| match *e {
                        Effect::LockOldestOfClan(clan) => st
                            .minions_of(s)
                            .any(|m| st.minions[m.index()].clan() == Some(clan)),
                        _ => true,
                    })
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Minions of `seat` that may play a mode as its actor.
    fn actor_candidates(&self, seat: SeatId, mode: &cards::ModeDef) -> Vec<Option<MinionId>> {
        let st = &self.state;
        match mode.actor {
            Actor::Methuselah => vec![None],
            // Only combat cards during combat: no modifiers or reactions,
            // wakes included (rulebook "Combat"; spec §4.1.3).
            Actor::ActingMinion => match &self.action {
                Some(a) if a.seat == seat && self.combat.is_none() => vec![Some(a.acting)],
                _ => vec![],
            },
            Actor::ReactingMinion => self.reacting_candidates(seat, mode),
            // The acting minion for its controller, a reacting minion for
            // any other Methuselah (the card's type follows the player).
            Actor::ActingOrReactingMinion => match &self.action {
                Some(a) if a.seat == seat && self.combat.is_none() => vec![Some(a.acting)],
                Some(_) => self.reacting_candidates(seat, mode),
                None => vec![],
            },
            Actor::ActingCandidate => {
                if self.action.is_some() {
                    vec![]
                } else if let Some(m) = self.cog_minion(seat) {
                    vec![Some(m)]
                } else if st.active != seat {
                    vec![]
                } else {
                    st.ready_unlocked(seat).into_iter().map(Some).collect()
                }
            }
            // A master card naming one of the player's ready vampires, not
            // locked by it (Grooming the Protégé).
            Actor::AnyReadyVampire => st
                .minions_of(seat)
                .filter(|&m| st.minions[m.index()].ready() && st.minions[m.index()].is_vampire())
                .map(Some)
                .collect(),
            Actor::Host
            | Actor::HostOutOfCombat
            | Actor::HostControllersPrey
            | Actor::Controller
            | Actor::ForOwnCombatant
            | Actor::OpposingCombatant
            | Actor::ChosenController => {
                vec![]
            }
            // In its player's master phase, for a ready unlocked minion.
            Actor::MasterForMinion => {
                if st.active != seat || !matches!(self.flow, Flow::MasterPhase) {
                    vec![]
                } else {
                    st.ready_unlocked(seat).into_iter().map(Some).collect()
                }
            }
            Actor::OtherOwnMinion => match &self.action {
                Some(a) if a.seat == seat && self.combat.is_none() => st
                    .minions_of(seat)
                    .filter(|&m| m != a.acting && st.minions[m.index()].ready())
                    .map(Some)
                    .collect(),
                _ => vec![],
            },
            // A ready vampire of the player's, out of the combat its
            // Methuselah's minion is in.
            Actor::OwnOutOfCombat => match &self.combat {
                Some(c) if c.side_of(seat).is_some() => st
                    .minions_of(seat)
                    .filter(|&m| {
                        let v = &st.minions[m.index()];
                        v.ready() && !v.is_ally() && c.side_of_minion(m).is_none()
                    })
                    .map(Some)
                    .collect(),
                _ => vec![],
            },
            // Any Methuselah's unlocked vampire out of the combat, in
            // torpor too (Martyr's Resilience).
            Actor::UnlockedOutOfCombat => match &self.combat {
                Some(c) => st
                    .minions_of(seat)
                    .filter(|&m| {
                        let v = &st.minions[m.index()];
                        v.present() && v.is_vampire() && !v.locked && c.side_of_minion(m).is_none()
                    })
                    .map(Some)
                    .collect(),
                None => vec![],
            },
            // The player's minion in the combat; ready, except in the
            // end-of-round window (Taste of Vitae's "not usable by a
            // vampire being burned or going to torpor" is the card's own
            // clause).
            Actor::Combatant => match &self.combat {
                Some(c) => c
                    .side_of(seat)
                    .map(|i| c.sides[i].minion)
                    .filter(|m| {
                        st.minions[m.index()].ready() || c.step == crate::combat::Step::EndOfRound
                    })
                    .map(|m| vec![Some(m)])
                    .unwrap_or_default(),
                None => vec![],
            },
        }
    }

    /// Ready unlocked minions of `seat` (other than the acting Methuselah's)
    /// that may play a reaction: woken or locked-only per the mode.
    fn reacting_candidates(&self, seat: SeatId, mode: &cards::ModeDef) -> Vec<Option<MinionId>> {
        let st = &self.state;
        match &self.action {
            Some(a) if a.seat != seat && self.combat.is_none() => {
                let needs_locked = mode.requires.contains(&Requirement::LockedActor);
                let either = mode.requires.contains(&Requirement::MayBeLocked);
                st.minions_of(seat)
                    .filter(|m| {
                        let v = &st.minions[m.index()];
                        // Expulsion: no reaction cards this turn; Unleashing
                        // the Bestial Soul: none this action.
                        if v.in_torpor || v.expelled || a.no_reactions.contains(m) {
                            return false;
                        }
                        // Helena: as if unlocked for Auspex reactions (the
                        // card checked by `play_options`).
                        let woken = a.woken.contains(m) || a.aus_unlocked.contains(m);
                        if either {
                            true
                        } else if needs_locked {
                            // A woken minion is still a locked one.
                            v.locked
                        } else {
                            !v.locked || woken
                        }
                    })
                    .map(Some)
                    .collect()
            }
            // An action card as it is played, its action not declared yet:
            // a reacting minion's cancel of it (Touch of Clarity: "can be
            // used to cancel all action cards", ANK 20200813-2) — ready
            // and unlocked, another Methuselah's.
            None if self.combat.is_none()
                && mode
                    .effects
                    .iter()
                    .any(|e| matches!(e, Effect::CancelPlayed { .. }))
                && matches!(&self.flow, Flow::AsPlayed { stack, .. }
                    if stack.last().is_some_and(|p| p.ctx.seat != seat)) =>
            {
                st.minions_of(seat)
                    .filter(|m| {
                        let v = &st.minions[m.index()];
                        v.ready() && !v.locked && !v.expelled
                    })
                    .map(Some)
                    .collect()
            }
            _ => vec![],
        }
    }

    /// What `seat` may play or use at an impulse in `windows`: the cards,
    /// the abilities of cards in play timed like them (Bowl of Convergence
    /// [AUS] on a block attempt), and the untimed abilities.
    pub(crate) fn impulse_options(&self, seat: SeatId, windows: &[Window]) -> Vec<Choice> {
        let mut out = self.play_options(seat, windows, None);
        for &w in windows {
            out.extend(self.ability_options(seat, w));
        }
        out.extend(self.ability_options(seat, Window::AnyImpulse));
        out
    }

    /// The vampire under Perfect Clarity a play of `card` (mode `mode`) by
    /// `actor` does not affect: a reaction card requiring Dominate or
    /// Presence, the actor not that vampire (whose own card is a modifier).
    pub(crate) fn clarity_shielded(
        &self,
        card: u16,
        mode: usize,
        actor: Option<MinionId>,
    ) -> Option<MinionId> {
        let (v, _) = self.action.as_ref()?.clarity?;
        let lib = LIBRARY.get(card as usize)?;
        if actor == Some(v) || !lib.types.contains(&CardType::Reaction) {
            return None;
        }
        lib.modes
            .get(cards::library_mode(card, mode))?
            .tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case("dom") || t.eq_ignore_ascii_case("pre"))
            .then_some(v)
    }

    /// Every legal play of a card from `seat`'s hand in any of `windows`,
    /// enumerated from the definitions.
    pub(crate) fn play_options(
        &self,
        seat: SeatId,
        windows: &[Window],
        played: Option<&PlayCtx>,
    ) -> Vec<Choice> {
        let st = &self.state;
        let action = self.action.as_ref();
        let combat = self.combat.as_ref();
        let referendum = self.referendum.as_ref();
        let mut out = Vec::new();
        let hand_len = st.seats[seat.index()].hand.len();
        let virtual_hand = self.virtual_hand(seat);
        // A card on a Fleshforge Chamber: played by a vampire of its clan.
        let store_clan = |i: usize| {
            // Spiritual Guidance's ash heap: as if from hand, no clan.
            if virtual_hand[i].1 == Some(crate::state::ASH_PLAY) {
                return Some("");
            }
            virtual_hand[i].1.and_then(|loc| {
                cards::lookup(st.cards[loc.index()].card)?.modes[0]
                    .effects
                    .iter()
                    .find_map(|e| match e {
                        Effect::PlayableFromStore(c) => Some(*c),
                        _ => None,
                    })
            })
        };
        for (hi, &(card, _)) in virtual_hand.iter().enumerate() {
            // Copies alike (same card, same place): one play for them all
            // (owner, 2026-10-04).
            if virtual_hand[..hi].contains(&virtual_hand[hi]) {
                continue;
            }
            let Some(def) = cards::lookup(card) else {
                continue;
            };
            if def.out_of_turn && st.seats[seat.index()].out_of_turn.is_some() {
                continue; // never two out-of-turn cards between your turns
            }
            // "You cannot voluntarily contest cards with yourself" (rulebook
            // "Contested Cards"; spec §1.13.1): a unique card is not
            // playable while a copy of it is in play under this seat —
            // contested or not (rules-qa 7.1).
            if st.controls_unique_copy(seat, card) {
                continue;
            }
            for (mi, mode) in def.modes.iter().enumerate() {
                if !mode.windows.iter().any(|w| windows.contains(w)) {
                    continue;
                }
                // The rulebook's one limited bleed increase per action; a
                // conditional one counts only while its condition holds,
                // read on the acting minion (Foreshadowing Destruction, PIB
                // 20110817).
                if let Some(a) = action.filter(|a| a.limited_used(st)) {
                    let counts = mode.effects.iter().any(|e| match e {
                        Effect::BleedMod { limited: true, .. }
                        | Effect::BleedModX { limited: true }
                        | Effect::BleedPerPick => true,
                        Effect::If {
                            requires,
                            then: Effect::BleedMod { limited: true, .. },
                            ..
                        } => a.condition_holds(st, seat, a.acting, requires),
                        _ => false,
                    });
                    if counts {
                        continue;
                    }
                }
                let tags = LIBRARY[card as usize].modes[cards::library_mode(card, mi)].tags;
                let clans = LIBRARY[card as usize].clans;
                for actor in self.actor_candidates(seat, mode) {
                    // "They cannot play cards named Torn Signpost"
                    // (Preternatural Strength).
                    if actor.is_some_and(|m| st.bars_playing(m, card)) {
                        continue;
                    }
                    // Helena, locked: reactions requiring Auspex only.
                    if let (Some(m), Some(a)) = (actor, action) {
                        if st.minions[m.index()].locked
                            && !a.woken.contains(&m)
                            && a.aus_unlocked.contains(&m)
                            && a.seat != seat
                            && !mode.requires.contains(&Requirement::LockedActor)
                            && !mode.requires.contains(&Requirement::MayBeLocked)
                            && !tags.iter().any(|t| t.eq_ignore_ascii_case("aus"))
                        {
                            continue;
                        }
                    }
                    // "Unlocks and attempts to block": only a minion that
                    // could.
                    if mode.requires.contains(&Requirement::CanAttemptBlock)
                        && !actor.is_some_and(|m| self.can_attempt_block(seat, m))
                    {
                        continue;
                    }
                    if (mode.requires.contains(&Requirement::NotPlayedThisGame)
                        || LIBRARY[card as usize].types.contains(&CardType::Event))
                        && st.once_played.contains(&card)
                    {
                        continue;
                    }
                    if mode.requires.contains(&Requirement::NotPlayedThisTurn)
                        && st.turn_played.contains(&card)
                    {
                        continue;
                    }
                    if st.barred_this_turn.contains(&card) {
                        continue;
                    }
                    // Approximation of Loyalty [PRE]: no more action
                    // modifiers on this action.
                    if action.is_some_and(|a| a.no_more_modifiers && a.seat == seat)
                        && LIBRARY[card as usize]
                            .types
                            .contains(&CardType::ActionModifier)
                        && matches!(mode.actor, Actor::ActingMinion | Actor::OtherOwnMinion)
                    {
                        continue;
                    }
                    // Perfect Clarity: no Dominate or Presence reaction
                    // this action, bleed redirection included (LSJ
                    // 20010621), but the polling step's own votes (rules-qa
                    // 71.25); with [THA], none from its combat opponent.
                    let shielded = self.clarity_shielded(card, mi, actor);
                    if let Some((v, superior)) = action.and_then(|a| a.clarity) {
                        let dom_pre = tags.iter().any(|t| {
                            t.eq_ignore_ascii_case("dom") || t.eq_ignore_ascii_case("pre")
                        });
                        let as_reaction = LIBRARY[card as usize]
                            .types
                            .contains(&CardType::Reaction)
                            && !matches!(mode.actor, Actor::ActingMinion | Actor::OtherOwnMinion);
                        let polling = referendum
                            .is_some_and(|r| r.step == crate::referendum::RefStep::Polling);
                        let opposing = superior
                            && combat.is_some_and(|c| {
                                c.side_of_minion(v)
                                    .is_some_and(|i| actor == Some(c.sides[1 - i].minion))
                            });
                        if dom_pre && (as_reaction && !polling || opposing) {
                            continue;
                        }
                    }
                    // Concoction of Vitality: no Discipline card for the
                    // acting vampire this action.
                    if action.is_some_and(|a| a.no_discipline_cards && a.seat == seat)
                        && LIBRARY[card as usize].modes.iter().any(|lm| {
                            lm.tags
                                .iter()
                                .any(|t| t.len() == 3 && t.chars().all(|c| c.is_ascii_alphabetic()))
                        })
                    {
                        continue;
                    }
                    // "Gehenna events cannot be played" (The Fourth Cycle).
                    if LIBRARY[card as usize].labels.contains(&Label::Gehenna)
                        && st.gehenna_barred()
                    {
                        continue;
                    }
                    if mode.requires.contains(&Requirement::OnePerMethuselah)
                        && st.controls_card_copy(seat, card)
                    {
                        continue;
                    }
                    // "Frenzy cards cannot be used on this vampire"
                    // (Tranquility Shield): checked on the minion it would
                    // be played on.
                    if let Some(c) = combat {
                        let immune = |m: MinionId| {
                            c.side_of_minion(m)
                                .is_some_and(|i| c.sides[i].frenzy_immune)
                        };
                        let on = self.frenzy_on(card, mi as u8, actor, Target::None);
                        if on.is_some_and(immune) {
                            continue;
                        }
                    }
                    // "Cancel a frenzy card played on this vampire as it is
                    // played" (Predator's Transformation [abo]).
                    if mode.requires.contains(&Requirement::PlayedFrenzyOnActor)
                        && !played.zip(actor).is_some_and(|(p, m)| {
                            self.frenzy_on(p.card, p.mode, p.actor, p.target) == Some(m)
                        })
                    {
                        continue;
                    }
                    // "Cancel the opposing minion's strike card as it is
                    // played" (Anticipation [AUS]).
                    if mode.requires.contains(&Requirement::PlayedStrikeByOpponent)
                        && !played
                            .zip(actor)
                            .is_some_and(|(p, m)| self.strike_card_against(m, p))
                    {
                        continue;
                    }
                    // "A strike card … or a grapple card as it is played by
                    // the opposing minion" (Charismatic Aura).
                    if mode
                        .requires
                        .contains(&Requirement::PlayedStrikeOrGrappleByOpponent)
                        && !played.zip(actor).is_some_and(|(p, m)| {
                            self.strike_card_against(m, p) || self.grapple_card_against(m, p)
                        })
                    {
                        continue;
                    }
                    if mode
                        .requires
                        .contains(&Requirement::PlayedRestrictsStrikesByOpponent)
                        && !played
                            .zip(actor)
                            .is_some_and(|(p, m)| self.restricts_strikes_of(m, p))
                    {
                        continue;
                    }
                    if mode
                        .requires
                        .contains(&Requirement::PlayedAimFrenzyGrappleByOpponent)
                        && !played
                            .zip(actor)
                            .is_some_and(|(p, m)| self.aim_frenzy_grapple_against(m, p))
                    {
                        continue;
                    }
                    if mode.requires.contains(&Requirement::CanAttemptBlockAnyone)
                        && !actor.is_some_and(|m| self.can_attempt_block_anyone(seat, m))
                    {
                        continue;
                    }
                    // Nothing in hand it could equip (Angel's Gift).
                    if let (
                        Some(Terms::EquipFromHand {
                            filter,
                            blood_share,
                        }),
                        Some(m),
                    ) = (mode.terms, actor)
                    {
                        if self
                            .equippable_from_hand(seat, m, filter, blood_share)
                            .is_empty()
                        {
                            continue;
                        }
                    }
                    // Nothing in hand it could employ (Pack Alpha [ani]).
                    if let (Some(terms), Some(m)) = (mode.terms, actor) {
                        let (label, ignore) = match terms {
                            Terms::EmployFromHand(l) => (Some(Some(l)), false),
                            Terms::EmployFromHandIgnoring(l) => (Some(Some(l)), true),
                            Terms::RecruitOrEmployFromHand => (Some(None), false),
                            _ => (None, false),
                        };
                        if let Some(label) = label {
                            if self.employable_from_hand(seat, m, label, ignore).is_empty() {
                                continue;
                            }
                        }
                    }
                    // An X blood cost with a printed minimum the actor
                    // cannot pay (Monkey Wrench: "X must be 1, 2 or 3").
                    if let (Some(Terms::XBloodIn { min, .. }), Some(m)) = (mode.terms, actor) {
                        if st.minions[m.index()].blood < min {
                            continue;
                        }
                    }
                    // A political action card cannot be played if its terms
                    // cannot be met (LSJ 20100129).
                    if mode.effects.iter().any(|e| match (e, actor) {
                        (Effect::CallReferendum(r), Some(m)) => !r.feasible(st, seat, m),
                        _ => false,
                    }) {
                        continue;
                    }
                    // Clan requirement: the playing minion's clan, or (a
                    // Methuselah's card) a ready vampire of the clan, or
                    // one playing as it (Abraham Mellon, Lady Scarlett
                    // Churchill).
                    if !clans.is_empty() {
                        let ok = match actor {
                            Some(m) => st.meets_clan(m, clans, false),
                            None => st.minions_of(seat).any(|m| {
                                st.minions[m.index()].ready() && st.meets_clan(m, clans, true)
                            }),
                        };
                        if !ok {
                            continue;
                        }
                    }
                    // Path icon: the playing minion follows the path, or
                    // (a Methuselah's card) a ready vampire of theirs does
                    // (rules-qa 49.7).
                    if let Some(path) = LIBRARY[card as usize].path {
                        let ok = match actor {
                            Some(m) => st.follows(m, path),
                            None => st
                                .minions_of(seat)
                                .any(|m| st.minions[m.index()].ready() && st.follows(m, path)),
                        };
                        if !ok {
                            continue;
                        }
                    }
                    // "This vampire cannot recruit allies or employ
                    // retainers" (Depravity).
                    if let Some(m) = actor {
                        let hires = mode.effects.iter().any(|e| {
                            matches!(
                                e,
                                Effect::DeclareAction(ActionKind::Recruit | ActionKind::Employ)
                            )
                        });
                        if hires && st.cannot_recruit_or_employ(m) {
                            continue;
                        }
                    }
                    // An ally playing this card "as a vampire" (Spectral
                    // Servitor): the card's own requirements see an
                    // Independent vampire of the grant's capacity (LSJ
                    // 20020503; rulebook "Allies").
                    let as_vampire =
                        actor.and_then(|m| cards::plays_card_as_vampire(st, m, card, mi));
                    if let Some(m) = actor {
                        if !cards::has_disciplines(st, m, card, mi) {
                            continue;
                        }
                        // One political action per minion per turn
                        // (rulebook "Political Action"); one Delaying
                        // Tactics canceled is not taken again this turn by
                        // the seat's minions.
                        if mode.effects.contains(&Effect::DeclareAction(
                            crate::decision::ActionKind::Political,
                        )) && (st.minions[m.index()].political_this_turn
                            || st.seats[seat.index()]
                                .forbidden_political
                                .contains(&ActionId::Card(card)))
                        {
                            continue;
                        }
                        // One bleed a minion each turn (rulebook "Bleed"),
                        // with a bleed action card too (Entrancement [pre]).
                        if mode
                            .effects
                            .contains(&Effect::DeclareAction(crate::decision::ActionKind::Bleed))
                            && st.minions[m.index()].bled_this_turn
                        {
                            continue;
                        }
                        // NRA (rules-qa 1.4): same action card name once per
                        // minion per turn.
                        if mode
                            .effects
                            .iter()
                            .any(|e| matches!(e, Effect::DeclareAction(_)))
                            && st.minions[m.index()]
                                .acted
                                .iter()
                                .any(|a| a.same(ActionId::Card(card)))
                        {
                            continue;
                        }
                        let x = cards::terms_x(st, seat, card, mode).unwrap_or(0);
                        let blood = cards::blood_cost(st, card, x) + self.reaction_surcharge(card);
                        // What announced reductions could take off (The
                        // Line, The Shard, Ravnos Carnival, Ravnos Cache).
                        let potential = self.cost_potential(&PlayCtx::new(
                            seat,
                            card,
                            mi as u8,
                            Some(m),
                            Target::None,
                            x,
                        ));
                        // Less the minion's printed reductions (Roger de
                        // Camden).
                        let mut blood = blood
                            + self.discipline_surcharge(m, card, mi as u8)
                            + self.combat_card_surcharge(m, card, mi as u8);
                        let mut pool = cards::pool_cost(st, seat, card, Target::None);
                        st.printed_cost_off(m, card, mi as u8)
                            .apply(&mut pool, &mut blood);
                        // Charisma's discount, taken as the payment takes it
                        // (off the pool cost if any, else the blood).
                        reduce(&mut pool, &mut blood, recruit_off(st, mode, m));
                        if blood.saturating_sub(potential) > st.minions[m.index()].blood {
                            continue;
                        }
                        // "This ally cannot have or use equipment or
                        // retainers" (Bone Shambler).
                        if st.no_equipment_or_retainers(m)
                            && [
                                crate::library::CardType::Equipment,
                                crate::library::CardType::Retainer,
                            ]
                            .iter()
                            .any(|t| LIBRARY[card as usize].types.contains(t))
                        {
                            continue;
                        }
                        // "This ally cannot have or use equipment"
                        // (Gravebound Drone).
                        if st.no_equipment(m)
                            && LIBRARY[card as usize]
                                .types
                                .contains(&crate::library::CardType::Equipment)
                        {
                            continue;
                        }
                        // Anachronism: no ranged weapon, from hand or
                        // becoming one (Zip Gun).
                        if st.ranged_barred(m, card) {
                            continue;
                        }
                        // An ally never pays a blood cost (advanced rules
                        // 5.5.1) nor plays political action cards (rulebook
                        // "Political action cards"), unless it plays the
                        // card "as a vampire" (rulebook "Allies"), action
                        // cards excepted where printed.
                        if st.minions[m.index()].is_ally() {
                            let lib = &LIBRARY[card as usize];
                            let as_vampire = as_vampire.is_some();
                            // Djeneba's toll is a minion's "blood or life":
                            // an ally pays it in life (rules-qa 42.2).
                            let toll = self.strike_card_toll(m, card, mi as u8);
                            // The Shard can take a blood cost to 0 for it
                            // (ANK 20230621), announced with the card (LSJ
                            // 20090705).
                            if !as_vampire
                                && (blood.saturating_sub(toll).saturating_sub(potential) > 0
                                    || lib
                                        .types
                                        .contains(&crate::library::CardType::PoliticalAction))
                            {
                                continue;
                            }
                        }
                        // "A vampire can play only one X each round / combat /
                        // action" (a canceled play counted at announcement).
                        let name = LIBRARY[card as usize].name;
                        let hit =
                            |v: &[(MinionId, &str)]| v.iter().any(|&(a, n)| a == m && n == name);
                        let limited = match mode.limit {
                            None => false,
                            Some(Limit::PerRound) => combat.is_some_and(|c| hit(&c.played_round)),
                            Some(Limit::PerCombat) => combat.is_some_and(|c| hit(&c.played_combat)),
                            Some(Limit::PerCombatThisMode) => combat.is_some_and(|c| {
                                c.played_combat_modes.contains(&(m, card, mi as u8))
                            }),
                            Some(Limit::PerAction) => action.is_some_and(|a| hit(&a.played)),
                            Some(Limit::OnePerAction) => {
                                action.is_some_and(|a| a.played.iter().any(|&(_, n)| n == name))
                            }
                            Some(Limit::BetweenUnlockPhases) => {
                                st.minions[m.index()].played_since_unlock.contains(&card)
                            }
                            Some(Limit::PerTurn) => st.turn_plays.contains(&(m, card)),
                            Some(
                                Limit::PerPhaseEach
                                | Limit::PerReferendum
                                | Limit::PerActionCard
                                | Limit::PerCombatCard
                                | Limit::TwicePerCombat,
                            ) => false,
                        };
                        // Rulebook "Action Modifier" / "Reaction": once per
                        // minion per action, by name whatever the level
                        // (rules-qa 46.1), a canceled one counted (advanced
                        // rules §1.8.3).
                        let once = combat.is_none()
                            && once_per_action(card)
                            && action.is_some_and(|a| hit(&a.played));
                        if limited || once {
                            continue;
                        }
                        // Effect-derived combat legality (alternation, the
                        // limited additional strike, grapples).
                        if let Some(c) = combat {
                            // An actor out of the combat plays for its
                            // Methuselah's combatant (Touch of Valeren [FOR]).
                            if let Some(i) = c.side_of_minion(m).or_else(|| {
                                (mode.actor == Actor::OwnOutOfCombat)
                                    .then(|| c.side_of(seat))
                                    .flatten()
                            }) {
                                if !c.allows(st, mode, i) {
                                    continue;
                                }
                                // Coordinate Attacks: no strike card this
                                // round.
                                if c.sides[i].no_strike_cards_round == Some(c.round)
                                    && mode.effects.iter().any(|e| matches!(e, Effect::Strike(_)))
                                {
                                    continue;
                                }
                                // Under a Fortitude-proof strike, no
                                // prevention card requiring Fortitude this
                                // round, even to cycle (LSJ 20050628).
                                let prevention = mode.effects.iter().any(|e| {
                                    matches!(
                                        e,
                                        Effect::Prevent(_)
                                            | Effect::PreventXPlus(_)
                                            | Effect::PreventNonAggravated(_)
                                            | Effect::PreventNonGun(_)
                                    )
                                });
                                if c.sides[i].no_fortitude_prevention
                                    && prevention
                                    && tags.iter().any(|t| t.eq_ignore_ascii_case("for"))
                                {
                                    continue;
                                }
                            }
                        }
                    }
                    // Karl Schrekt: a card whose title requirement he meets
                    // as a justicar sees a titled Camarilla vampire (owner,
                    // 2026-10-09: rules-qa 59.14).
                    let as_justicar = actor.is_some_and(|m| {
                        st.printed_mod(m, cards::Stat::PlaysAsJusticar, Default::default()) > 0
                    }) && mode.requires.iter().any(|r| {
                        matches!(r, Requirement::ActorTitleIn(cs)
                            if cs.iter().any(|c| matches!(c, cards::TitleClass::Justicar | cards::TitleClass::Any)))
                    });
                    if !mode.requires.iter().all(|&r| {
                        (as_justicar
                            && matches!(r, Requirement::ActorSectIn(ss) if ss.contains(&crate::crypt::Sect::Camarilla)))
                            || as_vampire.is_some_and(|g| match r {
                                Requirement::ActorSectIn(ss) => {
                                    ss.contains(&crate::crypt::Sect::Independent)
                                }
                                Requirement::ActorCapacityAtLeast(n) => g.capacity >= n,
                                _ => false,
                            })
                            || cards::holds(
                                r,
                                st,
                                seat,
                                actor,
                                played,
                                action,
                                combat,
                                referendum,
                                mode.waives_needed,
                            )
                    }) {
                        continue;
                    }
                    let pool = st.seats[seat.index()].pool;
                    let declares_action = mode
                        .effects
                        .iter()
                        .any(|e| matches!(e, Effect::DeclareAction(_)));
                    // "Beast cannot play action cards": every action card —
                    // action, political action, ally, equipment, retainer
                    // (owner, 2026-10-09: rules-qa 60.4).
                    if LIBRARY[card as usize].types.iter().any(|t| {
                        matches!(
                            t,
                            CardType::Action
                                | CardType::PoliticalAction
                                | CardType::Ally
                                | CardType::Equipment
                                | CardType::Retainer
                        )
                    }) && actor.is_some_and(|m| {
                        st.printed_mod(m, cards::Stat::NoActionCards, Default::default()) > 0
                    }) {
                        continue;
                    }
                    let targets: Vec<Target> = match mode.target {
                        None => vec![Target::None],
                        Some(spec) => match spec.kind {
                            // A vampire in the player's uncontrolled region
                            // (Govern the Unaligned [DOM]).
                            TargetKind::Uncontrolled => {
                                (0..st.seats[seat.index()].uncontrolled.len())
                                    .filter(|&i| {
                                        self.uncontrolled_target_qualifies(seat, spec, actor, i)
                                    })
                                    .map(|i| Target::Uncontrolled(i as u8))
                                    .collect()
                            }
                            // A card in play (Far Mastery's retainer).
                            TargetKind::Card => (0..st.cards.len())
                                .map(|i| CardId(i as u32))
                                .filter(|&id| self.card_target_qualifies(seat, spec, actor, id))
                                .map(Target::Card)
                                .collect(),
                            TargetKind::Minion => (0..st.minions.len())
                                .map(|i| MinionId(i as u32))
                                .filter(|&id| {
                                    st.minions[id.index()].present()
                                        && shielded != Some(id)
                                        // "Cannot be the target of directed
                                        // actions requiring Dominate or
                                        // Presence" (Aggressive Corpse).
                                        && !(declares_action
                                            && st.minions[id.index()].owner != seat
                                            && st.immune_to_directed(id, tags))
                                        // "Minions without Auspex cannot
                                        // perform actions directed at this
                                        // vampire" (Shadow Cloak [OBL]).
                                        && !(declares_action
                                            && st.minions[id.index()].owner != seat
                                            && actor.is_some_and(|a| !st.may_direct_at(a, id)))
                                        // An action to enter combat with a
                                        // minion its actor shuns (Reginald
                                        // Moore and the Tremere).
                                        && !(mode.effects.contains(&Effect::EnterCombatWithTarget)
                                            && actor.is_some_and(|a| st.shuns(a, id)))
                                        && spec.filter.iter().all(|&r| match r {
                                            // A vampire playing as the clan
                                            // the card requires (Lady
                                            // Scarlett, Toreador Grand
                                            // Ball's Toreador: ANK
                                            // 20220816-2).
                                            Requirement::TargetClan(c)
                                                if LIBRARY[card as usize]
                                                    .clans
                                                    .contains(&c) =>
                                            {
                                                st.meets_clan(id, &[c], actor != Some(id))
                                            }
                                            _ => cards::holds_for_minion(
                                                r,
                                                st,
                                                seat,
                                                referendum,
                                                action.map(|a| a.acting).or(actor),
                                                id,
                                                action,
                                            ),
                                        })
                                        // "Unlock a locked Anarch you
                                        // control; that Anarch attempts to
                                        // block" (engine-checked).
                                        && (!spec.filter.contains(&Requirement::CanAttemptBlock)
                                            || self.can_attempt_block(seat, id))
                                        && (!spec
                                            .filter
                                            .contains(&Requirement::TargetInCombatWithDamage)
                                            || self.combat.as_ref().is_some_and(|c| {
                                                c.side_of_minion(id).is_some_and(|t| {
                                                    c.sides[t].pending.total() > 0
                                                        && !(c.sides[t].no_fortitude_prevention
                                                            && tags.iter().any(|g| {
                                                                g.eq_ignore_ascii_case("for")
                                                            }))
                                                })
                                            }))
                                })
                                .map(Target::Minion)
                                .collect(),
                            TargetKind::Seat => (0..st.seats.len())
                                .map(|i| SeatId(i as u8))
                                .filter(|&t| {
                                    spec.filter
                                        .iter()
                                        .all(|&r| cards::holds_for_seat(r, st, seat, action, t))
                                })
                                .map(Target::Seat)
                                .collect(),
                        },
                    };
                    let discount = actor.map_or(0, |m| recruit_off(st, mode, m)) as i32;
                    for target in targets {
                        let potential = self
                            .cost_potential(&PlayCtx::new(seat, card, mi as u8, actor, target, 0))
                            as i32;
                        // Less the minion's printed reductions (Anne-Marie
                        // Bourgeois, Kuyén, Máddji).
                        let mut cost = cards::pool_cost(st, seat, card, target);
                        if let Some(m) = actor {
                            st.printed_cost_off(m, card, mi as u8)
                                .apply(&mut cost, &mut cards::blood_cost(st, card, 0));
                        }
                        if cost - discount - potential <= pool {
                            out.push(Choice::Play {
                                card: hi as u8,
                                mode: mi as u8,
                                actor,
                                target,
                            });
                        }
                    }
                }
            }
        }
        out.retain(|o| match *o {
            Choice::Play { card, actor, .. } if card as usize >= hand_len => {
                let clan = store_clan(card as usize);
                match actor {
                    // A store of no clan (The Erciyes Fragments): as if
                    // from hand, by any minion of its controller's or by
                    // the Methuselah.
                    None => clan == Some(""),
                    Some(m) => {
                        let v = &st.minions[m.index()];
                        v.owner == seat && (clan == Some("") || clan.is_some() && v.clan() == clan)
                    }
                }
            }
            _ => true,
        });
        out
    }

    pub(crate) fn master_request(&self) -> DecisionRequest {
        let s = self.state.active;
        let mut options = vec![Choice::Pass];
        if self.state.mpa_left(s) > 0 {
            options.extend(self.play_options(s, &[Window::MasterPhase], None));
            // Rulebook "Red List": a master phase action marks another's
            // Red List minion for the turn.
            options.extend(
                self.red_list_markable(s)
                    .into_iter()
                    .map(Choice::ChooseMinion),
            );
        }
        // "During their master phase" abilities of cards in play (Blood
        // Doll): no master phase action spent; and the untimed ones.
        options.extend(self.ability_options(s, Window::MasterPhase));
        options.extend(self.ability_options(s, Window::AnyImpulse));
        // A mandatory effect due (Brujah Debate): ordered among the rest,
        // no passing before it (rules-qa 33.1, 40.5).
        let due = self.master_dues();
        if !due.is_empty() {
            options.retain(|o| *o != Choice::Pass);
            options.extend(due.into_iter().map(|c| Choice::ResolveRider { card: c.0 }));
        }
        DecisionRequest::new(s, WindowLabel::MasterPhase, options)
    }

    /// The Red List minions `s` may mark: another Methuselah's, in play,
    /// not marked by `s` yet this turn.
    pub(crate) fn red_list_markable(&self, s: SeatId) -> Vec<MinionId> {
        let st = &self.state;
        (0..st.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(|&m| {
                let v = &st.minions[m.index()];
                v.present()
                    && v.owner != s
                    && v.vampire().is_some_and(|c| c.red_list)
                    && !st.red_list_marks.contains(&(s, m))
            })
            .collect()
    }

    pub(crate) fn master_apply(&mut self, choice: &Choice) {
        match choice {
            Choice::Pass => self.open_phase_rotation(Flow::MasterPhase),
            Choice::ChooseMinion(m) => {
                let s = self.state.active;
                self.state.spend_mpa(s);
                self.state.red_list_marks.push((s, *m));
                self.flow = Flow::MasterPhase;
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                let s = self.state.active;
                self.state.spend_mpa(s);
                // Played as announced, canceled or not.
                self.state.seats[s.index()].master_played = true;
                self.announce_play(s, *card, *mode, *actor, *target, Flow::MasterPhase);
            }
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                let s = self.state.active;
                if !self.use_ability(s, *source, *ability, *actor, *target, Flow::MasterPhase) {
                    self.flow = Flow::MasterPhase;
                }
            }
            Choice::ResolveRider { card } => {
                let (s, id) = (self.state.active, CardId(*card));
                self.state.seats[s.index()].master_resolved.push(id);
                self.flow = Flow::MasterPhase;
                let def = cards::lookup(self.state.cards[id.index()].card).expect("definition");
                for r in def
                    .riders
                    .iter()
                    .filter(|r| r.hook == Hook::MasterPhaseStart)
                {
                    for e in r.effects {
                        match *e {
                            Effect::LockOldestOfClan(clan) => {
                                self.interrupts.push(Interrupt::MinionOf {
                                    seat: s,
                                    of: s,
                                    purpose: crate::draws::MinionPurpose::LockOldest(clan),
                                });
                            }
                            other => unreachable!("not a master-phase rider effect: {other:?}"),
                        }
                    }
                }
            }
            other => unreachable!("illegal master phase choice {other:?}"),
        }
    }

    /// Announce a play (rulebook "Playing Cards"): out-of-turn accounting
    /// charged even if canceled, terms, then the as-played window; the flow
    /// returns to `resume` once it settles.
    pub(crate) fn announce_play(
        &mut self,
        s: SeatId,
        hand: u8,
        mode: u8,
        actor: Option<MinionId>,
        target: Target,
        resume: Flow,
    ) {
        let (card, from) = self.virtual_hand(s)[hand as usize];
        // "Each time a card is played this combat, reset the number of
        // counters on this card to 3" (Trap, Conquer the Beast).
        if let Some(c) = self.combat.as_ref() {
            for &(id, _) in &c.traps {
                if self.state.cards[id.index()].present() {
                    self.state.cards[id.index()].counters = 3;
                }
            }
        }
        let copy = match from {
            None => self.state.seats[s.index()].hand.take(hand as usize).1,
            // Spiritual Guidance's play, from the ash heap.
            Some(crate::state::ASH_PLAY) => {
                let seat = &mut self.state.seats[s.index()];
                let k = seat.ash.iter().position(|&c| c == card).expect("ash card");
                seat.ash_combat_play = false;
                seat.ash.take(k).1
            }
            Some(loc) => {
                let stored = &mut self.state.cards[loc.index()].stored;
                let k = stored.iter().position(|&c| c == card).expect("stored card");
                stored.take(k).1
            }
        };
        let def = cards::lookup(card).expect("played card has a definition");
        // The Disciplines a minion uses to play cards during an action,
        // canceled ones too (Dabbler; ANK 20220411).
        if let (Some(a), Some(m)) = (self.action.as_mut(), actor) {
            for t in LIBRARY[card as usize].modes[cards::library_mode(card, mode as usize)].tags {
                a.disciplines_played.push((m, cards::discipline_key(t)));
            }
        }
        self.freeze_auspex_foes();
        // "Only one … can be played in a game": played, even if canceled;
        // an event once each game (rulebook "Event Cards").
        if def.modes[mode as usize]
            .requires
            .contains(&Requirement::NotPlayedThisGame)
            || LIBRARY[card as usize].types.contains(&CardType::Event)
        {
            self.state.once_played.push(card);
        }
        if def.modes[mode as usize]
            .requires
            .contains(&Requirement::NotPlayedThisTurn)
        {
            self.state.turn_played.push(card);
        }
        // An action modifier or a reaction, once per minion per action.
        if let (Some(a), Some(m), None) = (self.action.as_mut(), actor, self.combat.as_ref()) {
            if once_per_action(card) {
                a.played.push((m, LIBRARY[card as usize].name));
            }
        }
        if let (Some(limit), Some(m)) = (def.modes[mode as usize].limit, actor) {
            let name = LIBRARY[card as usize].name;
            match limit {
                Limit::PerRound => {
                    if let Some(c) = self.combat.as_mut() {
                        c.played_round.push((m, name));
                    }
                }
                Limit::PerCombat => {
                    if let Some(c) = self.combat.as_mut() {
                        c.played_combat.push((m, name));
                    }
                }
                Limit::PerCombatThisMode => {
                    if let Some(c) = self.combat.as_mut() {
                        c.played_combat_modes.push((m, card, mode));
                    }
                }
                Limit::PerAction | Limit::OnePerAction => {
                    if let Some(a) = self.action.as_mut() {
                        a.played.push((m, name));
                    }
                }
                Limit::BetweenUnlockPhases => {
                    self.state.minions[m.index()].played_since_unlock.push(card);
                }
                Limit::PerTurn => self.state.turn_plays.push((m, card)),
                Limit::PerPhaseEach
                | Limit::PerReferendum
                | Limit::PerActionCard
                | Limit::PerCombatCard
                | Limit::TwicePerCombat => {}
            }
        }
        if def.out_of_turn {
            self.state.seats[s.index()].out_of_turn = Some(OutOfTurn { trifle: def.trifle });
        }
        // A reaction card played during an action, canceled or not
        // (Perfectionist; LSJ 20070330).
        if LIBRARY[card as usize].types.contains(&CardType::Reaction) {
            if let Some(a) = self.action.as_mut() {
                a.reaction_played = true;
            }
        }
        let mut ctx = PlayCtx {
            from,
            copy: crate::copies::Untracked(copy),
            ..PlayCtx::new(s, card, mode, actor, target, 0)
        };
        if let Some(x) = cards::terms_x(&self.state, s, card, &def.modes[mode as usize]) {
            ctx.x = x;
        }
        // Cost reductions to announce with the card, if any apply.
        if self.cost_choices(&ctx) {
            let resume = match std::mem::replace(&mut self.flow, Flow::GameOver) {
                outer @ Flow::AsPlayed { .. } => outer,
                _ => resume,
            };
            self.flow = Flow::CostTerms {
                ctx,
                resume: Box::new(resume),
                pending: None,
                then: CostThen::Terms,
            };
            return;
        }
        self.announce_terms(ctx, resume);
    }

    /// The rest of a play's announcement: its terms, then its as-played
    /// window.
    fn announce_terms(&mut self, ctx: PlayCtx, resume: Flow) {
        let def = cards::lookup(ctx.card).expect("played card has a definition");
        let mode = ctx.mode;
        match def.modes[mode as usize].terms {
            Some(Terms::CopiesControlled) => self.open_as_played(ctx, resume),
            Some(_) => {
                // A nested play keeps the outer as-played window's stack as
                // its resume point.
                let resume = match std::mem::replace(&mut self.flow, Flow::GameOver) {
                    outer @ Flow::AsPlayed { .. } => outer,
                    _ => resume,
                };
                self.flow = Flow::PlayTerms {
                    ctx,
                    resume: Box::new(resume),
                }
            }
            None => self.open_as_played(ctx, resume),
        }
    }

    /// The cards `seat` may play: its hand, then the cards on its
    /// Fleshforge Chambers (with the location they sit on).
    pub fn virtual_hand(&self, seat: SeatId) -> Vec<(u16, Option<CardId>)> {
        let st = &self.state;
        let mut out: Vec<(u16, Option<CardId>)> = st.seats[seat.index()]
            .hand
            .iter()
            .map(|&c| (c, None))
            .collect();
        for (ci, c) in st.cards.iter().enumerate() {
            let id = CardId(ci as u32);
            if c.present()
                && st.card_controller(id) == seat
                && cards::lookup(c.card).is_some_and(|d| {
                    d.modes[0]
                        .effects
                        .iter()
                        .any(|e| matches!(e, Effect::PlayableFromStore(_)))
                })
            {
                out.extend(c.stored.iter().map(|&s| (s, Some(id))));
            }
        }
        // Spiritual Guidance: the ash heap's combat cards, one per name.
        if st.seats[seat.index()].ash_combat_play {
            let ash = &st.seats[seat.index()].ash;
            out.extend(
                crate::state::distinct(ash, |c| {
                    LIBRARY[c as usize].types.contains(&CardType::Combat)
                })
                .into_iter()
                .map(|c| (c, Some(crate::state::ASH_PLAY))),
            );
        }
        out
    }

    /// The cost reductions that could apply to `ctx`'s card, not yet
    /// picked: the source and what it takes off at most.
    pub(crate) fn cost_sources(&self, ctx: &PlayCtx) -> Vec<(CardId, u8)> {
        let st = &self.state;
        let Some(def) = cards::lookup(ctx.card) else {
            return Vec::new();
        };
        let played_mode = &def.modes[ctx.mode as usize];
        // A card that plays another from hand (Piper, Contraband): the
        // reductions are for the card it names, once named.
        let names = played_mode
            .effects
            .iter()
            .any(|e| matches!(e, Effect::EmployNamed | Effect::EquipNamed));
        let (card, mode) = match (names, ctx.named) {
            (true, None) => return Vec::new(),
            (true, Some((c, m))) => match cards::lookup(c) {
                Some(d) => (c, &d.modes[m as usize]),
                None => return Vec::new(),
            },
            (false, _) => (ctx.card, played_mode),
        };
        let action_card = mode
            .effects
            .iter()
            .any(|e| matches!(e, Effect::DeclareAction(_)));
        let equip = mode
            .effects
            .contains(&Effect::DeclareAction(ActionKind::Equip));
        let (blood, pool) = if names {
            (
                cards::blood_cost(st, card, 0),
                ctx.actor
                    .map_or(0, |m| {
                        cards::pool_cost(st, ctx.seat, card, Target::Minion(m))
                    })
                    .max(0) as u8,
            )
        } else {
            (
                cards::blood_cost(st, card, ctx.x),
                cards::pool_cost(st, ctx.seat, card, ctx.target).max(0) as u8,
            )
        };
        let actor = ctx.actor.map(|m| &st.minions[m.index()]);
        let vampire = actor.is_some_and(|v| v.is_vampire());
        let mut out = Vec::new();
        for (ci, c) in st.cards.iter().enumerate() {
            let id = CardId(ci as u32);
            if !c.present()
                || st.card_controller(id) != ctx.seat
                || ctx.discounts.iter().flatten().any(|(d, _)| *d == id)
            {
                continue;
            }
            let n = match st.cost_source_of(id) {
                Some(cards::CostSource::Line)
                    if !c.locked && action_card && vampire && blood > 0 =>
                {
                    1
                }
                Some(cards::CostSource::Shard)
                    if !c.locked
                        && st.seats[ctx.seat.index()].has_edge
                        && (blood > 0 || pool > 0) =>
                {
                    2
                }
                // Not a political action card's cost (LSJ 20091021).
                Some(cards::CostSource::Carnival(clan))
                    if action_card
                        && actor.is_some_and(|v| v.clan() == Some(clan))
                        && blood > 0
                        && !mode
                            .effects
                            .contains(&Effect::DeclareAction(ActionKind::Political)) =>
                {
                    c.counters.min(blood)
                }
                Some(cards::CostSource::Cache) if !c.locked && equip && (blood > 0 || pool > 0) => {
                    c.counters.min(blood.max(pool))
                }
                Some(cards::CostSource::SunsetStrip)
                    if !c.locked && cards::mortal_hire(card) && (blood > 0 || pool > 0) =>
                {
                    1
                }
                // Focus the Blood: a combat card its host plays.
                Some(cards::CostSource::FocusBlood)
                    if c.host.is_some()
                        && c.host == ctx.actor
                        && blood > 0
                        && LIBRARY[card as usize].types.contains(&CardType::Combat)
                        && mode.windows.iter().all(|w| w.is_combat()) =>
                {
                    1
                }
                // An Anarch's equipment (Hackerspace).
                Some(cards::CostSource::Hackerspace)
                    if equip
                        && ctx
                            .actor
                            .is_some_and(|m| st.sect_of(m) == Some(crate::crypt::Sect::Anarch))
                        && (blood > 0 || pool > 0) =>
                {
                    c.counters.min(blood.max(pool))
                }
                _ => 0,
            };
            if n > 0 {
                out.push((id, n));
            }
        }
        out
    }

    /// The vampires whose blood may pay 1 of `ctx`'s card's cost (Regina
    /// Blake): a location, in torpor too (LSJ 20011022), a locquipment as
    /// equipment (LSJ 20080107), one another card puts in play (rules-qa
    /// 41.13).
    pub(crate) fn blood_payers(&self, ctx: &PlayCtx) -> Vec<MinionId> {
        let st = &self.state;
        // A card playing another (Piper, Contraband): the one it names.
        let names = cards::lookup(ctx.card).is_some_and(|d| {
            d.modes[ctx.mode as usize]
                .effects
                .iter()
                .any(|e| matches!(e, Effect::EmployNamed | Effect::EquipNamed))
        });
        let (card, mode, target) = match (names, ctx.named) {
            (true, None) => return Vec::new(),
            (true, Some((c, m))) => (c, m, ctx.actor.into()),
            (false, _) => (ctx.card, ctx.mode, ctx.target),
        };
        if ctx.blood_payer.is_some() {
            return Vec::new();
        }
        let costs = cards::pool_cost(st, ctx.seat, card, target) > 0
            || cards::blood_cost(st, card, ctx.x) > 0;
        let pctx = crate::state::PrintedCtx {
            card: Some((card, mode)),
            ..Default::default()
        };
        st.minions_of(ctx.seat)
            .filter(|&m| {
                costs
                    && st.minions[m.index()].blood > 0
                    && st.printed_mod(m, cards::Stat::BloodPaysCost, pctx) > 0
            })
            .collect()
    }

    /// The vampires that may lock to take 1 pool off a location master
    /// card their Methuselah plays (Jack), in torpor too (LSJ 20011022):
    /// none once one is announced.
    pub(crate) fn location_lockers(&self, ctx: &PlayCtx) -> Vec<MinionId> {
        let st = &self.state;
        let lib = &LIBRARY[ctx.card as usize];
        if ctx.locker.is_some()
            || !lib.types.contains(&CardType::Master)
            || !lib.labels.contains(&Label::Location)
        {
            return Vec::new();
        }
        let pctx = crate::state::PrintedCtx {
            card: Some((ctx.card, ctx.mode)),
            ..Default::default()
        };
        st.minions_of(ctx.seat)
            .filter(|&m| {
                !st.minions[m.index()].locked
                    && st.printed_mod(m, cards::Stat::LockForLocationDiscount, pctx) > 0
            })
            .collect()
    }

    /// Anything to announce with the card: a reduction, a vampire's blood.
    pub(crate) fn cost_choices(&self, ctx: &PlayCtx) -> bool {
        !self.cost_sources(ctx).is_empty()
            || !self.blood_payers(ctx).is_empty()
            || self.once_off_available(ctx)
            || !self.location_lockers(ctx).is_empty()
    }

    /// What reduction `c` could take off `ctx`'s card at most.
    fn cost_source_max(&self, ctx: &PlayCtx, c: CardId) -> u8 {
        self.cost_sources(ctx)
            .iter()
            .find(|(d, _)| *d == c)
            .map_or(0, |&(_, n)| n)
    }

    /// What the reductions could still take off `ctx`'s card (its
    /// affordability as it is offered).
    pub(crate) fn cost_potential(&self, ctx: &PlayCtx) -> u8 {
        self.cost_sources(ctx).iter().map(|&(_, n)| n).sum::<u8>()
            + u8::from(!self.blood_payers(ctx).is_empty())
            + u8::from(self.once_off_available(ctx))
            + u8::from(!self.location_lockers(ctx).is_empty())
    }

    pub(crate) fn cost_terms_request(
        &self,
        ctx: &PlayCtx,
        pending: Option<CardId>,
    ) -> DecisionRequest {
        let options = match pending {
            // How many counters (Ravnos Carnival, Ravnos Cache).
            Some(c) => (1..=self.cost_source_max(ctx, c))
                .map(Choice::SetX)
                .collect(),
            None => {
                let mut options = Vec::new();
                if self.affordable_with_discounts(ctx) {
                    options.push(Choice::Pass);
                }
                options.extend(
                    self.cost_sources(ctx)
                        .into_iter()
                        .map(|(c, _)| Choice::ResolveRider { card: c.0 }),
                );
                options.extend(self.blood_payers(ctx).into_iter().map(Choice::ChooseMinion));
                // Nergal: his blood off, for this card (rules-qa 60.7).
                if let (true, Some(m)) = (self.once_off_available(ctx), ctx.actor) {
                    options.push(Choice::ResolvePrinted { minion: m });
                }
                // Jack: locked for 1 pool off a location.
                options.extend(
                    self.location_lockers(ctx)
                        .into_iter()
                        .map(|m| Choice::ResolvePrinted { minion: m }),
                );
                options
            }
        };
        DecisionRequest::new(ctx.seat, WindowLabel::PlayTerms, options)
    }

    /// Could the card's cost be paid with every reduction it may take
    /// (British Museum's, then those announced: a searched equipment,
    /// rules-qa 38.5)? A pool cost paid down to 0 counts; with
    /// `pool_may_oust`, any pool cost does (Vast Wealth, RTR 19941109).
    pub(crate) fn payable_with_potential(&self, ctx: &PlayCtx, pool_may_oust: bool) -> bool {
        let st = &self.state;
        let m = ctx.actor.expect("minion");
        let off = self.cost_potential(ctx) + st.equip_discount(m, ctx.card);
        let mut blood = cards::blood_cost(st, ctx.card, ctx.x);
        let mut pool = cards::pool_cost(st, ctx.seat, ctx.card, ctx.target);
        st.printed_cost_off(m, ctx.card, ctx.mode)
            .apply(&mut pool, &mut blood);
        let have = (st.seats[ctx.seat.index()].pool, st.minions[m.index()].blood);
        payable((pool, blood), off, have, pool_may_oust)
    }

    /// Can the card's cost be paid with the reductions picked so far (an
    /// unaffordable card must announce them: LSJ 20090705)?
    fn affordable_with_discounts(&self, ctx: &PlayCtx) -> bool {
        let st = &self.state;
        let off: u8 = ctx.discounts.iter().flatten().map(|&(_, n)| n).sum::<u8>()
            + u8::from(ctx.blood_payer.is_some())
            + u8::from(ctx.once_off)
            + u8::from(ctx.locker.is_some())
            + ctx
                .actor
                .zip(cards::lookup(ctx.card))
                .map_or(0, |(m, d)| recruit_off(st, &d.modes[ctx.mode as usize], m));
        let blood = cards::blood_cost(st, ctx.card, ctx.x);
        let pool = cards::pool_cost(st, ctx.seat, ctx.card, ctx.target);
        // An ally has no blood to pay with, unless playing this card "as a
        // vampire".
        let have_blood = ctx.actor.map_or(u8::MAX, |m| {
            let v = &st.minions[m.index()];
            if v.is_ally()
                && cards::plays_card_as_vampire(st, m, ctx.card, ctx.mode as usize).is_none()
            {
                0
            } else {
                v.blood
            }
        });
        payable(
            (pool, blood),
            off,
            (st.seats[ctx.seat.index()].pool, have_blood),
            false,
        )
    }

    pub(crate) fn cost_terms_apply(
        &mut self,
        mut ctx: PlayCtx,
        resume: Flow,
        pending: Option<CardId>,
        then: CostThen,
        choice: &Choice,
    ) {
        let counter_source = |e: &Engine, c: CardId| {
            matches!(
                e.state.cost_source_of(c),
                Some(
                    cards::CostSource::Carnival(_)
                        | cards::CostSource::Cache
                        | cards::CostSource::Hackerspace
                )
            )
        };
        match (pending, choice) {
            (Some(c), Choice::SetX(n)) => {
                if let Some(slot) = ctx.discounts.iter_mut().find(|d| d.is_none()) {
                    *slot = Some((c, *n));
                }
            }
            (None, Choice::ResolveRider { card }) => {
                let c = CardId(*card);
                if counter_source(self, c) {
                    self.flow = Flow::CostTerms {
                        ctx,
                        resume: Box::new(resume),
                        pending: Some(c),
                        then,
                    };
                    return;
                }
                let n = self.cost_source_max(&ctx, c);
                if let Some(slot) = ctx.discounts.iter_mut().find(|d| d.is_none()) {
                    *slot = Some((c, n));
                }
            }
            (None, Choice::ChooseMinion(m)) => ctx.blood_payer = Some(*m),
            (None, Choice::ResolvePrinted { minion })
                if self.location_lockers(&ctx).contains(minion) =>
            {
                ctx.locker = Some(*minion)
            }
            (None, Choice::ResolvePrinted { .. }) => ctx.once_off = true,
            (None, Choice::Pass) => {
                self.cost_terms_done(ctx, resume, then);
                return;
            }
            (p, c) => unreachable!("illegal cost terms choice {c:?} ({p:?})"),
        }
        // More to pick, or done when nothing else applies.
        if !self.cost_choices(&ctx) {
            self.cost_terms_done(ctx, resume, then);
        } else {
            self.flow = Flow::CostTerms {
                ctx,
                resume: Box::new(resume),
                pending: None,
                then,
            };
        }
    }

    /// A named card's cost after the reductions announced with the card
    /// that played it, those taken now: The Line and The Shard locked, the
    /// counters spent for what they cover ("blood or pool" first, then
    /// blood-only, then counters: rules-qa 36.2) — after the paying
    /// minion's printed ones (Kuyén: ANK 20210913).
    pub(crate) fn pay_named_discounts(&mut self, ctx: &PlayCtx, pool: i32, blood: u8) -> (i32, u8) {
        let (mut pool, mut blood) = (pool, blood);
        if let Some(m) = ctx.actor {
            let (card, mode) = ctx.named.unwrap_or((ctx.card, ctx.mode));
            self.state
                .printed_cost_off(m, card, mode)
                .apply(&mut pool, &mut blood);
        }
        let (either, blood_off, counters) = self.take_discounts(ctx);
        reduce(&mut pool, &mut blood, either);
        blood = blood.saturating_sub(blood_off);
        let mut used = Vec::new();
        for (c, n) in counters {
            let avail = n.min(self.state.cards[c.index()].counters);
            let k = match self.state.cost_source_of(c) {
                Some(cards::CostSource::Cache | cards::CostSource::Hackerspace) if pool > 0 => {
                    let k = avail.min(pool as u8);
                    pool -= k as i32;
                    k
                }
                _ => {
                    let k = avail.min(blood);
                    blood -= k;
                    k
                }
            };
            used.push((c, k));
        }
        self.spend_counters(&used);
        // A vampire's blood paying 1 of what is left (Regina Blake).
        if let Some(r) = self.blood_payment(ctx.blood_payer, &mut pool, &mut blood) {
            self.state.minions[r.index()].blood -= 1;
        }
        (pool, blood)
    }

    /// The reductions announced: the card's terms next, or (named by them
    /// already) its as-played window.
    fn cost_terms_done(&mut self, ctx: PlayCtx, resume: Flow, then: CostThen) {
        match (then, resume) {
            (CostThen::Terms, r @ Flow::AsPlayed { .. }) => {
                self.flow = r;
                self.announce_terms(ctx, Flow::GameOver);
            }
            (CostThen::Terms, r) => self.announce_terms(ctx, r),
            (CostThen::AsPlayed, r @ Flow::AsPlayed { .. }) => {
                self.flow = r;
                self.open_as_played(ctx, Flow::GameOver);
            }
            (CostThen::AsPlayed, r) => self.open_as_played(ctx, r),
            (CostThen::EquipFound, _) => self.equip_found(ctx),
        }
    }

    /// May `actor` of `seat` take an action at `target` — not an action
    /// directed at a vampire it may not direct actions at (Shadow Cloak
    /// [OBL])?
    pub(crate) fn may_direct(&self, seat: SeatId, actor: MinionId, target: Target) -> bool {
        match target {
            Target::Minion(t) => {
                self.state.minions[t.index()].owner == seat || self.state.may_direct_at(actor, t)
            }
            _ => true,
        }
    }

    /// The extra blood or life the next reaction card costs this action
    /// (Unleashing the Bestial Soul [DOM]).
    fn reaction_surcharge(&self, card: u16) -> u8 {
        match &self.action {
            Some(a) if LIBRARY[card as usize].types.contains(&CardType::Reaction) => {
                a.reaction_surcharge
            }
            _ => 0,
        }
    }

    /// Equipment cards in `seat`'s hand minion `m` could be equipped with
    /// now (Angel's Gift, Contraband): matching `filter`, requirements met,
    /// cost payable — its pool cost partly in blood with `blood_share`.
    pub(crate) fn equippable_from_hand(
        &self,
        seat: SeatId,
        m: MinionId,
        filter: cards::CardFilter,
        blood_share: bool,
    ) -> Vec<(u16, u8)> {
        let st = &self.state;
        distinct(&st.seats[seat.index()].hand, |_| true)
            .into_iter()
            .filter(|&card| {
                let mut pool = cards::pool_cost(st, seat, card, Target::Minion(m));
                let mut blood = cards::blood_cost(st, card, 0);
                st.printed_cost_off(m, card, 0).apply(&mut pool, &mut blood);
                let have = st.minions[m.index()].blood;
                let share = if blood_share {
                    (pool as u8 / 2).min(have.saturating_sub(blood)) as i32
                } else {
                    0
                };
                // Concealed Weapon: "cannot cost 3 or more pool", its cost
                // as modified in play (LSJ 20040701), announced reductions
                // and other sources aside (LSJ 20080125).
                let cheap = filter != cards::CardFilter::ConcealableWeapon || pool < 3;
                filter.matches(card)
                    && cheap
                    && cards::lookup(card).is_some()
                    && self.equip_requirements_met(m, card)
                    && have >= blood
                    && st.seats[seat.index()].pool >= pool - share
            })
            .map(|card| (card, 0))
            .collect()
    }

    /// The most of `card`'s pool cost minion `m` may pay in blood
    /// (Contraband [OBF]: "up to half the cost rounded down").
    fn blood_share_max(&self, seat: SeatId, m: MinionId, card: u16) -> u8 {
        let st = &self.state;
        let pool = cards::pool_cost(st, seat, card, Target::Minion(m));
        let have = st.minions[m.index()]
            .blood
            .saturating_sub(cards::blood_cost(st, card, 0));
        (pool as u8 / 2).min(have)
    }

    /// The least of `card`'s pool cost minion `m` must pay in blood: what
    /// the pool cannot.
    fn blood_share_min(&self, seat: SeatId, m: MinionId, card: u16) -> u8 {
        let st = &self.state;
        let pool = cards::pool_cost(st, seat, card, Target::Minion(m));
        (pool - st.seats[seat.index()].pool).max(0) as u8
    }

    /// The `label` retainers in `seat`'s hand minion `m` could employ now,
    /// by card and mode, requirements met and cost affordable (Pack Alpha);
    /// with `ignore`, no clan or Discipline requirement: the basic version
    /// always, another as its Disciplines allow (Biothaumaturgic
    /// Experiment; LSJ 20080803).
    pub(crate) fn employable_from_hand(
        &self,
        seat: SeatId,
        m: MinionId,
        label: Option<crate::library::Label>,
        ignore: bool,
    ) -> Vec<(u16, u8)> {
        let st = &self.state;
        let mut out = Vec::new();
        for card in distinct(&st.seats[seat.index()].hand, |_| true) {
            let lib = &LIBRARY[card as usize];
            // A retainer of the label, or (none) any ally or retainer.
            let fits = match label {
                Some(l) => lib.types.contains(&CardType::Retainer) && lib.labels.contains(&l),
                None => {
                    lib.types.contains(&CardType::Retainer) || lib.types.contains(&CardType::Ally)
                }
            };
            if !fits {
                continue;
            }
            let Some(def) = cards::lookup(card) else {
                continue;
            };
            if st.controls_unique_copy(seat, card)
                || !(ignore
                    || lib.clans.is_empty()
                    || st.minions[m.index()]
                        .clan()
                        .is_some_and(|c| lib.clans.contains(&c)))
            {
                continue;
            }
            let mut pool = cards::pool_cost(st, seat, card, Target::Minion(m));
            let mut blood = cards::blood_cost(st, card, 0);
            st.printed_cost_off(m, card, 0).apply(&mut pool, &mut blood);
            if st.seats[seat.index()].pool < pool || st.minions[m.index()].blood < blood {
                continue;
            }
            for (mi, _) in def.modes.iter().enumerate() {
                if (ignore && mi == 0) || cards::has_disciplines(st, m, card, mi) {
                    out.push((card, mi as u8));
                }
            }
        }
        out
    }

    pub(crate) fn terms_request(&self, ctx: &PlayCtx) -> DecisionRequest {
        let def = cards::lookup(ctx.card).expect("definition");
        let options = match def.modes[ctx.mode as usize].terms {
            // The amount is announced in the printed range; at resolution
            // you take what you can (Villein rulings, LSJ 20100216).
            Some(Terms::BloodRange { min, max }) => (min..=max).map(Choice::SetX).collect(),
            Some(Terms::BoonPool) => {
                let Target::Seat(t) = ctx.target else {
                    unreachable!("a Boon without its Methuselah");
                };
                let need = (1 - self.state.seats[t.index()].pool).max(1);
                let max = self.state.seats[ctx.seat.index()].pool - 1;
                (need..=max)
                    .map(|x| Choice::SetX(x.min(u8::MAX as i32) as u8))
                    .collect()
            }
            Some(Terms::UpToTargetBlood) => {
                let t = ctx.target.minion();
                (0..=self.state.minions[t.index()].blood)
                    .map(Choice::SetX)
                    .collect()
            }
            // X may exceed what is needed but never the actor's blood
            // (Hidden Strength rulings); a fixed blood cost besides
            // (Lightning Reflexes [CEL]) is paid first.
            Some(Terms::XBlood) => {
                let blood = self.state.minions[ctx.actor.expect("actor").index()].blood;
                let fixed = cards::blood_cost(&self.state, ctx.card, 0);
                (0..=blood.saturating_sub(fixed))
                    .map(Choice::SetX)
                    .collect()
            }
            Some(Terms::XBloodIn { min, max }) => {
                let blood = self.state.minions[ctx.actor.expect("actor").index()].blood;
                (min..=max.min(blood)).map(Choice::SetX).collect()
            }
            Some(Terms::UpToMargin(n)) => {
                let margin = self.action.as_ref().map_or(0, |a| a.margin) as u8;
                (0..=n.min(margin)).map(Choice::SetX).collect()
            }
            Some(Terms::OwnOfClan { clan, other }) => self
                .state
                .minions_of(ctx.seat)
                .filter(|&m| {
                    let v = &self.state.minions[m.index()];
                    // A vampire playing as the clan the card requires may
                    // be chosen (ANK 20220816-2).
                    let of_clan = if LIBRARY[ctx.card as usize].clans.contains(&clan) {
                        self.state.meets_clan(m, &[clan], true)
                    } else {
                        v.clan() == Some(clan)
                    };
                    v.ready() && of_clan && !(other && ctx.target == Target::Minion(m))
                })
                .map(Choice::ChooseMinion)
                .collect(),
            // The equipment, then its blood share (Contraband [OBF]).
            Some(Terms::EquipFromHand {
                filter,
                blood_share,
            }) => match ctx.named {
                None => self
                    .equippable_from_hand(ctx.seat, ctx.actor.expect("actor"), filter, blood_share)
                    .into_iter()
                    .map(|(card, mode)| Choice::EmployFromHand { card, mode })
                    .collect(),
                // At least what the pool cannot pay.
                Some((card, _)) => {
                    let m = ctx.actor.expect("actor");
                    let max = self.blood_share_max(ctx.seat, m, card);
                    let min = self.blood_share_min(ctx.seat, m, card);
                    (min..=max).map(Choice::SetX).collect()
                }
            },
            Some(Terms::EmployFromHand(label)) => self
                .employable_from_hand(ctx.seat, ctx.actor.expect("actor"), Some(label), false)
                .into_iter()
                .map(|(card, mode)| Choice::EmployFromHand { card, mode })
                .collect(),
            Some(Terms::EmployFromHandIgnoring(label)) => self
                .employable_from_hand(ctx.seat, ctx.actor.expect("actor"), Some(label), true)
                .into_iter()
                .map(|(card, mode)| Choice::EmployFromHand { card, mode })
                .collect(),
            Some(Terms::RecruitOrEmployFromHand) => self
                .employable_from_hand(ctx.seat, ctx.actor.expect("actor"), None, false)
                .into_iter()
                .map(|(card, mode)| Choice::EmployFromHand { card, mode })
                .collect(),
            Some(Terms::TargetCapacity) => {
                let t = ctx.target.minion();
                vec![Choice::SetX(self.state.capacity_of(t).unwrap_or(0))]
            }
            Some(Terms::OwnAshVampire) => {
                crate::state::distinct(&self.state.seats[ctx.seat.index()].ash_crypt, |_| true)
                    .into_iter()
                    .map(Choice::AshVampire)
                    .collect()
            }
            // The ash heap, then the ally in it (Psychophagia).
            Some(Terms::AnyAshAlly) => match ctx.ash {
                None => self
                    .ash_ally_heaps()
                    .into_iter()
                    .map(Choice::ChooseSeat)
                    .collect(),
                Some((s, _)) => self
                    .ash_allies(s, &[])
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect(),
            },
            // Sanguine Instruction: the actor's superior Disciplines the
            // target lacks at superior.
            Some(Terms::SuperiorDiscipline) => {
                let m = ctx.actor.expect("actor");
                let t = match ctx.target {
                    Target::Minion(t) => Some(t),
                    _ => None,
                };
                crate::state::disciplines()
                    .into_iter()
                    .filter(|d| {
                        self.state.discipline_level(m, d) == 2
                            && t.is_none_or(|t| self.state.discipline_level(t, d) < 2)
                    })
                    .map(Choice::ChooseDiscipline)
                    .collect()
            }
            // Slake the Thirst: each amount the actor gained, by one effect.
            Some(Terms::ActorGain) => {
                let m = ctx.actor.expect("actor");
                let mut amounts: Vec<u8> = self
                    .state
                    .blood_gains
                    .iter()
                    .filter(|&&(g, _, _)| g == m)
                    .map(|&(_, n, _)| n)
                    .collect();
                amounts.sort_unstable();
                amounts.dedup();
                amounts.into_iter().map(Choice::SetX).collect()
            }
            // Taunt the Caged Beast [ANI]: the predator's ready vampire, the
            // prey's the target.
            Some(Terms::TauntPair) => {
                let of = self.state.predator(ctx.seat);
                self.state
                    .minions_of(of)
                    .filter(|&m| {
                        let v = &self.state.minions[m.index()];
                        v.is_vampire() && v.ready()
                    })
                    .map(Choice::ChooseMinion)
                    .collect()
            }
            // Blood Brothers of the actor's circle, one at a time; Pass
            // stops (X may be 0: a legal no-op, rules-qa 17.4).
            Some(Terms::CirclePicks(kind)) => {
                let mut options = vec![Choice::Pass];
                if ctx.picks.iter().any(Option::is_none) {
                    options.extend(
                        self.circle_candidates(ctx, kind)
                            .into_iter()
                            .map(Choice::ChooseMinion),
                    );
                }
                options
            }
            // Sudario Refraction: a library card of the ash heap with a copy
            // not named yet; none left, the naming ends.
            Some(Terms::AshCards(_)) => {
                let ash = &self.state.seats[ctx.seat.index()].ash;
                let named = |c: u16| ctx.ash_names.iter().filter(|&&n| n == Some(c)).count();
                let options: Vec<Choice> = crate::state::distinct(ash, |c| {
                    ash.iter().filter(|&&a| a == c).count() > named(c)
                })
                .into_iter()
                .map(Choice::FindCard)
                .collect();
                if options.is_empty() {
                    vec![Choice::Pass]
                } else {
                    options
                }
            }
            // "You can burn a card named X (if any)": any copy in play,
            // whoever controls it, or none; nothing in play elides it.
            Some(Terms::BurnNamedInPlay(name)) => {
                let mut options = vec![Choice::Pass];
                for (i, c) in self.state.cards.iter().enumerate() {
                    if c.present() && LIBRARY[c.card as usize].name == name {
                        options.push(Choice::BurnInPlay { card: i as u32 });
                    }
                }
                options
            }
            Some(Terms::CopiesControlled)
            | Some(Terms::Discard { .. })
            | Some(Terms::DiscardAny { .. })
            | Some(Terms::Count { .. })
            | Some(Terms::LifeFromTarget)
            | Some(Terms::AshCard)
            | Some(Terms::PreyAshPair)
            | Some(Terms::VampireToHoldTarget)
            | Some(Terms::XPool)
            | Some(Terms::XCounters)
            | Some(Terms::DirectedHandCard)
            | None => {
                unreachable!("terms window without a choice")
            }
        };
        DecisionRequest::new(ctx.seat, WindowLabel::PlayTerms, options)
    }

    /// The Blood Brothers a `Terms::CirclePicks` still offers: ready, its
    /// player's, of the actor's circle, not picked yet.
    fn circle_candidates(&self, ctx: &PlayCtx, kind: cards::CirclePick) -> Vec<MinionId> {
        let st = &self.state;
        let Some(actor) = ctx.actor else {
            return Vec::new();
        };
        st.minions_of(ctx.seat)
            .filter(|&m| {
                let v = &st.minions[m.index()];
                let circle = if m == actor {
                    matches!(
                        kind,
                        cards::CirclePick::SelfIncluded | cards::CirclePick::WithBloodSelfIncluded
                    ) && v.clan() == Some("Blood Brother")
                        && v.circle().is_some()
                } else {
                    st.same_circle(actor, m)
                };
                circle
                    && v.ready()
                    && !ctx.picks.contains(&Some(m))
                    && match kind {
                        cards::CirclePick::OthersUnlocked => !v.locked,
                        cards::CirclePick::WithBloodSelfIncluded => v.blood > 0,
                        cards::CirclePick::Others | cards::CirclePick::SelfIncluded => true,
                    }
            })
            .collect()
    }

    pub(crate) fn terms_apply(&mut self, mut ctx: PlayCtx, resume: Flow, choice: &Choice) {
        let ash_cards =
            cards::lookup(ctx.card).and_then(|d| match d.modes[ctx.mode as usize].terms {
                Some(Terms::AshCards(n)) => Some(n),
                _ => None,
            });
        let terms = cards::lookup(ctx.card).and_then(|d| d.modes[ctx.mode as usize].terms);
        let circle_picks = matches!(terms, Some(Terms::CirclePicks(_) | Terms::TauntPair));
        if circle_picks {
            if let Choice::ChooseMinion(m) = choice {
                let free = ctx
                    .picks
                    .iter()
                    .position(Option::is_none)
                    .expect("a free pick");
                ctx.picks[free] = Some(*m);
                // Taunt the Caged Beast: its one pick made, on to the play.
                let pair_done = terms == Some(Terms::TauntPair) && free == 0;
                if !pair_done {
                    self.flow = Flow::PlayTerms {
                        ctx,
                        resume: Box::new(resume),
                    };
                    return;
                }
            }
            if let Flow::AsPlayed { .. } = resume {
                self.flow = resume;
                self.open_as_played(ctx, Flow::GameOver);
            } else {
                self.open_as_played(ctx, resume);
            }
            return;
        }
        if let Some(n) = ash_cards {
            // One name more, until n are named or none is left.
            let named = ctx.ash_names.iter().filter(|c| c.is_some()).count();
            if let Choice::FindCard(card) = choice {
                ctx.ash_names[named] = Some(*card);
                if named + 1 < n as usize {
                    self.flow = Flow::PlayTerms {
                        ctx,
                        resume: Box::new(resume),
                    };
                    return;
                }
            }
            if let Flow::AsPlayed { .. } = resume {
                self.flow = resume;
                self.open_as_played(ctx, Flow::GameOver);
            } else {
                self.open_as_played(ctx, resume);
            }
            return;
        }
        match choice {
            Choice::SetX(n) => ctx.x = *n,
            Choice::BurnInPlay { card } => ctx.burn = Some(CardId(*card)),
            Choice::EmployFromHand { card, mode } => {
                ctx.named = Some((*card, *mode));
                // A blood share to choose next (Contraband [OBF]).
                let share = cards::lookup(ctx.card).is_some_and(|d| {
                    matches!(
                        d.modes[ctx.mode as usize].terms,
                        Some(Terms::EquipFromHand {
                            blood_share: true,
                            ..
                        })
                    )
                });
                if share && self.blood_share_max(ctx.seat, ctx.actor.expect("actor"), *card) > 0 {
                    self.flow = Flow::PlayTerms {
                        ctx,
                        resume: Box::new(resume),
                    };
                    return;
                }
            }
            Choice::ChooseMinion(m) => ctx.chosen = Some(*m),
            Choice::ChooseDiscipline(d) => ctx.discipline = Some(*d),
            Choice::ChooseSeat(s) => {
                ctx.ash = Some((*s, u16::MAX));
                self.flow = Flow::PlayTerms {
                    ctx,
                    resume: Box::new(resume),
                };
                return;
            }
            Choice::FindCard(card) => {
                let (s, _) = ctx.ash.expect("the ash heap chosen");
                ctx.ash = Some((s, *card));
            }
            // Redeem the Lost Soul: the burned vampire, by crypt card.
            Choice::AshVampire(card) => ctx.ash = Some((ctx.seat, *card)),
            Choice::Pass => ctx.burn = None,
            other => unreachable!("illegal terms choice {other:?}"),
        }
        // The card just named may take reductions (Piper's ally,
        // Contraband's equipment; ANK 20230824, RTR 20070707).
        if ctx.named.is_some() && self.cost_choices(&ctx) {
            self.flow = Flow::CostTerms {
                ctx,
                resume: Box::new(resume),
                pending: None,
                then: CostThen::AsPlayed,
            };
            return;
        }
        // A nested play resumes the outer as-played stack it interrupted;
        // restore it so the new window stacks on top.
        if let Flow::AsPlayed { .. } = resume {
            self.flow = resume;
            self.open_as_played(ctx, Flow::GameOver);
        } else {
            self.open_as_played(ctx, resume);
        }
    }

    /// Open the as-played window of `ctx`: an ordered window over the other
    /// living Methuselahs, clockwise from the player. Nested inside another
    /// as-played window (a cancel being canceled), it stacks.
    fn open_as_played(&mut self, ctx: PlayCtx, resume: Flow) {
        let order: Vec<SeatId> = self
            .state
            .clockwise_from(ctx.seat)
            .into_iter()
            .filter(|&s| s != ctx.seat)
            .collect();
        let pending = PendingPlay {
            ctx,
            window: OrderedWindow::new(order),
            canceled: false,
            cost_paid: false,
        };
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match flow {
            Flow::AsPlayed { mut stack, resume } => {
                let mut pending = pending;
                // Dark Influences: a cancel of another Methuselah's minion
                // card is canceled unpaid, the shield burned instead.
                if let Some(shield) = self.cancel_shield(&stack, &pending.ctx) {
                    pending.canceled = true;
                    pending.cost_paid = false;
                    self.burn_card_in_play(shield);
                }
                stack.push(pending);
                self.flow = Flow::AsPlayed { stack, resume };
            }
            _ => {
                self.flow = Flow::AsPlayed {
                    stack: vec![pending],
                    resume: Box::new(resume),
                }
            }
        }
    }

    /// The Dark Influences in play a cancel `ctx` of the stack's top card
    /// runs into: the canceled card a minion card of another Methuselah
    /// than the canceling one.
    fn cancel_shield(&self, stack: &[PendingPlay], ctx: &PlayCtx) -> Option<CardId> {
        let below = stack.last()?;
        let cancels = cards::lookup(ctx.card).is_some_and(|d| {
            d.modes[ctx.mode as usize]
                .effects
                .iter()
                .any(|e| matches!(e, Effect::CancelPlayed { .. }))
        });
        let t = LIBRARY[below.ctx.card as usize].types;
        let minion_card = !t.contains(&CardType::Master) && !t.contains(&CardType::Event);
        if !cancels || !minion_card || below.ctx.seat == ctx.seat {
            return None;
        }
        (0..self.state.cards.len())
            .map(|i| CardId(i as u32))
            .find(|&c| {
                self.state.cards[c.index()].present()
                    && cards::lookup(self.state.cards[c.index()].card)
                        .is_some_and(|d| d.modes[0].effects.contains(&Effect::CancelShield))
            })
    }

    /// The as-played window admits cancels, and wakes during an action
    /// (rules-qa 2.4; the Wake mode lists CardAsPlayed among its windows).
    pub(crate) fn as_played_request(&self, stack: &[PendingPlay]) -> Option<DecisionRequest> {
        let top = stack.last().expect("as-played stack");
        let seat = top.window.current()?;
        let mut options = vec![Choice::Pass];
        options.extend(self.play_options(seat, &[Window::CardAsPlayed], Some(&top.ctx)));
        // "The blocking minion's controller can burn 1 pool to cancel this
        // card as it is played" (True Love's Face [OBF][PRE]). Burning a
        // last pool this way is not offered (an oust mid-play: DESIGN).
        if let Some(n) = cards::lookup(top.ctx.card).and_then(|d| {
            d.modes[top.ctx.mode as usize]
                .effects
                .iter()
                .find_map(|e| match e {
                    Effect::CancelableForPool(n) => Some(*n),
                    _ => None,
                })
        }) {
            let blocker_seat = self.action.as_ref().and_then(|a| match a.state {
                crate::actions::ActionState::Attempt(m) => {
                    Some(self.state.minions[m.index()].owner)
                }
                _ => None,
            });
            if blocker_seat == Some(seat) && self.state.seats[seat.index()].pool > n as i32 {
                options.push(Choice::PayToCancel);
            }
        }
        // "The blocking minion can burn 1 blood or life to cancel this card
        // as it is played" (Crocodile's Tongue): while it has them.
        if let Some(n) = cards::lookup(top.ctx.card).and_then(|d| {
            d.modes[top.ctx.mode as usize]
                .effects
                .iter()
                .find_map(|e| match e {
                    Effect::CancelableForBlockerBlood(n) => Some(*n),
                    _ => None,
                })
        }) {
            if let Some(m) = self.action.as_ref().and_then(|a| match a.state {
                crate::actions::ActionState::Attempt(m) => Some(m),
                _ => None,
            }) {
                if self.state.minions[m.index()].owner == seat
                    && self.state.minions[m.index()].blood >= n
                {
                    options.push(Choice::PayToCancel);
                }
            }
        }
        // "Their controller can burn 2 pool to cancel this card as it is
        // played" (Golconda: Inner Peace): the target vampire's controller.
        if let (Some(n), Target::Minion(t)) = (
            cards::lookup(top.ctx.card).and_then(|d| {
                d.modes[top.ctx.mode as usize]
                    .effects
                    .iter()
                    .find_map(|e| match e {
                        Effect::CancelableForPoolByTargetController(n) => Some(*n),
                        _ => None,
                    })
            }),
            top.ctx.target,
        ) {
            if self.state.minions[t.index()].owner == seat
                && self.state.seats[seat.index()].pool > n as i32
            {
                options.push(Choice::PayToCancel);
            }
        }
        // "They can discard two combat cards to cancel this card as it is
        // played" (Target Vitals): the opposing minion's controller.
        if let Some(n) = cards::lookup(top.ctx.card).and_then(|d| {
            d.modes[top.ctx.mode as usize]
                .effects
                .iter()
                .find_map(|e| match e {
                    Effect::CancelableByDiscardingCombat(n) => Some(*n),
                    _ => None,
                })
        }) {
            let opposing_seat = self.combat.as_ref().and_then(|c| {
                let i = c.side_of_minion(top.ctx.actor?)?;
                Some(c.sides[1 - i].seat)
            });
            let combat_cards = self.state.seats[seat.index()]
                .hand
                .iter()
                .filter(|&&c| LIBRARY[c as usize].types.contains(&CardType::Combat))
                .count();
            if opposing_seat == Some(seat) && combat_cards >= n as usize {
                options.push(Choice::PayToCancel);
            }
        }
        // Abilities cancelling a card as it is played (The Vozhd of
        // Gravesend: a strike card of the minion opposing it).
        options.extend(
            self.ability_options(seat, Window::CardAsPlayed)
                .into_iter()
                .filter(|o| match *o {
                    Choice::UseAbility {
                        source, ability, ..
                    } => {
                        let mode = source.mode(&self.state, ability);
                        (!mode.requires.contains(&Requirement::PlayedStrikeByOpponent)
                            || self.played_strike_by_opponent(source, &top.ctx))
                            && mode.requires.iter().all(|r| match *r {
                                Requirement::PlayedFrenzyOnYour(clan) => {
                                    self.played_frenzy_on(seat, clan, &top.ctx)
                                }
                                Requirement::PlayedGrappleOrAimByOpponent => {
                                    self.played_grapple_by_opponent(source, &top.ctx)
                                }
                                Requirement::PlayedActionCardAtYour(label) => {
                                    cards::played_action_card_at(&self.state, seat, label, &top.ctx)
                                }
                                _ => true,
                            })
                    }
                    _ => true,
                }),
        );
        Some(DecisionRequest::new(
            seat,
            WindowLabel::CardAsPlayed,
            options,
        ))
    }

    /// Week of Nightmares with no counters left: "each Ravnos burns 1 blood
    /// for each nightmare counter on them or is burned, then burn this card
    /// and the nightmare counters" — paying if it can.
    pub(crate) fn nightmares_end(&mut self, card: CardId, clan: &str) {
        for i in 0..self.state.minions.len() {
            let m = MinionId(i as u32);
            let v = &self.state.minions[i];
            let n = v.nightmare;
            if n == 0 || !v.present() || v.clan() != Some(clan) {
                continue;
            }
            if v.blood >= n {
                self.state.minions[i].blood -= n;
            } else {
                self.burn_minion(m);
            }
        }
        for v in &mut self.state.minions {
            v.nightmare = 0;
        }
        self.burn_card_in_play(card);
    }

    /// Is the card being played a frenzy card (a frenzy mode, Deep Song
    /// [ANI], or the Frenzy label) on a vampire of `clan` that `seat`
    /// controls, its target or its player (Meditative Grove)?
    fn played_frenzy_on(&self, seat: SeatId, clan: &str, ctx: &PlayCtx) -> bool {
        self.frenzy_on(ctx.card, ctx.mode, ctx.actor, ctx.target)
            .is_some_and(|m| {
                let v = &self.state.minions[m.index()];
                v.owner == seat && v.clan() == Some(clan)
            })
    }

    /// The minion a frenzy card is played on: the one it targets, chooses
    /// or affects (LSJ 20051113) — the opposing minion for a combat card
    /// affecting it (Terror Frenzy; ANK 20220409), else its player's own.
    /// None for a card that is no frenzy card.
    pub(crate) fn frenzy_on(
        &self,
        card: u16,
        mode: u8,
        actor: Option<MinionId>,
        target: Target,
    ) -> Option<MinionId> {
        let effects = cards::lookup(card).map_or(&[][..], |d| d.modes[mode as usize].effects);
        let frenzy = LIBRARY[card as usize].labels.contains(&Label::Frenzy)
            || effects.contains(&Effect::Frenzy);
        if !frenzy {
            return None;
        }
        if let Target::Minion(m) = target {
            return Some(m);
        }
        let on_opponent = effects.iter().any(|e| {
            matches!(
                e,
                Effect::OpposingRestraint | Effect::OpposingCombatCardsCost(_)
            )
        });
        match (on_opponent, self.combat.as_ref(), actor) {
            (true, Some(c), Some(m)) => c.opponent_of(m),
            _ => actor,
        }
    }

    pub(crate) fn as_played_apply(
        &mut self,
        mut stack: Vec<PendingPlay>,
        resume: Box<Flow>,
        choice: &Choice,
    ) {
        match choice {
            Choice::Pass => {
                stack.last_mut().expect("as-played stack").window.pass();
                self.flow = Flow::AsPlayed { stack, resume };
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                let seat = stack
                    .last()
                    .and_then(|p| p.window.current())
                    .expect("as-played impulse holder");
                // The play voids prior passes on the window it interrupts.
                stack.last_mut().expect("as-played stack").window.play();
                self.flow = Flow::AsPlayed { stack, resume };
                self.announce_play(seat, *card, *mode, *actor, *target, Flow::GameOver);
            }
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                let top = stack.last_mut().expect("as-played stack");
                let seat = top.window.current().expect("as-played impulse holder");
                top.window.play();
                // "Cancel a card as it is played": the play below is
                // canceled, settled as the window closes.
                if let Some(refund) =
                    source
                        .mode(&self.state, *ability)
                        .effects
                        .iter()
                        .find_map(|e| match e {
                            Effect::CancelPlayed { refund } => Some(*refund),
                            _ => None,
                        })
                {
                    top.canceled = true;
                    top.cost_paid = !refund;
                }
                // "That Methuselah gains another master phase action"
                // (Bleeding the Vine), as Wash's.
                if source
                    .mode(&self.state, *ability)
                    .effects
                    .contains(&Effect::CanceledGetsMasterAction)
                {
                    let t = top.ctx.seat;
                    let out_of_turn = cards::lookup(top.ctx.card).is_some_and(|d| d.out_of_turn);
                    let seat = &mut self.state.seats[t.index()];
                    if out_of_turn {
                        seat.mpa_next += 1;
                    } else {
                        seat.mpa += 1;
                    }
                }
                let flow = Flow::AsPlayed { stack, resume };
                if !self.use_ability(seat, *source, *ability, *actor, *target, flow.clone()) {
                    self.flow = flow;
                }
            }
            Choice::PayToCancel
                if cards::lookup(stack.last().expect("as-played stack").ctx.card).is_some_and(
                    |d| {
                        let ctx = &stack.last().expect("as-played stack").ctx;
                        d.modes[ctx.mode as usize]
                            .effects
                            .iter()
                            .any(|e| matches!(e, Effect::CancelableByDiscardingCombat(_)))
                    },
                ) =>
            {
                // Target Vitals: two combat cards of their choice discarded.
                let top = stack.last_mut().expect("as-played stack");
                let n = cards::lookup(top.ctx.card)
                    .and_then(|d| {
                        d.modes[top.ctx.mode as usize]
                            .effects
                            .iter()
                            .find_map(|e| match e {
                                Effect::CancelableByDiscardingCombat(n) => Some(*n),
                                _ => None,
                            })
                    })
                    .expect("a cancel by discarding");
                let seat = top.window.current().expect("as-played impulse holder");
                top.window.play();
                top.canceled = true;
                self.interrupts.push(Interrupt::DiscardChosen {
                    seat,
                    left: n,
                    done: 0,
                    combat_only: true,
                });
                self.flow = Flow::AsPlayed { stack, resume };
            }
            Choice::PayToCancel => {
                let top = stack.last_mut().expect("as-played stack");
                // Crocodile's Tongue: the blocking minion's blood or life.
                let blocker = cards::lookup(top.ctx.card).and_then(|d| {
                    d.modes[top.ctx.mode as usize]
                        .effects
                        .iter()
                        .find_map(|e| match e {
                            Effect::CancelableForBlockerBlood(n) => Some(*n),
                            _ => None,
                        })
                });
                if let Some(n) = blocker {
                    top.window.play();
                    top.canceled = true;
                    let m = match self.action.as_ref().map(|a| a.state) {
                        Some(crate::actions::ActionState::Attempt(m)) => m,
                        _ => unreachable!("a blocking minion"),
                    };
                    self.state.burn_blood(m, n);
                    let v = &self.state.minions[m.index()];
                    if v.is_ally() && v.blood == 0 {
                        self.burn_minion(m);
                    }
                    self.flow = Flow::AsPlayed { stack, resume };
                    return;
                }
                let n = cards::lookup(top.ctx.card)
                    .and_then(|d| {
                        d.modes[top.ctx.mode as usize]
                            .effects
                            .iter()
                            .find_map(|e| match e {
                                Effect::CancelableForPool(n)
                                | Effect::CancelableForPoolByTargetController(n) => Some(*n),
                                _ => None,
                            })
                    })
                    .expect("a cancel for pool");
                let seat = top.window.current().expect("as-played impulse holder");
                top.window.play();
                top.canceled = true;
                self.state.seats[seat.index()].pool -= n as i32;
                self.flow = Flow::AsPlayed { stack, resume };
            }
            other => unreachable!("illegal as-played choice {other:?}"),
        }
    }

    /// Is the card being played a grapple or aim card of the minion opposing
    /// `source`'s bearer in combat (Sword of the Archangel)?
    fn played_grapple_by_opponent(&self, source: AbilitySource, ctx: &PlayCtx) -> bool {
        let AbilitySource::Card(c) = source else {
            return false;
        };
        let labels = LIBRARY[ctx.card as usize].labels;
        let kind = labels.contains(&Label::Grapple) || labels.contains(&Label::Aim);
        let host = self.state.cards[c.index()].bearer();
        kind && self
            .combat
            .as_ref()
            .zip(host)
            .is_some_and(|(cb, h)| cb.opponent_of(h).is_some_and(|o| ctx.actor == Some(o)))
    }

    /// The card played (`ctx`) is a strike card of the minion opposing
    /// `source`'s host in its combat (The Vozhd of Gravesend).
    fn played_strike_by_opponent(&self, source: AbilitySource, ctx: &PlayCtx) -> bool {
        let host = match source {
            AbilitySource::Card(c) => self.state.cards[c.index()].bearer(),
            AbilitySource::Vampire(m) => Some(m),
        };
        host.is_some_and(|h| self.strike_card_against(h, ctx))
    }

    /// `ctx` plays an aim, frenzy or grapple card for the minion opposing
    /// `m` in combat (Bollix [pre]).
    pub(crate) fn aim_frenzy_grapple_against(&self, m: MinionId, ctx: &PlayCtx) -> bool {
        let Some(cb) = self.combat.as_ref() else {
            return false;
        };
        let Some(i) = cb.side_of_minion(m) else {
            return false;
        };
        let labels = LIBRARY[ctx.card as usize].labels;
        let frenzy = cards::lookup(ctx.card).is_some_and(|d| {
            d.modes
                .get(ctx.mode as usize)
                .is_some_and(|md| md.effects.contains(&Effect::Frenzy))
        });
        ctx.actor == Some(cb.sides[1 - i].minion)
            && (frenzy
                || [Label::Aim, Label::Frenzy, Label::Grapple]
                    .iter()
                    .any(|l| labels.contains(l)))
    }

    /// `ctx` plays, for `m`'s combat opponent, a card restricting `m`'s
    /// strikes (Groundfighting): hand strikes only (Immortal Grapple), or
    /// no equipment while `m` is armed (Drawing Out the Beast); not range,
    /// dodges, strike counts, equipment or Disciplines (LSJ 20050221).
    pub(crate) fn restricts_strikes_of(&self, m: MinionId, ctx: &PlayCtx) -> bool {
        let Some(cb) = self.combat.as_ref() else {
            return false;
        };
        let Some(i) = cb.side_of_minion(m) else {
            return false;
        };
        let armed = self
            .state
            .holds_a(m, cards::Holding::Label(crate::library::Label::Weapon));
        ctx.actor == Some(cb.sides[1 - i].minion)
            && cards::lookup(ctx.card).is_some_and(|d| {
                d.modes.get(ctx.mode as usize).is_some_and(|md| {
                    md.effects.iter().any(|e| match e {
                        Effect::OnlyHandStrikes => true,
                        Effect::DrawOutBeast { .. } => armed,
                        _ => false,
                    })
                })
            })
    }

    /// The card played (`ctx`) is a grapple card of the minion opposing `m`
    /// in its combat.
    pub(crate) fn grapple_card_against(&self, m: MinionId, ctx: &PlayCtx) -> bool {
        let Some(cb) = self.combat.as_ref() else {
            return false;
        };
        let Some(i) = cb.side_of_minion(m) else {
            return false;
        };
        LIBRARY[ctx.card as usize].labels.contains(&Label::Grapple)
            && ctx.actor == Some(cb.sides[1 - i].minion)
    }

    /// The card played (`ctx`) is a strike card (at strike declaration or
    /// the maneuver, Slam) of the minion in combat with `m` (Anticipation
    /// [AUS]).
    pub(crate) fn strike_card_against(&self, m: MinionId, ctx: &PlayCtx) -> bool {
        let Some(cb) = self.combat.as_ref() else {
            return false;
        };
        let Some(i) = cb.side_of_minion(m) else {
            return false;
        };
        is_strike_mode(ctx.card, ctx.mode) && ctx.actor == Some(cb.sides[1 - i].minion)
    }

    /// The top as-played window closed: resolve (or discard) that play, then
    /// continue beneath it. A play canceled by the one just resolved is
    /// settled immediately: nothing is "being played" any more.
    pub(crate) fn as_played_advance(&mut self, mut stack: Vec<PendingPlay>, resume: Box<Flow>) {
        let top = stack.pop().expect("as-played stack");
        debug_assert!(top.window.is_closed());
        let mut declared_action = false;
        if top.canceled {
            self.settle_canceled(top.ctx, top.cost_paid);
        } else {
            declared_action = self.resolve_play(top.ctx, &mut stack);
        }
        while stack.last().is_some_and(|p| p.canceled) {
            let p = stack.pop().expect("canceled play");
            self.settle_canceled(p.ctx, p.cost_paid);
        }
        // Resolution may have ousted the player (pool paid down to 0), in
        // which case the flow already moved on and must not be resumed; a
        // resolved action card has opened its action instead.
        if self.ousted_during_play {
            self.ousted_during_play = false;
            return;
        }
        if declared_action {
            debug_assert!(stack.is_empty());
            return;
        }
        if stack.is_empty() {
            self.flow = *resume;
        } else {
            self.flow = Flow::AsPlayed { stack, resume };
        }
    }

    /// A canceled card was still played (spec §1.8.3): to the ash heap and
    /// replaced (§1.9.3), not from a Fleshforge Chamber (rules-qa 37.1).
    /// Costs are paid at resolution (§1.7.1): only a "still paid" cancel
    /// (Meditative Grove) pays a non-action card's now, surcharges included
    /// (ANK 20260216).
    fn settle_canceled(&mut self, ctx: PlayCtx, cost_paid: bool) {
        let action_card = cards::lookup(ctx.card).is_some_and(|d| {
            d.modes[ctx.mode as usize]
                .effects
                .iter()
                .any(|e| matches!(e, Effect::DeclareAction(_)))
        });
        if cost_paid && !action_card {
            let mut pool = cards::pool_cost(&self.state, ctx.seat, ctx.card, ctx.target);
            // Less the playing minion's printed reductions, as at resolution
            // (Roger de Camden, Ragnar Nordstrom) — not a card naming
            // another's (Piper): those are the named card's.
            let names = cards::lookup(ctx.card).is_some_and(|d| {
                d.modes[ctx.mode as usize]
                    .effects
                    .iter()
                    .any(|e| matches!(e, Effect::EmployNamed | Effect::EquipNamed))
            });
            let mut blood_off = if names {
                0
            } else {
                self.printed_off_play(&ctx, &mut pool)
            };
            // Paid, so Nergal's reduction is spent (advanced rules §1.8.3).
            if !names {
                let blood =
                    cards::blood_cost(&self.state, ctx.card, ctx.x).saturating_sub(blood_off);
                let once = self.once_off_amount(&ctx, blood);
                blood_off += once;
                self.spend_once_cost_off(&ctx, once);
            }
            self.state.seats[ctx.seat.index()].pool -= pool.max(0);
            self.pay_play_blood(&ctx, blood_off);
        }
        // "Remove this card from the game, even if … canceled"
        // (Reinforcements).
        if self.played_from_removing_store(ctx.from) {
            let owner = self.played_card_owner(ctx.seat, ctx.from);
            self.state.seats[owner.index()]
                .removed
                .put(ctx.card, *ctx.copy);
        } else if cards::removed_once_played(ctx.card, ctx.mode) {
            self.state.seats[ctx.seat.index()]
                .removed
                .put(ctx.card, *ctx.copy);
        } else {
            self.state.seats[ctx.seat.index()]
                .ash
                .put(ctx.card, *ctx.copy);
        }
        if ctx.from.is_none() {
            self.replace(ctx.seat, 1, None);
            if self.learjet_play(&ctx) {
                self.offer_extra_draw(ctx.seat);
            }
        }
        // Victor Donaldson: his Methuselah's canceled card back to the
        // library, if they will — once it is replaced.
        let pc = crate::state::PrintedCtx {
            card: Some((ctx.card, ctx.mode)),
            ..Default::default()
        };
        if let Some(m) = self
            .state
            .minions_of(ctx.seat)
            .find(|&m| self.state.printed_mod(m, cards::Stat::RecyclesCanceled, pc) > 0)
        {
            self.interrupts.push(Interrupt::MayRecycleCanceled {
                seat: ctx.seat,
                minion: m,
                card: ctx.card,
            });
        }
    }

    /// The owner of a card played by `seat` from `from` (The Erciyes
    /// Fragments: the prey's card stays its owner's — its removal and its
    /// return to hand, rules-qa 60.19).
    pub(crate) fn played_card_owner(&self, seat: SeatId, from: Option<CardId>) -> SeatId {
        from.filter(|&f| f != crate::state::ASH_PLAY)
            .and_then(|f| self.state.cards[f.index()].stored_owner)
            .unwrap_or(seat)
    }

    /// Was a card played from a store whose cards are removed instead of
    /// burned (The Erciyes Fragments)?
    pub(crate) fn played_from_removing_store(&self, from: Option<CardId>) -> bool {
        // Spiritual Guidance: "if that card would be burned, instead remove
        // it from the game".
        if from == Some(crate::state::ASH_PLAY) {
            return true;
        }
        from.is_some_and(|f| {
            cards::lookup(self.state.cards[f.index()].card).is_some_and(|d| {
                d.modes[0]
                    .effects
                    .contains(&Effect::StoredRemovedWhenBurned)
            })
        })
    }

    /// Can Nergal take his once-each-turn blood off `ctx`'s card: his to
    /// play, costing blood, unspent this turn, not yet chosen?
    pub(crate) fn once_off_available(&self, ctx: &PlayCtx) -> bool {
        let st = &self.state;
        let Some(m) = ctx.actor else {
            return false;
        };
        let pctx = crate::state::PrintedCtx {
            card: Some((ctx.card, ctx.mode)),
            ..Default::default()
        };
        !ctx.once_off
            && cards::blood_cost(st, ctx.card, ctx.x) > 0
            && !st.turn_uses.contains(&crate::state::once_cost_off_key(m))
            && st.printed_mod(m, cards::Stat::BloodCostOffOncePerTurn, pctx) > 0
    }

    /// Nergal's blood off, if chosen for `ctx`'s card, off a cost of
    /// `blood`: spent only as the cost is paid (LSJ 20091021-2: not by a
    /// card canceled unpaid nor an action left unpayable).
    pub(crate) fn once_off_amount(&self, ctx: &PlayCtx, blood: u8) -> u8 {
        match (ctx.once_off, ctx.actor) {
            (true, Some(m))
                if blood > 0
                    && !self
                        .state
                        .turn_uses
                        .contains(&crate::state::once_cost_off_key(m)) =>
            {
                1
            }
            _ => 0,
        }
    }

    /// Nergal's blood off taken: spent for the turn.
    pub(crate) fn spend_once_cost_off(&mut self, ctx: &PlayCtx, taken: u8) {
        if let (true, Some(m)) = (taken > 0, ctx.actor) {
            self.state
                .turn_uses
                .push(crate::state::once_cost_off_key(m));
        }
    }

    /// The playing minion's printed reductions on `ctx`'s card: taken off
    /// `pool` — "blood or pool" off the pool while some is left —, the
    /// blood they take off returned.
    fn printed_off_play(&self, ctx: &PlayCtx, pool: &mut i32) -> u8 {
        let Some(m) = ctx.actor else {
            return 0;
        };
        let off = self.state.printed_cost_off_x(m, ctx.card, ctx.mode, ctx.x);
        *pool = (*pool - off.pool as i32).max(0);
        if *pool > 0 {
            *pool = (*pool - off.either as i32).max(0);
            off.blood
        } else {
            off.blood + off.either
        }
    }

    /// The blood (or, for an ally playing as a vampire, life) a card costs
    /// its player, surcharges included, less `blood_off`: true if that
    /// burned the ally paying it.
    fn pay_play_blood(&mut self, ctx: &PlayCtx, blood_off: u8) -> bool {
        let Some(m) = ctx.actor else {
            return false;
        };
        let surcharge = self.reaction_surcharge(ctx.card);
        if surcharge > 0 {
            // "The next reaction card": this one takes it.
            self.action.as_mut().expect("action").reaction_surcharge = 0;
        }
        let blood = (cards::blood_cost(&self.state, ctx.card, ctx.x)
            + surcharge
            + self.discipline_surcharge(m, ctx.card, ctx.mode)
            + self.combat_card_surcharge(m, ctx.card, ctx.mode))
        .saturating_sub(blood_off);
        let v = &mut self.state.minions[m.index()];
        v.blood = v.blood.saturating_sub(blood);
        if blood > 0 && v.is_ally() && v.blood == 0 {
            let combatant = self
                .combat
                .as_ref()
                .is_some_and(|c| c.side_of_minion(m).is_some());
            self.burn_minion(m);
            // A combatant gone, the combat ends: no strike resolves (ANK
            // 20180928-1).
            if combatant && self.combat.is_some() {
                self.end_combat_now();
            }
            return true;
        }
        false
    }

    /// Resolution: pay the cost, apply the effects, move the card, replace.
    /// Returns true when the play declared an action (the flow is now the
    /// action's).
    fn resolve_play(&mut self, ctx: PlayCtx, stack: &mut [PendingPlay]) -> bool {
        let s = ctx.seat;
        let own_turn = s == self.state.active;
        let def = cards::lookup(ctx.card).expect("definition");
        let mode = &def.modes[ctx.mode as usize];
        let is_reaction = LIBRARY[ctx.card as usize]
            .types
            .contains(&CardType::Reaction);
        // An action card: the action is declared now (cost at the action's
        // resolution) and the card is replaced before any modifier (spec
        // §1.9.1).
        if let Some(Effect::DeclareAction(kind)) = mode
            .effects
            .iter()
            .find(|e| matches!(e, Effect::DeclareAction(_)))
        {
            let actor = ctx.actor.expect("acting minion");
            // Not from the hand (Fleshforge Chamber): nothing to replace.
            // Blood Weakens holds a Discipline action card until its action
            // ends (LSJ 20080809; rules-qa 40.4); Bum's Rush's "after this
            // action" too, after every combat and after-resolution effect
            // (LSJ 20001120, LSJ 20101021).
            if ctx.from.is_none() {
                if def.delayed_replacement == Some(Until::EndOfAction) {
                    self.state.seats[s.index()].held[Release::EndOfAction] += 1;
                } else if self.weakened_card(&ctx) {
                    self.hold_weakened(s, None);
                } else {
                    self.replace(s, 1, None);
                }
            }
            self.declare_action(actor, *kind, None, Some(ctx), None);
            return true;
        }
        let mut pool = cards::pool_cost(&self.state, s, ctx.card, ctx.target);
        // Choosing is targeting (Secure Haven; LSJ 20031010): a master card
        // choosing a vampire as it is played, or played for one (Piper),
        // costs 1 more for its haven.
        if LIBRARY[ctx.card as usize].types.contains(&CardType::Master) {
            let for_minion = matches!(mode.actor, Actor::MasterForMinion)
                .then_some(ctx.actor)
                .flatten();
            // Archon Investigation targets the bleeding vampire (LSJ
            // 20010817).
            let acting = mode
                .effects
                .contains(&Effect::BurnActingMinion)
                .then(|| self.action.as_ref().map(|a| a.acting))
                .flatten();
            for m in ctx.chosen.into_iter().chain(for_minion).chain(acting) {
                if Target::Minion(m) != ctx.target {
                    pool += self.state.havens_on(m) as i32;
                }
            }
        }
        // The Shard, announced with a card that is not an action card:
        // locked now (not if the card was canceled, ANK 20230620).
        let mut blood_off = 0u8;
        // A card playing another from hand (Piper, Contraband): its
        // reductions are the named card's, taken as that card is paid.
        let names = mode
            .effects
            .iter()
            .any(|e| matches!(e, Effect::EmployNamed | Effect::EquipNamed));
        // The playing minion's printed reductions (Roger de Camden).
        if !names {
            blood_off += self.printed_off_play(&ctx, &mut pool);
            let blood = cards::blood_cost(&self.state, ctx.card, ctx.x).saturating_sub(blood_off);
            let once = self.once_off_amount(&ctx, blood);
            blood_off += once;
            self.spend_once_cost_off(&ctx, once);
        }
        let mut focus_burned = Vec::new();
        for (c, n) in ctx.discounts.into_iter().flatten().filter(|_| !names) {
            let card = &self.state.cards[c.index()];
            let (present, locked) = (card.present(), card.locked);
            let source = self.state.cost_source_of(c);
            if present
                && !locked
                && (source == Some(cards::CostSource::Shard)
                    && self.state.seats[s.index()].has_edge
                    || source == Some(cards::CostSource::SunsetStrip))
            {
                self.state.cards[c.index()].locked = true;
                if pool > 0 {
                    pool = (pool - n as i32).max(0);
                } else {
                    blood_off += n;
                }
            }
            // Focus the Blood: burned as the combat card's cost is paid.
            if present && self.state.cost_source_of(c) == Some(cards::CostSource::FocusBlood) {
                blood_off += n;
                focus_burned.push(c);
            }
        }
        for c in focus_burned {
            self.burn_card_in_play(c);
        }
        // A vampire's blood paying 1 of it (Regina Blake).
        if let Some(r) = ctx.blood_payer.filter(|_| !names) {
            let before = cards::blood_cost(&self.state, ctx.card, ctx.x).saturating_sub(blood_off);
            let mut blood = before;
            if let Some(r) = self.blood_payment(Some(r), &mut pool, &mut blood) {
                self.state.minions[r.index()].blood -= 1;
                blood_off += before - blood;
            }
        }
        // Jack: locked for 1 pool off, if still unlocked.
        if let Some(j) = ctx.locker.filter(|_| !names) {
            let v = &self.state.minions[j.index()];
            if v.in_play && !v.locked {
                self.state.minions[j.index()].locked = true;
                pool = (pool - 1).max(0);
            }
        }
        self.state.seats[s.index()].pool -= pool;
        if pool > 0 {
            self.note_pool_burned(s);
        }
        // An ally playing "as a vampire" pays in life: its last life
        // burns it, and the card has no effect — an action modifier so
        // played fails the action, a strike ends the combat (ANK
        // 20180928-1).
        let payer_burned = self.pay_play_blood(&ctx, blood_off);
        let mut in_play: Option<CardId> = None;
        let mut search = None;
        // A card whose delayed effect is represented in play takes its
        // layers then (the card itself goes to the ash heap now); Disarm,
        // its opposing vampire burned, none.
        let held = mode.effects.iter().any(|e| {
            matches!(
                e,
                Effect::PutOnOpponentOnHit { .. } | Effect::PutOnActorAfterCombat
            )
        });
        let on_opponent = mode.effects.contains(&Effect::PutOnOpponent);
        for effect in mode.effects.iter().filter(|_| !payer_burned) {
            match *effect {
                Effect::Layer(_) if held || on_opponent && in_play.is_none() => {}
                // Once the card is replaced (rules-qa 12.4).
                Effect::SearchStore(n) => search = Some(n),
                Effect::CancelPlayed { refund } => {
                    // The play beneath this one on the stack is the card
                    // being played that this cancel targets.
                    if let Some(below) = stack.last_mut() {
                        below.canceled = true;
                        below.cost_paid = !refund;
                    }
                }
                Effect::BarCanceledThisTurn => {
                    if let Some(below) = stack.last() {
                        self.state.barred_this_turn.push(below.ctx.card);
                    }
                }
                Effect::CanceledGetsMasterAction => {
                    if let Some(below) = stack.last() {
                        let t = below.ctx.seat;
                        let out_of_turn =
                            cards::lookup(below.ctx.card).is_some_and(|d| d.out_of_turn);
                        let seat = &mut self.state.seats[t.index()];
                        if out_of_turn {
                            seat.mpa_next += 1;
                        } else {
                            seat.mpa += 1;
                        }
                    }
                }
                e => self.apply_card_effect(e, &ctx, &mut in_play),
            }
        }
        match in_play {
            // Its search puts its cards on it first: the search's end
            // enters the contest (Black Market Cache).
            Some(_) if search.is_some() => {}
            // Entered play: a unique copy contests once its own entry
            // effects have resolved (spec §1.13.2).
            Some(id) => {
                if self.played_from_removing_store(ctx.from) {
                    let owner = self.played_card_owner(s, ctx.from);
                    let c = &mut self.state.cards[id.index()];
                    c.removed_when_burned = true;
                    c.owner = owner;
                }
                if LIBRARY[ctx.card as usize].labels.contains(&Label::Gehenna) {
                    self.gehenna_played(id);
                }
                self.enter_library_contest(id)
            }
            None if self.played_from_removing_store(ctx.from) => {
                let owner = self.played_card_owner(s, ctx.from);
                self.state.seats[owner.index()]
                    .removed
                    .put(ctx.card, *ctx.copy)
            }
            None => self.state.seats[s.index()].ash.put(ctx.card, *ctx.copy),
        }
        if def.trifle && self.state.active == s {
            let seat = &mut self.state.seats[s.index()];
            if !seat.trifle_used {
                seat.mpa += 1;
                seat.trifle_used = true;
            }
        }
        let deferred = is_reaction
            && self
                .action
                .as_ref()
                .is_some_and(|a| a.defer_reaction_replacement);
        // Learjet: this play's replacement, now or when it comes due.
        let learjet = self.learjet_play(&ctx);
        let held_before = self.state.seats[s.index()].held;
        let in_play_before = self.state.seats[s.index()].held_in_play.len();
        // The card's own "do not replace until" clause outlasts an
        // end-of-action deferral (spec §1.9.2: the longest delay wins).
        match def.delayed_replacement {
            _ if ctx.from.is_some() => {}
            // The replacement waits on the condition, not on the card: a
            // burned card still waits (LSJ 20080805).
            Some(u) => match (u.release(), in_play) {
                (Some(at), _) => self.state.seats[s.index()].held[at] += 1,
                // "As long as this card is in play".
                (None, Some(id)) => self.state.seats[s.index()].held_in_play.push(id),
                (None, None) => self.replace(s, 1, None),
            },
            None if deferred => self.state.seats[s.index()].held[Release::EndOfAction] += 1,
            // Blood Weakens: a minion's card played using a Discipline
            // during an action waits for its end, or for the hand to empty.
            None if self.action.is_some() && self.weakened_card(&ctx) => {
                self.hold_weakened(s, in_play)
            }
            None => self.replace(s, 1, in_play),
        }
        if learjet && ctx.from.is_none() {
            let seat = &mut self.state.seats[s.index()];
            let after = seat.held;
            if after.total() > held_before.total() {
                seat.learjet_held.add_diff(held_before, after);
            } else if seat.held_in_play.len() == in_play_before {
                self.offer_extra_draw(s);
            }
        }
        // A forced attempt's library toll, once the card is replaced (ANK
        // 20260317).
        let tolls = self
            .action
            .as_mut()
            .map(|a| std::mem::take(&mut a.library_toll))
            .unwrap_or_default();
        for (seat, n) in tolls {
            self.burn_library_top(seat, n);
        }
        // Black Market Cache's search, chosen as it resolves after its
        // replacement is drawn (rules-qa 12.4): queued behind that draw's
        // choices, if any.
        if let (Some(left), Some(card)) = (search, in_play) {
            self.interrupts.push(Interrupt::StoreSearch {
                seat: s,
                card,
                left,
            });
        }
        // Its effects may have ousted the player already (Piper's ally,
        // paid down to 0 as it resolves): on their turn, the turn passed
        // with it.
        let ousted_by_effects = self.state.seats[s.index()].ousted;
        if ousted_by_effects && own_turn {
            self.ousted_during_play = true;
        } else if self.state.seats[s.index()].pool == 0 || ousted_by_effects {
            if own_turn {
                if self.oust_and_pass(s) {
                    self.ousted_during_play = true;
                }
            } else {
                // An out-of-turn play paid down to 0 (Archon Investigation
                // at 3 pool): ousted, the turn goes on without them.
                if !ousted_by_effects {
                    self.oust(s);
                }
                if self.state.living() <= 1 {
                    self.ousted_during_play = true;
                } else {
                    for p in stack.iter_mut() {
                        p.window.remove(s);
                    }
                    if let Some(a) = self.action.as_mut() {
                        a.order.retain(|&o| o != s);
                        a.window.remove(s);
                    }
                }
            }
        } else if self.state.seats[self.state.active.index()].ousted {
            // A rider fired by the play's effects (Archon Investigation's
            // burn under Tension in the Ranks) ousted the active Methuselah:
            // the turn passes with the action.
            if self.state.living() > 1 {
                self.next_turn();
            }
            self.ousted_during_play = true;
        }
        false
    }

    /// A library card enters play (the single entry point: contests are
    /// checked by the caller once the card's entry effects resolved).
    pub(crate) fn enter_play(&mut self, mut card: CardInPlay) -> CardId {
        card.entered = self.state.turn;
        card.entered_action = self.state.action_serial;
        self.state.cards.push(card);
        CardId(self.state.cards.len() as u32 - 1)
    }

    /// "Put this card on … / in play" as the card resolves: the card itself
    /// enters play (rules-qa 47.1; from the ash heap: `place_held`,
    /// rules-qa 21.1); with `one_per_vampire`, not a second of its name on
    /// its host (a legal no-op, rules-qa 17.4); a title card gives its
    /// title, a unique one's once its contest is over (rules-qa 19.1).
    pub(crate) fn place_now(
        &mut self,
        card: CardInPlay,
        layers: ModeLayers,
        one_per_vampire: bool,
    ) -> Option<CardId> {
        if one_per_vampire
            && card
                .host
                .is_some_and(|h| self.state.holds_its_name(h, card.card))
        {
            return None;
        }
        let (c, host) = (card.card, card.host);
        let id = self.enter_play(card);
        if layers == ModeLayers::Push {
            self.push_mode_layers(id);
        }
        if let (Some(t), Some(h)) = (cards::card_title(c), host) {
            let unique = LIBRARY[c as usize].labels.contains(&Label::Unique);
            if !unique || self.state.library_rivals(id).is_empty() {
                self.gain_title(h, t, Some(id));
            }
        }
        Some(id)
    }

    /// One effect of a card resolving (a play, or an action card on
    /// success): its own (`Family::Play`: entering play, blood moves,
    /// layers), the others passed to their family's interpreter.
    pub(crate) fn apply_card_effect(
        &mut self,
        effect: Effect,
        ctx: &PlayCtx,
        in_play: &mut Option<CardId>,
    ) {
        let s = ctx.seat;
        match effect {
            Effect::GainPerUnlockedCircleMember => {
                let m = ctx.actor.expect("actor");
                let x = (0..self.state.minions.len())
                    .map(|i| MinionId(i as u32))
                    .filter(|&o| {
                        let v = &self.state.minions[o.index()];
                        v.ready() && !v.locked && self.state.same_circle(m, o)
                    })
                    .count() as u8;
                self.state.gain_blood(m, x + 1);
            }
            Effect::BloodToUncontrolledCircle => {
                let circle = ctx.actor.and_then(|m| {
                    let v = &self.state.minions[m.index()];
                    (v.clan() == Some("Blood Brother"))
                        .then(|| v.circle())
                        .flatten()
                });
                if let Some(circle) = circle {
                    for i in 0..self.state.seats[s.index()].uncontrolled.len() {
                        let slot = self.state.seats[s.index()].uncontrolled[i].card;
                        if slot != crate::state::CREATED
                            && crate::crypt::CRYPT[slot as usize].clan == "Blood Brother"
                            && crate::crypt::CRYPT[slot as usize].circle == Some(circle)
                        {
                            self.state.add_uncontrolled_blood(s, i, 1);
                        }
                    }
                }
            }
            Effect::BurnActorBlood(n) => self.state.burn_blood(ctx.actor.expect("actor"), n),
            // Tribute to the Master: each ready vampire's, none needed
            // (ANK 20210717).
            Effect::OwnReadyVampiresBloodToPool => {
                let ms: Vec<MinionId> = self
                    .state
                    .minions_of(s)
                    .filter(|&m| {
                        let v = &self.state.minions[m.index()];
                        v.ready() && v.is_vampire() && v.blood > 0
                    })
                    .collect();
                for m in ms {
                    self.state.burn_blood(m, 1);
                    self.state.seats[s.index()].pool += 1;
                }
            }
            // Major Boon: in play now, remembering the bled Methuselah.
            Effect::TakeBleedForTarget => {
                if let (Some(a), Some(id)) = (self.action.as_mut(), *in_play) {
                    if let Some(t) = a.bleed_target {
                        a.bleed_payer = Some((s, Some(id)));
                        self.state.cards[id.index()].against = crate::ids::SeatSet::single(t);
                    }
                }
            }
            // Sudario Refraction: the named cards on top, drawn into a hand
            // not full (LSJ 20080612-2), then 3 discarded at random and
            // replaced.
            Effect::SudarioRefraction => {
                for card in ctx.ash_names.into_iter().flatten() {
                    let seat = &mut self.state.seats[s.index()];
                    if let Some(k) = seat.ash.iter().position(|&c| c == card) {
                        let (card, copy) = seat.ash.take(k);
                        seat.library.put(card, copy);
                    }
                }
                self.draw_up(s);
                let k = 3.min(self.state.seats[s.index()].hand.len());
                self.discard_at_random(s, k);
                self.replace(s, k, None);
            }
            Effect::CryptTopToUncontrolled => {
                let st = &mut self.state.seats[s.index()];
                if let Some((card, copy)) = st.crypt.take_top() {
                    st.uncontrolled.push(crate::state::UncontrolledSlot {
                        card,
                        copy: crate::copies::Untracked(copy),
                        counters: 0,
                        banished: None,
                    });
                }
            }
            Effect::CryptClanToUncontrolled(clan) => {
                self.interrupts
                    .push(Interrupt::CryptSearch { seat: s, clan });
            }
            // Trochomancy: from the ash heap of the Methuselah bled.
            Effect::RemoveBleedTargetAshCards(n) => {
                let st = &self.state;
                if let Some(t) = self.action.as_ref().and_then(|a| a.directed_seat(st)) {
                    self.interrupts.push(Interrupt::RemoveFromAsh {
                        seat: s,
                        of: t,
                        left: n,
                        pool: 0,
                    });
                }
            }
            // Legacy: unlocked, then the search (an action effect: the
            // action's own rule not barring an "unlock this vampire").
            Effect::MergeWithBase => {
                let m = ctx.actor.expect("actor");
                self.state.unlock(m);
                self.interrupts
                    .push(Interrupt::BaseSearch { seat: s, minion: m });
            }
            Effect::NextReferendumPasses => {
                self.state.seats[s.index()].auto_pass_next = true;
            }
            Effect::AutoPassFirstSabbatReferendum => {
                self.state.seats[s.index()].auto_pass_pending = true;
            }
            Effect::RetainerToStartingLife => {
                if let Target::Card(c) = ctx.target {
                    if self.state.cards[c.index()].present() {
                        let card = self.state.cards[c.index()].card;
                        let mode = self.state.cards[c.index()].mode;
                        let life = cards::lookup(card)
                            .and_then(|d| d.modes.get(mode as usize))
                            .and_then(|m| {
                                m.effects.iter().find_map(|e| match e {
                                    Effect::EmployRetainer { life } => Some(*life),
                                    _ => None,
                                })
                            })
                            .unwrap_or(0);
                        let owner = self.state.card_controller(c);
                        let life = if LIBRARY[card as usize]
                            .labels
                            .contains(&crate::library::Label::Zombie)
                        {
                            life + self.state.zombie_life_bonus(owner)
                        } else {
                            life
                        };
                        let k = &mut self.state.cards[c.index()].counters;
                        *k = (*k).max(life);
                    }
                }
            }
            // Liquidation: "burn 7 cards from the top of your library to
            // gain 3 pool".
            Effect::BurnLibraryTop(n) => self.burn_library_top(s, n),
            Effect::BloodToEachUncontrolledOfSect(sect, n) => {
                for i in 0..self.state.seats[s.index()].uncontrolled.len() {
                    if self.state.slot_sect(s, i) == sect {
                        self.state.add_uncontrolled_blood(s, i, n);
                    }
                }
            }
            Effect::FuryEnters => {
                // A token whose card has left play is taken again (no leak
                // over a long game).
                let spent = (0..self.state.minions.len()).find(|&i| {
                    matches!(self.state.minions[i].kind, crate::state::MinionKind::Fury { id, .. }
                        if !self.state.cards[id.index()].present())
                });
                if let (Some(id), Some(i)) = (*in_play, spent) {
                    self.state.minions[i] = crate::state::Minion {
                        in_play: false,
                        ..crate::state::Minion::new(
                            s,
                            crate::state::MinionKind::Fury { card: ctx.card, id },
                            0,
                        )
                    };
                } else if let Some(id) = *in_play {
                    self.state.minions.push(crate::state::Minion {
                        in_play: false,
                        ..crate::state::Minion::new(
                            s,
                            crate::state::MinionKind::Fury { card: ctx.card, id },
                            0,
                        )
                    });
                }
            }
            // Call the Great Beast: X counters, X the Baali's capacity — 11
            // or more, it rises at once (ANK 20180723).
            Effect::CountersFromTargetCapacity => {
                if let (Some(id), Target::Minion(t)) = (*in_play, ctx.target) {
                    self.state.cards[id.index()].counters = self.state.capacity_of(t).unwrap_or(0);
                    if self.state.cards[id.index()].counters == 0 {
                        self.burn_card_in_play(id);
                    }
                }
            }
            Effect::CountersFromActorCapacity => {
                if let (Some(id), Some(a)) = (*in_play, ctx.actor) {
                    self.state.cards[id.index()].counters = self.state.capacity_of(a).unwrap_or(0);
                }
            }
            Effect::GreatBeastAtEleven => {
                if let Some(id) = *in_play {
                    self.great_beast_at_eleven(id);
                }
            }
            // Condemnation: Mute: the other condemnation cards on its
            // minion.
            Effect::BurnOthersLabeledOnTarget(label) => {
                if let Target::Minion(t) = ctx.target {
                    let others: Vec<CardId> = (0..self.state.cards.len())
                        .map(|i| CardId(i as u32))
                        .filter(|&c| {
                            let card = &self.state.cards[c.index()];
                            Some(c) != *in_play
                                && card.present()
                                && card.host == Some(t)
                                && LIBRARY[card.card as usize].labels.contains(&label)
                        })
                        .collect();
                    for c in others {
                        self.burn_card_in_play(c);
                    }
                }
            }
            Effect::BurnRangedWeaponsOnTarget { except } => {
                if let Target::Minion(t) = ctx.target {
                    let ranged: Vec<CardId> = (0..self.state.cards.len())
                        .map(|i| CardId(i as u32))
                        .filter(|&c| {
                            let card = &self.state.cards[c.index()];
                            card.present()
                                && card.host == Some(t)
                                && cards::is_ranged_weapon(card.card)
                                && LIBRARY[card.card as usize].name != except
                        })
                        .collect();
                    for c in ranged {
                        self.burn_card_in_play(c);
                    }
                }
            }
            // Vampiric Disease: one counter at most.
            Effect::DiseaseTarget => {
                let t = ctx.target.minion();
                self.state.minions[t.index()].disease = true;
            }
            // Chosen as the card was played (rules-qa 28.5).
            Effect::ChooseSecondOfClan(_) => {
                if let Some(id) = *in_play {
                    let chosen = &mut self.state.cards[id.index()].chosen;
                    chosen.truncate(1);
                    chosen.extend(ctx.chosen);
                }
            }
            Effect::ChooseOwnOfClan(_) => {
                if let Some(id) = *in_play {
                    self.state.cards[id.index()].chosen = ctx.chosen.into_iter().collect();
                }
            }
            Effect::ChooseTarget => {
                if let (Some(id), Target::Minion(t)) = (*in_play, ctx.target) {
                    self.state.cards[id.index()].chosen = vec![t];
                }
            }
            Effect::RevealCryptTop {
                clan,
                blood,
                if_younger,
                shuffle,
            } => {
                // Younger than the acting vampire, read now (Mesu Bedshet).
                let than = ctx.actor.and_then(|m| self.state.capacity_of(m));
                let seat = &mut self.state.seats[s.index()];
                if let Some((card, copy)) = seat.crypt.take_top() {
                    let c = &crate::crypt::CRYPT[card as usize];
                    if c.clan == clan {
                        seat.uncontrolled.push(crate::state::UncontrolledSlot {
                            card,
                            copy: crate::copies::Untracked(copy),
                            counters: 0,
                            banished: None,
                        });
                        let i = seat.uncontrolled.len() - 1;
                        if !if_younger || than.is_some_and(|t| c.capacity < t) {
                            self.state.add_uncontrolled_blood(s, i, blood);
                        }
                    } else if shuffle {
                        seat.crypt.put(card, copy);
                        seat.crypt.shuffle(&mut self.rng);
                    } else {
                        seat.crypt.put_at(0, card, copy);
                    }
                }
            }
            // The card put in play is the vampire (its `body`), ready and
            // unlocked with no blood, as the ally's card is its ally.
            Effect::TrainInUncontrolled { counters, .. } => {
                *in_play = Some(self.enter_play(CardInPlay {
                    in_play: false,
                    training: true,
                    counters,
                    ..CardInPlay::entering(ctx.card, *ctx.copy, s, None, ctx.mode)
                }));
            }
            // Mozambique Allure: the crypt draw ends in the uncontrolled
            // region; the new sect replaces the old, its title none
            // (required untitled).
            Effect::LaibonOrCryptDraw(clans) => {
                let m = ctx.actor.expect("actor");
                if self.state.sect_of(m) == Some(crate::crypt::Sect::Laibon) {
                    let seat = &mut self.state.seats[s.index()];
                    if let Some((card, copy)) = seat.crypt.take_top() {
                        seat.uncontrolled.push(crate::state::UncontrolledSlot {
                            card,
                            copy: crate::copies::Untracked(copy),
                            counters: 0,
                            banished: None,
                        });
                    }
                } else {
                    self.state.minions[m.index()].sect_override = Some(crate::crypt::Sect::Laibon);
                    if self.state.minions[m.index()]
                        .clan()
                        .is_some_and(|c| clans.contains(&c))
                    {
                        self.state.unlock(m);
                    }
                }
            }
            Effect::PoolToVampires => {
                self.interrupts
                    .push(Interrupt::PoolToVampires { seat: s, to: None });
            }
            Effect::BecomesVampire {
                sect: printed,
                master_search,
                must_hunt,
            } => {
                let sect = printed
                    .or_else(|| ctx.actor.and_then(|a| self.state.sect_of(a)))
                    .expect("a sire with a sect");
                let id = in_play.expect("the card put in play");
                let clan = ctx
                    .actor
                    .and_then(|a| self.state.minions[a.index()].clan())
                    .expect("an acting vampire");
                let m = MinionId(self.state.minions.len() as u32);
                self.state.minions.push(crate::state::Minion {
                    must_hunt,
                    // "Cannot perform actions this turn" (Third Tradition:
                    // Progeny), as a recruited ally.
                    recruited: !must_hunt,
                    ..crate::state::Minion::new(
                        s,
                        crate::state::MinionKind::Created {
                            card: ctx.card,
                            id,
                            clan,
                            sect,
                            born: &crate::state::NEWBORN,
                        },
                        0,
                    )
                });
                self.state.cards[id.index()].body = Some(m);
                if master_search {
                    self.interrupts
                        .push(crate::draws::Interrupt::DisciplineSearch { seat: s, minion: m });
                }
            }
            Effect::RotschreckEndsCombat => {
                let t = ctx.target.minion();
                if let Some(c) = self.combat.as_mut() {
                    c.rotschreck = Some(t);
                    self.end_combat_now();
                }
            }
            // Three copies controlled: every copy in play removed from the
            // game (to their owners'), 3 pool, then the picks.
            Effect::GiveBoonPool => {
                let (Target::Seat(t), Some(id)) = (ctx.target, *in_play) else {
                    unreachable!("a Boon without its Methuselah");
                };
                self.give_boon_pool(s, t, ctx.x, id);
            }
            Effect::AshurTablets => {
                let name = LIBRARY[ctx.card as usize].name;
                let copies: Vec<CardId> = (0..self.state.cards.len())
                    .map(|i| CardId(i as u32))
                    .filter(|&c| {
                        let x = &self.state.cards[c.index()];
                        x.present() && LIBRARY[x.card as usize].name == name
                    })
                    .collect();
                let mine = copies
                    .iter()
                    .filter(|&&c| self.state.card_controller(c) == s)
                    .count();
                if mine >= 3 {
                    for c in copies {
                        self.remove_card_in_play(c);
                    }
                    self.state.seats[s.index()].pool += 3;
                    self.interrupts.push(crate::draws::Interrupt::AshurPicks {
                        seat: s,
                        left: 13,
                        to_library: false,
                    });
                }
            }
            // The master card in play as its ally (Jake Washington): the
            // card and the minion linked, as a recruited ally's; a unique
            // one another Methuselah has is contested.
            Effect::BecomesAlly(body) => {
                let m = MinionId(self.state.minions.len() as u32);
                let id = self.enter_play(CardInPlay {
                    body: Some(m),
                    ..CardInPlay::entering(ctx.card, *ctx.copy, s, None, ctx.mode)
                });
                self.state.minions.push(crate::state::Minion::new(
                    s,
                    crate::state::MinionKind::Ally {
                        card: ctx.card,
                        id,
                        body,
                    },
                    body.life,
                ));
                *in_play = Some(id);
                self.enter_card_contest(m);
            }
            // "Put this card on this vampire" as the combat card resolves
            // (Wall of Filth): the card itself enters play, not the ash heap
            // — one of its name a vampire, a second one to the ash heap.
            Effect::PutOnActorNow | Effect::PutOnActorNowStacking => {
                let actor = ctx.actor.expect("a combatant");
                let placed = self.state.minions[actor.index()]
                    .in_play
                    .then(|| {
                        self.place_now(
                            CardInPlay {
                                by: Some(actor),
                                ..CardInPlay::entering(
                                    ctx.card,
                                    *ctx.copy,
                                    s,
                                    Some(actor),
                                    ctx.mode,
                                )
                            },
                            ModeLayers::FromEffects,
                            effect == Effect::PutOnActorNow,
                        )
                    })
                    .flatten();
                if let Some(id) = placed {
                    *in_play = Some(id);
                    // Inner Essence: for this combat only.
                    let ends = cards::lookup(ctx.card).is_some_and(|d| {
                        d.modes[ctx.mode as usize]
                            .effects
                            .contains(&Effect::BurnAtCombatEnd)
                    });
                    if let (true, Some(c)) = (ends, self.combat.as_mut()) {
                        c.burn_at_end.push(id);
                    }
                }
            }
            // Trap, Conquer the Beast: in play for the combat, 3 counters.
            Effect::TrapPresses | Effect::BeastPresses { .. } => {
                let actor = ctx.actor.expect("a combatant");
                let id = self.enter_play(CardInPlay {
                    by: Some(actor),
                    counters: 3,
                    ..CardInPlay::entering(ctx.card, *ctx.copy, s, None, ctx.mode)
                });
                let c = self.combat.as_mut().expect("combat");
                let owner = match effect {
                    Effect::BeastPresses { optional } => {
                        let i = c.side_of_minion(actor).expect("a combatant");
                        c.sides[i].no_equipment = true;
                        Some((i, optional))
                    }
                    _ => None,
                };
                c.traps.push((id, owner));
                *in_play = Some(id);
            }
            // Morbidity: "put this card in play" with up to n of the
            // opposing vampire's blood, until the combat ends.
            Effect::MorbidityTake(n) => {
                let actor = ctx.actor.expect("a combatant");
                let c = self.combat.as_ref().expect("combat");
                let opposing = c
                    .sides
                    .iter()
                    .map(|side| side.minion)
                    .find(|&m| m != actor)
                    .expect("an opposing minion");
                let moved = n.min(self.state.minions[opposing.index()].blood);
                self.state.minions[opposing.index()].blood -= moved;
                let id = self.enter_play(CardInPlay {
                    by: Some(actor),
                    counters: moved,
                    ..CardInPlay::entering(ctx.card, *ctx.copy, s, None, ctx.mode)
                });
                self.combat
                    .as_mut()
                    .expect("combat")
                    .morbidity
                    .push((id, opposing));
                *in_play = Some(id);
            }
            // Disarm: on the opposing vampire as it resolves, the card
            // itself (rules-qa 47.1); "only one" masked as it is played
            // (`OpponentWithout`). The opposing vampire burned, nothing.
            Effect::PutOnOpponent => {
                let actor = ctx.actor.expect("a combatant");
                let c = self.combat.as_ref().expect("combat");
                let on = c.sides[1 - c.side_of_minion(actor).expect("a combatant")].minion;
                if self.state.minions[on.index()].in_play {
                    *in_play = self.place_now(
                        CardInPlay {
                            by: Some(actor),
                            ..CardInPlay::entering(ctx.card, *ctx.copy, s, Some(on), ctx.mode)
                        },
                        ModeLayers::FromEffects,
                        false,
                    );
                }
            }
            Effect::PutOnTarget
            | Effect::PutInPlay
            | Effect::PutOnActor
            | Effect::PutOnActorRemembering => {
                let host = match (effect, ctx.target) {
                    (Effect::PutOnTarget, Target::Minion(host)) => Some(host),
                    (Effect::PutOnTarget, _) => unreachable!("put-on without a minion target"),
                    (Effect::PutOnActor | Effect::PutOnActorRemembering, _) => {
                        Some(ctx.actor.expect("actor"))
                    }
                    _ => None,
                };
                let against = match effect {
                    Effect::PutOnActorRemembering => self
                        .action
                        .as_ref()
                        .map_or(Default::default(), |a| a.directed_seats(&self.state)),
                    _ => Default::default(),
                };
                // "A vampire can have only one" is a requirement of these
                // modes (`ActorWithout`, `TargetWithout`: Heart of the City,
                // Secure Haven), masked as the card is played.
                *in_play = self.place_now(
                    CardInPlay {
                        by: ctx.actor,
                        discipline: ctx.discipline,
                        against,
                        ..CardInPlay::entering(ctx.card, *ctx.copy, s, host, ctx.mode)
                    },
                    ModeLayers::FromEffects,
                    false,
                );
            }
            // An action card's steal, on success: nothing if the target left
            // play meanwhile (PIB 20121031: still successful, and paid).
            Effect::StealTargetAlly => {
                if let Target::Minion(m) = ctx.target {
                    if self.state.minions[m.index()].present() {
                        self.state.minions[m.index()].owner = s;
                        self.enter_card_contest(m);
                    }
                }
            }
            // Still in hand, still allowed and payable: equipped, not an
            // action (LSJ 20080815).
            Effect::EquipNamed => {
                let (Some((card, mode)), Some(m)) = (ctx.named, ctx.actor) else {
                    unreachable!("an equip without its named card");
                };
                let Some(Terms::EquipFromHand {
                    filter,
                    blood_share,
                }) = cards::lookup(ctx.card).and_then(|d| d.modes[ctx.mode as usize].terms)
                else {
                    unreachable!("an equip without its terms");
                };
                let ok = self.state.minions[m.index()].present()
                    && self
                        .equippable_from_hand(s, m, filter, blood_share)
                        .contains(&(card, mode));
                let share = ctx.x.min(self.blood_share_max(s, m, card));
                let ok = ok && share >= self.blood_share_min(s, m, card);
                if ok {
                    let hand = &mut self.state.seats[s.index()].hand;
                    let i = hand.iter().position(|&c| c == card).expect("in hand");
                    let (_, copy) = hand.take(i);
                    let (pool, blood) = self.pay_named_discounts(
                        ctx,
                        cards::pool_cost(&self.state, s, card, Target::Minion(m)),
                        cards::blood_cost(&self.state, card, 0),
                    );
                    let share = share.min(pool.max(0) as u8);
                    self.state.seats[s.index()].pool -= pool - share as i32;
                    self.state.minions[m.index()].blood -= blood + share;
                    let play = PlayCtx {
                        copy: crate::copies::Untracked(copy),
                        ..PlayCtx::new(s, card, mode, Some(m), Target::Minion(m), 0)
                    };
                    // Paid down to 0 pool: ousted as the play resolves.
                    self.put_equipment(s, m, play);
                    self.replace(s, 1, None);
                }
            }
            // The named retainer from hand, if still there and affordable:
            // its cost paid, requirements as normal, not an action (Pack
            // Alpha [ani]); a card played from hand is replaced.
            Effect::EmployNamed => {
                let (Some((card, mode)), Some(m)) = (ctx.named, ctx.actor) else {
                    unreachable!("an employ without its named card");
                };
                let terms = cards::lookup(ctx.card).and_then(|d| d.modes[ctx.mode as usize].terms);
                let ignore = matches!(terms, Some(Terms::EmployFromHandIgnoring(_)));
                let label = match terms {
                    Some(Terms::RecruitOrEmployFromHand) => None,
                    _ => Some(crate::library::Label::Animal),
                };
                let ok = self.state.minions[m.index()].present()
                    && self
                        .employable_from_hand(s, m, label, ignore)
                        .contains(&(card, mode));
                if ok {
                    let hand = &mut self.state.seats[s.index()].hand;
                    let i = hand.iter().position(|&c| c == card).expect("in hand");
                    let (_, copy) = hand.take(i);
                    let (pool, blood) = self.pay_named_discounts(
                        ctx,
                        cards::pool_cost(&self.state, s, card, Target::Minion(m)),
                        cards::blood_cost(&self.state, card, 0),
                    );
                    self.state.seats[s.index()].pool -= pool;
                    if pool > 0 {
                        self.note_pool_burned(s);
                    }
                    self.state.minions[m.index()].blood -=
                        blood.min(self.state.minions[m.index()].blood);
                    let play = PlayCtx {
                        copy: crate::copies::Untracked(copy),
                        ..PlayCtx::new(s, card, mode, Some(m), Target::Minion(m), 0)
                    };
                    // An ally recruited so cannot act this turn (RBK
                    // recruit-ally).
                    if LIBRARY[card as usize].types.contains(&CardType::Ally) {
                        self.recruit_ally(s, card, copy, mode);
                    } else {
                        self.put_retainer(s, m, play, true);
                    }
                    self.replace(s, 1, None);
                    // Paid down to 0 on one's own turn (Piper): ousted, the
                    // turn passed with it; out of turn, `resolve_play`
                    // ousts them and the turn goes on.
                    if pool > 0 && self.state.seats[s.index()].pool == 0 && s == self.state.active {
                        self.oust_and_pass(s);
                    }
                }
            }
            Effect::StealTargetCard => {
                if let Target::Card(c) = ctx.target {
                    let acting = ctx.actor.expect("the acting minion");
                    if self.state.cards[c.index()].present()
                        && self
                            .state
                            .can_hold(acting, self.state.cards[c.index()].card, &[])
                    {
                        self.state.cards[c.index()].host = Some(acting);
                    }
                }
            }
            Effect::EnterWithCounters(n) => {
                let id = in_play.expect("counters on a card that entered play");
                self.state.cards[id.index()].counters = n;
            }
            Effect::UnlockActorIfClanCapacity { clan, n } => {
                if let Some(m) = ctx.actor {
                    if self.state.minions[m.index()].clan() == Some(clan)
                        && self.state.capacity_of(m).is_some_and(|c| c >= n)
                    {
                        self.effect_unlock(m);
                    }
                }
            }
            Effect::EnterWithCountersPerOwnAnarch => {
                let id = in_play.expect("counters on a card that entered play");
                let n = self
                    .state
                    .minions_of(s)
                    .filter(|&m| {
                        self.state.minions[m.index()].is_vampire()
                            && self.state.sect_of(m) == Some(crate::crypt::Sect::Anarch)
                    })
                    .count();
                self.state.cards[id.index()].counters = n.min(u8::MAX as usize) as u8;
            }
            Effect::EnterWithCountersPerOwnClan(clan) => {
                let id = in_play.expect("counters on a card that entered play");
                let n = self
                    .state
                    .minions_of(s)
                    .filter(|&m| self.state.minions[m.index()].clan() == Some(clan))
                    .count();
                self.state.cards[id.index()].counters = n.min(u8::MAX as usize) as u8;
                // "If this location has no counters, burn it."
                if n == 0 {
                    self.burn_card_in_play(id);
                }
            }
            Effect::MoveBloodTargetToPoolX => {
                let t = ctx.target.minion();
                let moved = ctx.x.min(self.state.minions[t.index()].blood);
                self.state.minions[t.index()].blood -= moved;
                self.state.seats[s.index()].pool += moved as i32;
            }
            Effect::Layer(modifier) => {
                let source = in_play.expect("layer source must be in play");
                self.state.layers.push(Layer {
                    scope: Scope::InPlay,
                    modifier,
                    source,
                });
            }
            // Kaymakli Nightmares: each Methuselah's crypt cards in the
            // uncontrolled region back to the crypt, their blood to the pool,
            // the cards on them burned; a created vampire burned instead (LSJ
            // 20041201-2), not counted; the crypt shuffled, as many drawn
            // out.
            Effect::KaymakliNightmares => {
                for t in self.state.clockwise_from(s) {
                    let slots = std::mem::take(&mut self.state.seats[t.index()].uncontrolled);
                    let mut n = 0;
                    for slot in slots {
                        self.state.seats[t.index()].pool += slot.counters as i32;
                        if let Some(b) = slot.banished {
                            for c in b.cards {
                                let ci = &self.state.cards[c.index()];
                                let (owner, card, copy) = (ci.owner, ci.card, *ci.copy);
                                self.state.seats[owner.index()].ash.put(card, copy);
                            }
                        }
                        if slot.card != crate::state::CREATED {
                            self.state.seats[t.index()].crypt.put(slot.card, *slot.copy);
                            n += 1;
                        }
                    }
                    self.state.seats[t.index()].crypt.shuffle(&mut self.rng);
                    for _ in 0..n {
                        let seat = &mut self.state.seats[t.index()];
                        let Some((card, copy)) = seat.crypt.take_top() else {
                            break;
                        };
                        seat.uncontrolled.push(crate::state::UncontrolledSlot {
                            card,
                            copy: crate::copies::Untracked(copy),
                            counters: 0,
                            banished: None,
                        });
                    }
                }
            }
            // The tops revealed, not moved; a crypt emptied since shows
            // nothing.
            Effect::PetraResonance => {
                let shown: Vec<(SeatId, u8)> = self
                    .state
                    .clockwise_from(s)
                    .into_iter()
                    .filter_map(|t| {
                        let top = *self.state.seats[t.index()].crypt.last()?;
                        Some((t, crate::crypt::CRYPT[top as usize].capacity))
                    })
                    .collect();
                if let (Some(hi), Some(lo)) = (
                    shown.iter().map(|&(_, c)| c).max(),
                    shown.iter().map(|&(_, c)| c).min(),
                ) {
                    let count = |c: u8| shown.iter().filter(|&&(_, k)| k == c).count();
                    let mut loss = vec![0; self.state.seats.len()];
                    if count(hi) > 1 || count(lo) > 1 {
                        for &(t, _) in &shown {
                            loss[t.index()] = 1;
                        }
                    } else if hi != lo {
                        let (winner, loser) = (
                            shown.iter().find(|&&(_, c)| c == hi).expect("highest").0,
                            shown.iter().find(|&&(_, c)| c == lo).expect("lowest").0,
                        );
                        let taken = 3.min(self.state.seats[loser.index()].pool);
                        self.state.seats[winner.index()].pool += taken;
                        loss[loser.index()] = taken;
                    }
                    for &(t, _) in &shown {
                        self.state.seats[t.index()].crypt.shuffle(&mut self.rng);
                    }
                    self.burn_pool_batch(&loss);
                }
            }
            Effect::SearchLibraryToHand(filter) => {
                self.interrupts
                    .push(Interrupt::LibraryToHand { seat: s, filter });
            }
            // Chameleon: the stolen vampire becomes Anarch.
            Effect::TargetBecomesAnarch => {
                let t = ctx.target.minion();
                if self.state.minions[t.index()].present() {
                    self.make_anarch(t);
                }
            }
            // Third Tradition: Progeny: up to n of the acting vampire's
            // blood to the new vampire, decided now (PIB 20150418).
            Effect::ActingBloodToNewUpTo(n) => {
                let to = in_play.and_then(|id| self.state.cards[id.index()].body);
                if let (Some(from), Some(to)) = (ctx.actor, to) {
                    self.interrupts.push(Interrupt::MoveBloodUpTo {
                        seat: s,
                        from,
                        to,
                        max: n,
                    });
                }
            }
            // Redeem the Lost Soul: the vampire still in the ash heap.
            Effect::RedeemAshVampire => {
                let seat = &mut self.state.seats[s.index()];
                if let Some(i) = ctx
                    .ash
                    .and_then(|(_, card)| seat.ash_crypt.iter().position(|&c| c == card))
                {
                    let (card, copy) = seat.ash_crypt.take(i);
                    seat.removed_crypt.put(card, copy);
                    let cap = crate::crypt::CRYPT[card as usize].capacity;
                    seat.pool += (cap / 2) as i32;
                }
            }
            // "Steal 1 blood or life from a minion" (Tier of Souls): an ally
            // left with no life is burned; blood the acting vampire cannot
            // hold goes to the bank.
            Effect::StealBloodFromTarget(n) => {
                let t = ctx.target.minion();
                let v = &mut self.state.minions[t.index()];
                let taken = n.min(v.blood);
                v.blood -= taken;
                let lifeless = v.is_ally() && v.blood == 0;
                if let Some(a) = ctx.actor {
                    if self.state.minions[a.index()].present() {
                        self.state.gain_blood(a, taken);
                    }
                }
                if lifeless {
                    self.burn_minion(t);
                }
            }
            // The ally named as the card was played, if still in that ash
            // heap (else no effect); the acting vampire gains the blood (a
            // hunt: successful if any, LSJ 20050720-2), a hunt bonus more
            // from the bank (The Hungry Coyote; owner, 2026-10-08:
            // rules-qa 53.8).
            Effect::EatAshAlly { blood, unlock } => {
                let (Some((from, card)), Some(acting)) = (ctx.ash, ctx.actor) else {
                    unreachable!("an ash ally named at play");
                };
                let hunt = cards::lookup(ctx.card).is_some_and(|d| {
                    d.modes[ctx.mode as usize]
                        .effects
                        .contains(&Effect::HuntAction)
                });
                let blood = blood
                    + if hunt {
                        self.state.hunt_bonus(acting)
                    } else {
                        0
                    };
                let seat = &mut self.state.seats[from.index()];
                if let Some(k) = seat.ash.iter().rposition(|&c| c == card) {
                    let (_, copy) = seat.ash.take(k);
                    seat.removed.put(card, copy);
                    if self.state.minions[acting.index()].present() {
                        let before = self.state.minions[acting.index()].blood;
                        if hunt {
                            self.state.hunt_blood(acting, blood);
                        } else {
                            self.state.gain_blood(acting, blood);
                        }
                        if unlock {
                            self.state.unlock(acting);
                        }
                        if self.state.minions[acting.index()].blood > before {
                            self.successful_hunt_riders(acting);
                        }
                    }
                }
            }
            // Putrescent Sustenance: the named ash heap ally out of the game,
            // life to the zombie ally chosen.
            Effect::EatAshAllyToTarget(n) => {
                let Some((from, card)) = ctx.ash else {
                    unreachable!("an ash ally named at play");
                };
                let seat = &mut self.state.seats[from.index()];
                if let Some(k) = seat.ash.iter().rposition(|&c| c == card) {
                    let (_, copy) = seat.ash.take(k);
                    seat.removed.put(card, copy);
                    if let Target::Minion(t) = ctx.target {
                        if self.state.minions[t.index()].present() {
                            let b = self.state.minions[t.index()].blood + n;
                            self.state.set_blood(t, b);
                        }
                    }
                }
            }
            // "Gains enough blood to reach full capacity" (Giant's Blood).
            Effect::BloodToTargetFull => {
                let t = ctx.target.minion();
                if let Some(cap) = self.state.capacity_of(t) {
                    let b = self.state.minions[t.index()].blood.max(cap);
                    self.state.set_blood(t, b);
                }
            }
            // Removed from the game, not burned; its controller gains pool
            // equal to its capacity (Golconda: Inner Peace).
            Effect::RemoveTargetForCapacityPool => {
                let t = ctx.target.minion();
                if self.state.minions[t.index()].present() {
                    let cap = self.state.capacity_of(t).unwrap_or(0);
                    let v = &self.state.minions[t.index()];
                    let controller = v.owner;
                    let owner = v.origin.or(v.temp_from).unwrap_or(controller);
                    let card = v.crypt_card();
                    let copy = *v.copy;
                    // The cards on it are burned as it leaves play (advanced
                    // rules §1.12.1), its owner's too.
                    for ci in 0..self.state.cards.len() {
                        let c = &self.state.cards[ci];
                        if c.in_play && c.host == Some(t) {
                            self.burn_card_in_play(CardId(ci as u32));
                        }
                    }
                    self.remove_minion(t);
                    if let Some(c) = card {
                        self.state.seats[owner.index()].removed_crypt.put(c, copy);
                    }
                    self.state.seats[controller.index()].pool += cap as i32;
                }
            }
            // "Move up to 3 blood from a ready vampire you control to a
            // younger vampire of the same clan in your uncontrolled region"
            // (Grooming the Protégé): X named at play, what the vampire has.
            Effect::MoveBloodXToUncontrolledTarget => {
                let Target::Uncontrolled(i) = ctx.target else {
                    unreachable!("a move without an uncontrolled target");
                };
                let a = ctx.actor.expect("the vampire moving the blood");
                let n = ctx.x.min(self.state.minions[a.index()].blood);
                self.state.minions[a.index()].blood -= n;
                self.state.add_uncontrolled_blood(s, i as usize, n);
            }
            Effect::Stun => {
                let t = ctx.target.minion();
                let v = &mut self.state.minions[t.index()];
                v.locked = true;
                v.skip_unlock = true;
            }
            // Its control changes; one on a minion moves onto a ready
            // minion of the player's, their choice (RTR 19960112), or is
            // burned if none can hold it (LSJ 19990928).
            Effect::FourthCycle(cap) => {
                for i in 0..self.state.cards.len() {
                    let c = &self.state.cards[i];
                    if c.present()
                        && !c.effect_only
                        && LIBRARY[c.card as usize].labels.contains(&Label::Gehenna)
                    {
                        self.burn_card_in_play(CardId(i as u32));
                    }
                }
                let small: Vec<MinionId> = (0..self.state.minions.len())
                    .map(|i| MinionId(i as u32))
                    .filter(|&m| {
                        let v = &self.state.minions[m.index()];
                        v.ready()
                            && v.is_vampire()
                            && self.state.capacity_of(m).is_some_and(|c| c <= cap)
                    })
                    .collect();
                if !small.is_empty() {
                    self.minions_to_torpor(&small);
                }
            }
            Effect::StealLocation => {
                if let Target::Card(c) = ctx.target {
                    if self.state.cards[c.index()].present() {
                        self.give_location(c, s);
                    }
                }
            }
            // For good: the cards on it go with it; its ash heap stays its
            // owner's; a title it carries may now be contested.
            Effect::StealTargetVampire => {
                let t = ctx.target.minion();
                let v = &mut self.state.minions[t.index()];
                if v.owner != s {
                    let origin = v.origin.or(v.temp_from).unwrap_or(v.owner);
                    v.origin = (origin != s).then_some(origin);
                    v.temp_from = None;
                    v.owner = s;
                    self.enter_title_contest(t);
                }
            }
            Effect::StolenToReadyForBlood(n) => {
                let t = ctx.target.minion();
                if let Some(a) = ctx.actor {
                    self.interrupts.push(Interrupt::MoveForBlood {
                        seat: s,
                        payer: a,
                        minion: t,
                        n,
                    });
                }
            }
            // Read before the card burns (Rewilding: "burn a location and
            // burn 2 pool from its controller").
            Effect::TargetControllerBurnsPool(n) => {
                if let Target::Card(c) = ctx.target {
                    let controller = self.state.card_controller(c);
                    let mut loss = vec![0; self.state.seats.len()];
                    loss[controller.index()] = n as i32;
                    self.burn_pool_batch(&loss);
                }
            }
            Effect::DiscardFromTargetHand => {
                let Target::Seat(t) = ctx.target else {
                    unreachable!("a discard without a seat target");
                };
                self.interrupts
                    .push(crate::draws::Interrupt::DiscardFromHandOf { seat: s, of: t });
            }
            Effect::LockPlayedCard => {
                if let Some(id) = *in_play {
                    self.state.cards[id.index()].locked = true;
                }
            }
            // Rutor's Hand: the damage, or [THA] 3 blood instead if the
            // vampire has them (its controller's choice).
            Effect::ActorTakesAggravated { damage, avoid_for } => {
                let m = ctx.actor.expect("actor");
                let v = &self.state.minions[m.index()];
                if avoid_for > 0 && v.blood >= avoid_for {
                    self.interrupts.push(crate::draws::Interrupt::AvoidDamage {
                        seat: s,
                        minion: m,
                        blood: avoid_for,
                        damage,
                    });
                } else if v.present() {
                    self.damage_out_of_combat(m, damage, true);
                }
            }
            Effect::StealPoolFromTarget(n) => {
                let Target::Seat(t) = ctx.target else {
                    unreachable!("a pool steal without a seat target");
                };
                let taken = (n as i32).min(self.state.seats[t.index()].pool);
                self.state.seats[s.index()].pool += taken;
                let mut loss = vec![0; self.state.seats.len()];
                loss[t.index()] = taken;
                self.burn_pool_batch(&loss);
            }
            Effect::RemovePreyAshToBurnPool { remove, pool } => {
                let Target::Seat(t) = ctx.target else {
                    unreachable!("an ash removal without a seat target");
                };
                self.interrupts
                    .push(crate::draws::Interrupt::RemoveFromAsh {
                        seat: s,
                        of: t,
                        left: remove,
                        pool,
                    });
            }
            Effect::BurnChosenInPlay => {
                if let Some(id) = ctx.burn {
                    self.burn_card_in_play(id);
                }
            }
            // A bonus whose condition is read live for the rest of the
            // action.
            Effect::If { on, requires, then } => {
                let a = self
                    .action
                    .as_mut()
                    .expect("a conditional bonus during an action");
                let on = match on {
                    cards::On::Actor => ctx.actor.expect("actor"),
                    cards::On::Acting => a.acting,
                };
                let (stat, n, to, limited) = match *then {
                    Effect::BleedMod { n, limited } => {
                        (cards::Stat::Bleed, n as i8, a.acting, limited)
                    }
                    Effect::AddIntercept(n) => {
                        let to = ctx
                            .actor
                            .or(match a.state {
                                crate::actions::ActionState::Attempt(m) => Some(m),
                                _ => None,
                            })
                            .expect("intercepting minion");
                        (cards::Stat::Intercept, n as i8, to, false)
                    }
                    other => unreachable!("not a live conditional bonus: {other:?}"),
                };
                a.conditional.push(crate::actions::Conditional {
                    stat,
                    to,
                    on,
                    seat: s,
                    requires,
                    n,
                    limited,
                });
            }
            e => match e.family() {
                Family::Shared => self.apply_shared_effect(e, ctx, By::Play(*in_play)),
                Family::Action => self.apply_action_effect(e, ctx),
                Family::Combat => self.apply_combat_effect(e, ctx),
                // A mode shared between a bleed and the polling step
                // (Protected District) carries both effects; the one
                // without a context does nothing.
                Family::Vote => {
                    if self.referendum.is_some() {
                        self.apply_vote_effect(e, ctx)
                    }
                }
                Family::Marker => {}
                Family::Play => unreachable!("a play effect without its arm: {e:?}"),
                f @ (Family::Ability | Family::Success | Family::Entry | Family::Rider) => {
                    unreachable!("{f:?} effect in a play: {e:?}")
                }
            },
        }
    }

    /// One effect resolved the same from a play and from an in-play ability
    /// (`Family::Shared`): `ctx` is the play's or the ability's, whose actor
    /// is its host (or the user's combatant).
    pub(crate) fn apply_shared_effect(&mut self, effect: Effect, ctx: &PlayCtx, by: By) {
        match effect {
            // The target of a master card (Life in the City) or the
            // vampire in the uncontrolled region an action card chose
            // (Govern the Unaligned [DOM]); re-checked at resolution.
            Effect::BloodToTarget(n) => {
                let t = ctx.target.minion();
                self.state.gain_blood(t, n);
            }
            Effect::BloodToUncontrolledTarget(n) => {
                let Target::Uncontrolled(i) = ctx.target else {
                    unreachable!("blood to the uncontrolled region without a slot");
                };
                self.state.add_uncontrolled_blood(ctx.seat, i as usize, n);
            }
            // "Add 3 blood or life to a minion, not to exceed their starting
            // life" (Touch of Valeren [ACTION]); "add enough life to …
            // reach their starting life" (Mobile HQ): up to it, no further.
            Effect::BloodToTargetToStart(n) => {
                if let Target::Minion(t) = ctx.target {
                    let v = &self.state.minions[t.index()];
                    if v.present() {
                        // Renewed Vigor's "enough … to reach" passes u8::MAX.
                        let b = self.state.cap_starting(t, v.blood.saturating_add(n));
                        self.state.set_blood(t, b);
                    }
                }
            }
            // "This vampire gains n blood": the minion playing the card
            // (Restoration); an ability's actor is its host, not the acting
            // minion it helps.
            Effect::BloodToActing(n) => {
                let to = match by {
                    By::Play(_) => ctx.actor,
                    By::Ability(_) => self.action.as_ref().map(|a| a.acting),
                };
                if let Some(a) = to {
                    if self.state.minions[a.index()].present() {
                        self.state.gain_blood(a, n);
                    }
                }
            }
            // Loki's Gift [PRO]: "burn 1 blood from any vampire".
            Effect::TargetBurnsBlood(n) => {
                if let Target::Minion(t) = ctx.target {
                    self.state.burn_blood(t, n);
                }
            }
            // Vulnerability: the vampire still in play (re-checked).
            Effect::BurnTargetMinion => {
                if let Target::Minion(t) = ctx.target {
                    if self.state.minions[t.index()].in_play {
                        self.burn_minion(t);
                    }
                }
            }
            // The target card, if still in play (Pack Alpha [ANI]: the
            // animal retainer burned; War Ghoul's location).
            Effect::BurnTargetCard => {
                if let Target::Card(c) = ctx.target {
                    if self.state.cards[c.index()].in_play {
                        self.burn_card_in_play(c);
                    }
                }
            }
            // Renewed Vigor [obe]: a vampire out of torpor (the card action
            // re-checked its target).
            Effect::TargetLeavesTorpor => {
                if let Target::Minion(t) = ctx.target {
                    if self.state.minions[t.index()].present() {
                        self.leave_torpor(t);
                    }
                }
            }
            Effect::LockTarget => {
                if let Target::Minion(t) = ctx.target {
                    self.state.minions[t.index()].locked = true;
                }
            }
            // The target's filter decides whether it must be ready: an
            // action card's is re-checked as it succeeds (Hive Mind's), an
            // ability's met as it is used (Zizi and Metro Underground
            // unlock a locked vampire, ready or not).
            Effect::UnlockTarget => {
                if let Target::Minion(t) = ctx.target {
                    self.effect_unlock(t);
                }
            }
            // In an action or out of one (Piper's Anarch in the master
            // phase, Daria's lock).
            Effect::LockActor => {
                self.state.minions[ctx.actor.expect("actor").index()].locked = true;
            }
            // Hive Mind [san]: the acting Blood Brother, at its action's
            // resolution; during an action, the action's own rule ("does
            // not unlock", Daring the Dawn) applies (Kalinda's unlock).
            Effect::UnlockActor => self.effect_unlock(ctx.actor.expect("actor")),
            // A played combat card ending the combat before range (Alpha
            // Glint [ANI][FOR]).
            Effect::EndCombat => {
                if self.combat.is_some() {
                    self.end_combat_now();
                }
            }
            Effect::GainPool(n) => self.state.seats[ctx.seat.index()].pool += n as i32,
            // "Your prey burns n pool" (Predator's Transformation [ABO]);
            // "burn 1 pool from your prey" (Privileged Position).
            Effect::PreyBurnsPool(n) => {
                let mut loss = vec![0i32; self.state.seats.len()];
                loss[self.state.prey(ctx.seat).index()] = n as i32;
                self.burn_pool_batch(&loss);
            }
            // Redistribution: the acting vampire's circle; Jerry: his
            // circle's blood moved among his Methuselah's.
            Effect::CircleBloodMoves { equipment } => {
                if let Some(c) = ctx
                    .actor
                    .and_then(|m| self.state.minions[m.index()].circle())
                {
                    self.interrupts.push(Interrupt::CircleBlood {
                        seat: ctx.seat,
                        circle: c,
                        from: None,
                        to: None,
                        equipment,
                        moving: None,
                        paired: [None; 12],
                    });
                }
            }
            // On the card that entered play, or on the ability's (Shilmulo
            // Tarot, on equip and in its unlock ability).
            Effect::StoreTopCards(n) => {
                let id = match by {
                    By::Play(in_play) => in_play.expect("cards stored on a card that entered play"),
                    By::Ability(source) => source.card(),
                };
                self.store_top_cards(id, n);
            }
            other => unreachable!("not a shared effect: {other:?}"),
        }
    }

    pub(crate) fn discard_request(&self) -> DecisionRequest {
        let s = self.state.active;
        let seat = &self.state.seats[s.index()];
        // A burn due (The Uncoiling, Dragonbound): no passing the impulse
        // before it (rules-qa 40.5).
        let due: Vec<Choice> = self
            .discard_dues()
            .into_iter()
            .map(|u| Choice::ResolveRider { card: u.0 })
            .collect();
        let mut options = if due.is_empty() {
            vec![Choice::Pass]
        } else {
            due
        };
        if seat.dpa > 0 {
            options.extend(
                (0..seat.hand.len())
                    .filter(|&i| crate::state::first_copy(&seat.hand, i))
                    .map(|i| Choice::Discard(i as u8)),
            );
            // An event card put in play with a discard phase action, no
            // more than one each phase (rulebook "Discard Phase").
            if !seat.event_played {
                options.extend(self.play_options(s, &[Window::DiscardPhase], None));
            }
        }
        // Fiendish Tongue: "this Tzimisce", the clan read again in the
        // discard phase (rules-qa 54.5).
        for m in self.state.minions_of(s) {
            let v = &self.state.minions[m.index()];
            if v.discard_unlock.is_some_and(|(clan, n)| {
                v.clan() == Some(clan) && v.ready() && v.locked && v.blood >= n
            }) {
                options.push(Choice::UnlockForBlood { minion: m });
            }
        }
        // "During your discard phase" abilities (Mora, the Death Seer), and
        // those spending a discard phase action (Ayse Dhanial).
        options.extend(self.ability_options(s, Window::DiscardPhase));
        options.extend(self.ability_options(s, Window::DiscardPhaseAction));
        options.extend(self.ability_options(s, Window::AnyImpulse));
        DecisionRequest::new(s, WindowLabel::DiscardPhase, options)
    }

    /// "During your discard phase" riders printed on the active
    /// Methuselah's vampires (Roy), applied as the impulse leaves them.
    fn discard_phase_vampire_riders(&mut self) {
        let s = self.state.active;
        let ms: Vec<MinionId> = self.state.minions_of(s).collect();
        for m in ms {
            if !self.state.minions[m.index()].present() {
                continue;
            }
            for r in self.printed_riders(m, Hook::DiscardPhase, Some(s)) {
                for e in r.effects {
                    match *e {
                        Effect::LockActor => self.state.minions[m.index()].locked = true,
                        other => unreachable!("not a discard-phase rider effect: {other:?}"),
                    }
                }
            }
        }
    }

    pub(crate) fn discard_apply(&mut self, choice: &Choice) {
        let s = self.state.active;
        match choice {
            Choice::Pass => {
                self.discard_phase_vampire_riders();
                self.open_phase_rotation(Flow::Discard)
            }
            Choice::UnlockForBlood { minion } => {
                let v = &mut self.state.minions[minion.index()];
                v.blood -= v.discard_unlock.take().map_or(0, |(_, n)| n);
                self.state.unlock(*minion);
                self.flow = Flow::Discard;
            }
            Choice::Discard(i) => {
                let seat = &mut self.state.seats[s.index()];
                let (card, copy) = seat.hand.take(*i as usize);
                seat.ash.put(card, copy);
                // The player spends whichever discard phase action serves
                // best: Powerbase: Los Angeles's on a card requiring an
                // Anarch or making a vampire Anarch, another one otherwise.
                let labels = LIBRARY[card as usize].labels;
                let anarch =
                    labels.contains(&Label::RequiresAnarch) || labels.contains(&Label::MakesAnarch);
                let unlock = anarch && seat.granted_dpa > 0;
                if unlock || seat.dpa == seat.granted_dpa {
                    seat.granted_dpa -= 1;
                }
                seat.dpa -= 1;
                self.replace(s, 1, None);
                // Camarilla Threat: the additional pool, a last one ousting.
                let toll = self.state.discard_toll();
                if toll > 0 {
                    self.pay_pool_for_ability(s, toll as i32);
                    if self.state.seats[s.index()].ousted {
                        return;
                    }
                }
                if unlock {
                    self.interrupts.push(Interrupt::MinionOf {
                        seat: s,
                        of: s,
                        purpose: crate::draws::MinionPurpose::UnlockAnarch,
                    });
                }
                self.flow = Flow::Discard;
            }
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                if !self.use_ability(s, *source, *ability, *actor, *target, Flow::Discard) {
                    self.flow = Flow::Discard;
                }
            }
            Choice::ResolveRider { card } => {
                let c = CardId(*card);
                self.state.seats[s.index()].discard_resolved.push(c);
                self.flow = Flow::Discard;
                let dragonbound = self
                    .state
                    .layers
                    .iter()
                    .any(|l| l.source == c && l.modifier == cards::Modifier::TorporPoolBurn);
                let own_rider = cards::lookup(self.state.cards[c.index()].card).is_some_and(|d| {
                    d.riders.iter().any(|r| {
                        r.hook == Hook::DiscardPhase
                            && !r.effects.contains(&Effect::PredatorTakesControl)
                            && !r.effects.is_empty()
                    })
                });
                if own_rider {
                    self.resolve_discard_rider(c);
                } else if dragonbound {
                    // Burned now; out of pool, ousted at once.
                    let n = self
                        .state
                        .minions_of(s)
                        .filter(|&m| {
                            let v = &self.state.minions[m.index()];
                            v.is_vampire() && v.in_torpor
                        })
                        .count() as i32;
                    self.state.seats[s.index()].pool -= n;
                    if self.state.seats[s.index()].pool <= 0 {
                        self.oust_and_pass(s);
                    }
                } else {
                    self.uncoiling(c);
                }
            }
            // An event: a discard phase action — any; an ordinary one is
            // spent first, which leaves Powerbase: Los Angeles's unlock
            // open (rules-qa 40.1).
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                let seat = &mut self.state.seats[s.index()];
                if seat.dpa == seat.granted_dpa {
                    seat.granted_dpa -= 1;
                }
                seat.dpa -= 1;
                seat.event_played = true;
                self.announce_play(s, *card, *mode, *actor, *target, Flow::Discard);
            }
            other => unreachable!("illegal discard choice {other:?}"),
        }
    }

    /// In-play abilities `seat` may use in `window`: host abilities of its
    /// cards and granted actions of any card in play ("Vampires can call a
    /// referendum …"); each actor branch finds the candidate users,
    /// `ability_gate` checks them.
    pub(crate) fn ability_options(&self, seat: SeatId, window: Window) -> Vec<Choice> {
        let st = &self.state;
        let mut out = Vec::new();
        for (ci, c) in st.cards.iter().enumerate() {
            let id = CardId(ci as u32);
            // A contested card is out of play: nothing printed on it
            // applies; nor on a card that became another (Enhanced
            // Coagulant).
            if !c.present() || cards::became(c.card, c.mode) || st.text_suppressed(id) {
                continue;
            }
            let Some(def) = cards::lookup(c.card) else {
                continue;
            };
            let source = AbilitySource::Card(id);
            for (ai, ability) in def.abilities.iter().enumerate() {
                if !ability.windows.contains(&window) {
                    continue;
                }
                let gate = |user| self.ability_gate(seat, window, source, ai as u8, user);
                match ability.actor {
                    // The controller picks the ability's target (a hunting
                    // ground: "a ready vampire you control can gain 1 blood").
                    Actor::Controller => {
                        if gate(None).is_none() {
                            continue;
                        }
                        for target in self.ability_targets(seat, ability.target, c.host) {
                            // A vampire younger than this card's counters
                            // (Pit of Contemplation).
                            if let (true, Target::Minion(t)) = (
                                ability.target.is_some_and(|sp| {
                                    sp.filter
                                        .contains(&Requirement::TargetCapacityBelowCounters)
                                }),
                                target,
                            ) {
                                let cap = st.capacity_of(t).unwrap_or(u8::MAX);
                                if cap >= c.counters {
                                    continue;
                                }
                            }
                            out.push(use_card(id, ai, None, target));
                        }
                    }
                    // The acting minion, whoever controls the card ("an
                    // Anarch can burn 1 blood … as they announce a bleed
                    // action", Club Illusion).
                    Actor::ActingMinion => {
                        let Some(a) = self.action.as_ref() else {
                            continue;
                        };
                        let m = a.acting;
                        if !st.minions[m.index()].present() || gate(None).is_none() {
                            continue;
                        }
                        out.push(use_card(id, ai, Some(m), Target::None));
                    }
                    // Kaymakli Fragment: the bearer's opponent in combat.
                    Actor::OpposingCombatant => {
                        let (Some(host), Some(cb)) = (c.bearer(), self.combat.as_ref()) else {
                            continue;
                        };
                        let Some(i) = cb.side_of_minion(host) else {
                            continue;
                        };
                        let opp = cb.sides[1 - i].minion;
                        if gate(Some(opp)).is_none() {
                            continue;
                        }
                        out.push(use_card(id, ai, Some(opp), Target::Card(id)));
                    }
                    // Any ready vampire of the user's, whoever controls the
                    // card (Carver's Meat Packing and Storage).
                    Actor::AnyReadyVampire => {
                        let users: Vec<MinionId> = st
                            .minions_of(seat)
                            .filter(|&m| {
                                st.minions[m.index()].ready()
                                    && st.minions[m.index()].is_vampire()
                                    && gate(Some(m)).is_some()
                            })
                            .collect();
                        for m in users {
                            for target in self.ability_targets(seat, ability.target, None) {
                                out.push(use_card(id, ai, Some(m), target));
                            }
                        }
                    }
                    // The chosen vampire's controller, the vampire in
                    // play (in torpor too; not uncontrolled: PIB 20150522).
                    Actor::ChosenController => {
                        let Some(&m) = c.chosen.first() else {
                            continue;
                        };
                        if !st.minions[m.index()].present() || gate(None).is_none() {
                            continue;
                        }
                        out.push(use_card(id, ai, None, Target::None));
                    }
                    // "Methuselahs can …": any Methuselah, in their own
                    // window, whoever controls the card.
                    Actor::Methuselah => {
                        if gate(None).is_some() {
                            for target in self.ability_targets(seat, ability.target, None) {
                                out.push(use_card(id, ai, None, target));
                            }
                        }
                    }
                    // The host's controller uses it (a card put on another
                    // Methuselah's vampire is theirs to use) — for the host,
                    // or, from out of the combat, for its controller's
                    // combatant (Underbridge Stray's press).
                    Actor::Host | Actor::ForOwnCombatant => {
                        let for_own = ability.actor == Actor::ForOwnCombatant;
                        // An unhosted card works for its controller's
                        // combatant (Dance of the Dead).
                        let unhosted = self
                            .combat
                            .as_ref()
                            .filter(|_| {
                                for_own && c.bearer().is_none() && st.card_controller(id) == seat
                            })
                            .and_then(|cb| cb.side_of(seat).map(|i| cb.sides[i].minion));
                        let Some(host) = c
                            .bearer()
                            .filter(|h| st.minions[h.index()].owner == seat)
                            .or(unhosted)
                        else {
                            continue;
                        };
                        if !st.minions[host.index()].present() {
                            continue;
                        }
                        // In combat, the host must be the seat's combatant —
                        // or the seat have one, for its own combatant.
                        let placed = match self.combat.as_ref() {
                            Some(cb) if for_own => cb.side_of(seat).is_some(),
                            Some(cb) => cb.side_of(seat) == cb.side_of_minion(host),
                            None => !for_own,
                        };
                        if !placed || gate(None).is_none() {
                            continue;
                        }
                        let declares = declared_kind(ability);
                        // A target of its own (War Ghoul's rush).
                        for target in self.ability_targets(seat, ability.target, Some(host)) {
                            if declares.is_some() && !self.may_direct(seat, host, target) {
                                continue;
                            }
                            out.push(use_card(id, ai, declares.map(|_| host), target));
                        }
                    }
                    Actor::ActingCandidate => {
                        for m in self.actor_candidates(seat, ability).into_iter().flatten() {
                            if gate(Some(m)).is_none() {
                                continue;
                            }
                            // "Another Ravnos" (Week of Nightmares): never the
                            // actor itself — no action sure to fizzle
                            // (rules-qa 17.4).
                            let other = ability
                                .target
                                .is_some_and(|sp| sp.filter.contains(&Requirement::OtherThanActor));
                            for target in
                                self.ability_targets_by(seat, ability.target, c.host, Some(m))
                            {
                                if other && target == Target::Minion(m) {
                                    continue;
                                }
                                // An action at a minion it may not direct
                                // actions at (Secure Haven, Shadow Cloak).
                                if !self.may_direct(seat, m, target) {
                                    continue;
                                }
                                out.push(use_card(id, ai, Some(m), target));
                            }
                        }
                    }
                    other => unreachable!("not an ability actor: {other:?}"),
                }
            }
        }
        self.vampire_ability_options(seat, window, &mut out);
        out
    }

    /// The checks every in-play ability passes, a card's or a vampire's,
    /// for the candidate `user`: its phase, its limits, a payable cost, a
    /// granted action its actor may take, every requirement
    /// (`cards::slots`). Returns the ability's user.
    fn ability_gate(
        &self,
        seat: SeatId,
        window: Window,
        source: AbilitySource,
        ai: u8,
        user: Option<MinionId>,
    ) -> Option<AbilityUser> {
        let st = &self.state;
        let ability = self.ability_mode(source, ai);
        let u = self.ability_user(source, ai, seat, user);
        let card = match source {
            AbilitySource::Card(id) => Some(id),
            AbilitySource::Vampire(_) => None,
        };
        if u.seat != seat {
            return None;
        }
        // Its phase ("during your minion phase", Lord Tremere).
        if ability.requires.contains(&Requirement::OthersMinionPhase)
            && !self.in_others_minion_phase(seat)
            || ability.requires.contains(&Requirement::OwnMinionPhase)
                && !self.in_own_minion_phase(seat)
            || ability.requires.contains(&Requirement::InMasterPhase)
                && self.phase() != crate::game::Phase::Master
        {
            return None;
        }
        // "During X" abilities: once per phase per copy (rules-qa 6.6) —
        // per mode with `Limit::PerPhaseEach`. Not an ability paid with a
        // master phase action (Open War, Twilight Camp): as often as they
        // are spent.
        let per_mpa = ability.effects.contains(&Effect::SpendMasterPhaseAction);
        if window.is_phase()
            && !per_mpa
            && st
                .phase_uses
                .contains(&phase_key(source, ai, ability.limit))
        {
            return None;
        }
        // The same effect twice a phase (Carfax Abbey's second pick, with a
        // baron): the later waits for the earlier, the options being the
        // same.
        if ability.limit == Some(Limit::PerPhaseEach)
            && source.abilities(st)[..ai as usize]
                .iter()
                .enumerate()
                .any(|(bi, b)| {
                    b.limit == Some(Limit::PerPhaseEach)
                        && b.effects == ability.effects
                        && b.windows == ability.windows
                        && !st
                            .phase_uses
                            .contains(&phase_key(source, bi as u8, b.limit))
                })
        {
            return None;
        }
        // Per-copy limits ("each combat", "once each turn": by any
        // vampire, attempted — Black Forest Base, LSJ 20091224). Kevlar
        // Vest's gun branch stays usable while points are left (rules-qa
        // 10.3).
        let allowance = ability.limit == Some(Limit::PerCombat)
            && ability
                .effects
                .iter()
                .any(|e| matches!(e, Effect::PreventGunStrikeAllowance(_)))
            && self.combat.as_ref().is_some_and(|cb| {
                cb.gun_prevention_left
                    .iter()
                    .any(|&(c, n)| Some(c) == card && n > 0)
            });
        if self.limit_spent(ability.limit, source, ai) && !allowance {
            return None;
        }
        // An ally never pays a blood cost (advanced rules 5.5.1).
        if u.host.is_some_and(|h| st.minions[h.index()].is_ally())
            && ability.effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::PayBlood(_)
                        | Effect::HostBurnBlood(_)
                        | Effect::HostBurnBloodToUnlock
                        | Effect::HostBloodToPool(_)
                )
            })
        {
            return None;
        }
        // "Burn X pool": payable, down to 0 at most (The Uncoiling).
        if let (true, Some(id)) = (
            ability.effects.contains(&Effect::PayPoolPerOtherEvent),
            card,
        ) {
            if st.seats[seat.index()].pool < st.other_events(id) as i32 {
                return None;
            }
        }
        if let (Some(id), Some(host)) = (card, u.host) {
            let c = &st.cards[id.index()];
            // "Cannot use equipment during the resulting combat" (Form of
            // the Bat).
            if (cards::is_equipment_in_play(c.card) || cards::becomes_weapon(c.card))
                && self.combat.as_ref().is_some_and(|cb| {
                    cb.side_of_minion(host)
                        .is_some_and(|i| cb.sides[i].equipment_barred())
                })
            {
                return None;
            }
            // Anachronism: no ranged weapon used.
            if st.ranged_barred(host, c.card) {
                return None;
            }
        }
        // Effect-derived combat legality (a dodge under a grapple).
        if let Some((cb, i)) = self
            .combat
            .as_ref()
            .zip(u.actor)
            .and_then(|(cb, m)| cb.side_of_minion(m).map(|i| (cb, i)))
        {
            if !cb.allows_from(st, ability, i, card) {
                return None;
            }
        }
        // A granted action ("this vampire can bleed as a Ⓓ action", Codex
        // of the Edenic Groundskeepers): declared in the minion phase by
        // its actor, ready and unlocked; one bleed a turn, the same copy
        // once a turn (rules-qa 1.4).
        if let (Some(kind), Some(m)) = (declared_kind(ability), u.actor) {
            let action = source.action_id(st, ai);
            if !self.may_take_granted(seat, m, kind, action) {
                return None;
            }
            // One political action per minion per turn (rulebook
            // "Political Action"); not one Delaying Tactics canceled.
            if kind == ActionKind::Political
                && (st.minions[m.index()].political_this_turn
                    || st.seats[seat.index()].forbidden_political.contains(&action))
            {
                return None;
            }
            // No action a Gehenna event provides for Karen (LSJ 20100121).
            if card.is_some_and(|id| {
                LIBRARY[st.cards[id.index()].card as usize]
                    .labels
                    .contains(&Label::Gehenna)
            }) && st.gehenna_immune(m)
            {
                return None;
            }
            if ability.effects.iter().any(|e| match e {
                Effect::CallReferendum(r) => !r.feasible(st, seat, m),
                _ => false,
            }) {
                return None;
            }
        }
        ability
            .requires
            .iter()
            .all(|&r| self.ability_holds(r, seat, source, ability.actor, u))
            .then_some(u)
    }

    /// Does requirement `r` of an in-play ability hold for its user `u`:
    /// read by name where only the engine knows it, then by the card's,
    /// the host's and the play's evaluators?
    fn ability_holds(
        &self,
        r: Requirement,
        seat: SeatId,
        source: AbilitySource,
        actor: Actor,
        u: AbilityUser,
    ) -> bool {
        use Requirement as R;
        let st = &self.state;
        let engine = match (r, source) {
            // "… you control can …" (Alamut, Dead Pool, The Gate of
            // Acheron): the card's controller's only.
            (R::YouControlThisCard, AbilitySource::Card(id)) => st.card_controller(id) == seat,
            // "… controlled by another Methuselah can …" (The Rack, The
            // Uncoiling).
            (R::OthersControlThisCard, AbilitySource::Card(id)) => st.card_controller(id) != seat,
            (R::CardControllersTurn, AbilitySource::Card(id)) => {
                st.card_controller(id) == st.active
            }
            // "This vampire can …" (Haven Uncovered); Melange's "this
            // reacting vampire".
            (R::ActorIsHost | R::ActingIsHost, AbilitySource::Card(id)) => {
                u.actor.is_some() && st.cards[id.index()].host == u.actor
            }
            (R::NotHost, AbilitySource::Card(id)) => {
                u.actor.is_none() || st.cards[id.index()].host != u.actor
            }
            // "The chosen Assamite can …" (Priority Contract).
            (R::ActorIsChosen, AbilitySource::Card(id)) => {
                u.actor.is_some() && st.cards[id.index()].chosen.first() == u.actor.as_ref()
            }
            // Oppugnant Night: the Blood Brothers of its player's circle.
            (R::ActorOfByCircle, AbilitySource::Card(id)) => st.cards[id.index()]
                .by
                .is_some_and(|b| u.actor.is_some_and(|m| b == m || st.same_circle(b, m))),
            (R::ActingIsBy, AbilitySource::Card(id)) => {
                u.actor.is_some() && st.cards[id.index()].by == u.actor
            }
            // Melange: bleeding the controller of the minion the card is on.
            (R::BleedAgainstHostController, AbilitySource::Card(id)) => {
                self.action.as_ref().is_some_and(|a| {
                    a.bleed_target.is_some()
                        && st.cards[id.index()]
                            .host
                            .map(|h| st.minions[h.index()].owner)
                            == a.bleed_target
                })
            }
            // "If the opposing vampire is burned during this weapon's
            // strike resolution" (Sword of the Archangel).
            (R::WeaponBurnedOpponent, AbilitySource::Card(id)) => self
                .combat
                .as_ref()
                .is_some_and(|cb| cb.burned_by.contains(&id)),
            (R::SourceHeldWhenBlocking, AbilitySource::Card(id)) => self
                .combat
                .as_ref()
                .is_some_and(|cb| cb.held_at_block.contains(&id)),
            // The host's own block attempt (Larry, Angelica); a combat it
            // is not in (Huldu, Saulot's Guiding Wisdom: "another minion").
            (R::HostAttempting, _) => self.action.as_ref().is_some_and(|a| {
                u.host
                    .is_some_and(|h| a.state == crate::actions::ActionState::Attempt(h))
            }),
            (R::HostOutOfCombat, _) => self
                .combat
                .as_ref()
                .is_none_or(|cb| u.host.is_none_or(|h| cb.side_of_minion(h).is_none())),
            // "Unlock and attempt to block" (Under Siege): a vampire that
            // could — or the one attempting (ANK 20181122-2).
            (R::CanAttemptBlock, _) => u.actor.is_some_and(|m| {
                self.action
                    .as_ref()
                    .is_some_and(|a| a.state == crate::actions::ActionState::Attempt(m))
                    || self.can_attempt_block(seat, m)
            }),
            _ => true,
        };
        // An `ActingCandidate` ability reads its card for `CardUnlocked`
        // only.
        let card = match source {
            AbilitySource::Card(_) if actor == Actor::ActingCandidate && r != R::CardUnlocked => {
                None
            }
            AbilitySource::Card(id) => Some(id),
            AbilitySource::Vampire(_) => None,
        };
        engine
            && card.is_none_or(|id| cards::holds_for_card(r, st, id, self.action.as_ref()))
            && u.host.is_none_or(|h| cards::holds_for_host(r, st, h))
            && self.holds_live(r, seat, u.actor)
    }

    /// Has `ability` of `source` spent its per-copy `limit` ("each
    /// combat", "during an action": once per action per copy and ability,
    /// rules-qa 6.6, 9.6, 42.9)?
    fn limit_spent(&self, limit: Option<Limit>, source: AbilitySource, ability: u8) -> bool {
        limit_slot(limit, source, ability).is_some_and(|(ledger, key, uses)| {
            self.ledger(ledger)
                .is_some_and(|used| used.iter().filter(|&&k| k == key).count() >= uses)
        })
    }

    /// Count a use of `ability` of `source` against its per-copy `limit`.
    fn mark_limit(&mut self, limit: Option<Limit>, source: AbilitySource, ability: u8) {
        if let Some((ledger, key, _)) = limit_slot(limit, source, ability) {
            if let Some(used) = self.ledger_mut(ledger) {
                used.push(key);
            }
        }
    }

    /// The uses recorded in `ledger`, if it is open (a combat, an action,
    /// a referendum underway; the turn's always).
    fn ledger(&self, ledger: Ledger) -> Option<&Vec<(AbilitySource, u8)>> {
        match ledger {
            Ledger::Combat => self.combat.as_ref().map(|c| &c.abilities_used),
            Ledger::Round => self.combat.as_ref().map(|c| &c.abilities_used_round),
            Ledger::Action => self.action.as_ref().map(|a| &a.abilities_used),
            Ledger::Referendum => self.referendum.as_ref().map(|r| &r.abilities_used),
            Ledger::Turn => Some(&self.state.turn_uses),
        }
    }

    fn ledger_mut(&mut self, ledger: Ledger) -> Option<&mut Vec<(AbilitySource, u8)>> {
        match ledger {
            Ledger::Combat => self.combat.as_mut().map(|c| &mut c.abilities_used),
            Ledger::Round => self.combat.as_mut().map(|c| &mut c.abilities_used_round),
            Ledger::Action => self.action.as_mut().map(|a| &mut a.abilities_used),
            Ledger::Referendum => self.referendum.as_mut().map(|r| &mut r.abilities_used),
            Ledger::Turn => Some(&mut self.state.turn_uses),
        }
    }

    /// The card in its as-played window, if one is (the top of the stack):
    /// what an ability's "played card" predicates read (Diana Iadanza's
    /// "cancel a reaction card as it is played").
    pub(crate) fn as_played_top(&self) -> Option<&PlayCtx> {
        match &self.flow {
            Flow::AsPlayed { stack, .. } => stack.last().map(|p| &p.ctx),
            _ => None,
        }
    }

    /// `cards::holds` for `seat` and `actor` in the live action, combat and
    /// referendum, the card in its as-played window if any.
    fn holds_live(&self, r: Requirement, seat: SeatId, actor: Option<MinionId>) -> bool {
        cards::holds(
            r,
            &self.state,
            seat,
            actor,
            self.as_played_top(),
            self.action.as_ref(),
            self.combat.as_ref(),
            self.referendum.as_ref(),
            false,
        )
    }

    /// The targets an ability picks from (no target spec: one untargeted
    /// use).
    fn ability_targets(
        &self,
        seat: SeatId,
        spec: Option<TargetSpec>,
        host: Option<MinionId>,
    ) -> Vec<Target> {
        self.ability_targets_by(seat, spec, host, None)
    }

    fn ability_targets_by(
        &self,
        seat: SeatId,
        spec: Option<TargetSpec>,
        host: Option<MinionId>,
        actor: Option<MinionId>,
    ) -> Vec<Target> {
        let st = &self.state;
        match spec {
            None => vec![Target::None],
            Some(
                spec @ TargetSpec {
                    kind: TargetKind::Uncontrolled,
                    ..
                },
            ) => (0..st.seats[seat.index()].uncontrolled.len())
                .filter(|&i| self.uncontrolled_target_qualifies(seat, spec, host, i))
                .map(|i| Target::Uncontrolled(i as u8))
                .collect(),
            Some(
                spec @ TargetSpec {
                    kind: TargetKind::Card,
                    ..
                },
            ) => (0..st.cards.len())
                .map(|i| CardId(i as u32))
                .filter(|&id| self.card_target_qualifies(seat, spec, host, id))
                .map(Target::Card)
                .collect(),
            // A Methuselah ("look at any Methuselah's hand", SchreckNET).
            Some(
                spec @ TargetSpec {
                    kind: TargetKind::Seat,
                    ..
                },
            ) => st
                .clockwise_from(seat)
                .into_iter()
                .filter(|&t| {
                    spec.filter
                        .iter()
                        .all(|&r| cards::holds_for_seat(r, st, seat, self.action.as_ref(), t))
                        && !st.seats[t.index()].ousted
                })
                .map(Target::Seat)
                .collect(),
            Some(spec) => (0..st.minions.len())
                .map(|i| MinionId(i as u32))
                .filter(|&id| self.minion_target_qualifies_by(seat, spec, host, actor, id))
                // In combat with damage pending (engine-checked).
                .filter(|&id| {
                    let pending = || {
                        self.combat
                            .as_ref()
                            .and_then(|c| c.side_of_minion(id).map(|t| c.sides[t].pending))
                    };
                    (!spec.filter.contains(&Requirement::TargetInCombatWithDamage)
                        || pending().is_some_and(|d| d.total() > 0))
                        && (!spec
                            .filter
                            .contains(&Requirement::TargetInCombatWithNormalDamage)
                            || pending().is_some_and(|d| d.normal > 0))
                })
                .map(Target::Minion)
                .collect(),
        }
    }

    /// Does minion `id` qualify as the target of an ability of `host`'s?
    /// Read again at resolution for a granted action's target (spec
    /// §3.4.1).
    pub(crate) fn minion_target_qualifies(
        &self,
        seat: SeatId,
        spec: TargetSpec,
        host: Option<MinionId>,
        id: MinionId,
    ) -> bool {
        self.minion_target_qualifies_by(seat, spec, host, None, id)
    }

    /// As `minion_target_qualifies`, `actor` the minion using the ability
    /// out of an action (the acting candidate of a granted action: Cave of
    /// Apples' "younger vampire").
    pub(crate) fn minion_target_qualifies_by(
        &self,
        seat: SeatId,
        spec: TargetSpec,
        host: Option<MinionId>,
        actor: Option<MinionId>,
        id: MinionId,
    ) -> bool {
        let st = &self.state;
        st.minions[id.index()].present()
            && !(spec.filter.contains(&cards::Requirement::NotHost) && Some(id) == host)
            && !(spec.filter.contains(&cards::Requirement::TargetIsHost) && Some(id) != host)
            && spec.filter.iter().all(|&r| {
                cards::holds_for_minion(
                    r,
                    st,
                    seat,
                    self.referendum.as_ref(),
                    self.action.as_ref().map(|a| a.acting).or(actor).or(host),
                    id,
                    self.action.as_ref(),
                )
            })
    }

    /// Does the vampire in slot `i` of the seat's uncontrolled region
    /// qualify as the target of `actor`'s card or ability? Read again at
    /// resolution.
    pub(crate) fn uncontrolled_target_qualifies(
        &self,
        seat: SeatId,
        spec: TargetSpec,
        actor: Option<MinionId>,
        i: usize,
    ) -> bool {
        // The minion playing or using the card ("younger" than the Street
        // Cred player, the blocker too), else the acting one.
        let acting = actor.or(self.action.as_ref().map(|a| a.acting));
        i < self.state.seats[seat.index()].uncontrolled.len()
            && spec
                .filter
                .iter()
                .all(|&r| cards::holds_for_uncontrolled(r, &self.state, seat, acting, i))
    }

    /// Does card `id` qualify as the target of an ability of `host`'s: in
    /// play, on a minion in play if on one? Read again at resolution.
    pub(crate) fn card_target_qualifies(
        &self,
        seat: SeatId,
        spec: TargetSpec,
        host: Option<MinionId>,
        id: CardId,
    ) -> bool {
        let st = &self.state;
        let c = &st.cards[id.index()];
        // Equipment on the host's opponent in combat (Heroic Might's
        // strike).
        let opposing = !spec
            .filter
            .contains(&Requirement::TargetOnOpposingCombatant)
            || self
                .combat
                .as_ref()
                .zip(host)
                .is_some_and(|(cb, h)| cb.opponent_of(h).is_some_and(|o| c.host == Some(o)));
        // A retainer in combat with damage pending (Martyr's Resilience).
        let damaged = !spec.filter.contains(&Requirement::TargetInCombatWithDamage)
            || self
                .combat
                .as_ref()
                .is_some_and(|cb| cb.retainer_pending.iter().any(|&(r, n)| r == id && n > 0));
        opposing
            && damaged
            && c.present()
            && c.host.is_none_or(|h| st.minions[h.index()].present())
            && spec
                .filter
                .iter()
                .all(|&r| cards::holds_for_card_target(r, st, seat, host, id))
    }

    /// Abilities printed on the seat's vampires (`cards::vampires`) usable
    /// in `window`: a vampire's own in its combat (`Actor::Host`), or, in
    /// the combat or out of it, in torpor included (LSJ 20011022), for its
    /// Methuselah's minion in combat (`Actor::ForOwnCombatant`, Opikun).
    fn vampire_ability_options(&self, seat: SeatId, window: Window, out: &mut Vec<Choice>) {
        let st = &self.state;
        let cb = self.combat.as_ref();
        // Its controller's prey's abilities on a vampire (Keith Moody): the
        // vampires of the user's predator.
        let predator = st.predator(seat);
        let theirs = st
            .minions_of(predator)
            .filter(|_| predator != seat)
            .map(|m| (m, true));
        for (m, of_predator) in st.minions_of(seat).map(|m| (m, false)).chain(theirs) {
            let Some(def) = cards::vampire_of(&st.minions[m.index()]) else {
                continue;
            };
            let source = AbilitySource::Vampire(m);
            for (ai, ability) in def.abilities.iter().enumerate() {
                if !ability.windows.contains(&window)
                    || (ability.actor == Actor::HostControllersPrey) != of_predator
                {
                    continue;
                }
                // Where the vampire is: in its combat, or out of one
                // (Randall), for a combatant of its Methuselah's.
                let placed = match (ability.actor, cb) {
                    (Actor::Host, Some(cb)) => cb.side_of_minion(m).is_some(),
                    (Actor::ForOwnCombatant, Some(cb)) => cb.side_of(seat).is_some(),
                    (Actor::HostOutOfCombat, Some(cb)) => cb.side_of_minion(m).is_none(),
                    (Actor::ForOwnCombatant | Actor::HostOutOfCombat, None) => false,
                    (Actor::Host | Actor::HostControllersPrey, _) => true,
                    (other, _) => unreachable!("not a vampire ability actor: {other:?}"),
                };
                if !placed
                    || self
                        .ability_gate(seat, window, source, ai as u8, None)
                        .is_none()
                {
                    continue;
                }
                // "After a combat involving <him> ends … if the opposing
                // minion is not ready" (Ian Carfax): the combat just over.
                if ability
                    .requires
                    .contains(&Requirement::HostCombatOpponentNotReady)
                    && !self
                        .action
                        .as_ref()
                        .and_then(|a| a.last_combat)
                        .is_some_and(|pair| {
                            pair.contains(&m)
                                && pair
                                    .iter()
                                    .any(|&o| o != m && !st.minions[o.index()].ready())
                        })
                {
                    continue;
                }
                // The vampire's printed action ("Sakhar can search your
                // library … as a +1 stealth action"), taken by the vampire
                // itself.
                let declares = declared_kind(ability);
                for target in self.ability_targets(seat, ability.target, Some(m)) {
                    if declares.is_some() && !self.may_direct(seat, m, target) {
                        continue;
                    }
                    // A card with nowhere to go (Luciano Carvalho).
                    if ability.terms == Some(Terms::VampireToHoldTarget)
                        && self.vampires_to_hold(seat, target).is_empty()
                    {
                        continue;
                    }
                    out.push(Choice::UseAbility {
                        source,
                        ability: ai as u8,
                        actor: declares.map(|_| m),
                        target,
                    });
                }
            }
        }
    }

    /// "This combat, combat cards cost the opposing vampire +1 blood"
    /// (Terror Frenzy [ANI]) on `actor`'s combat cards, plus the opposing
    /// combatant's printed toll on its strike cards (`strike_card_toll`).
    pub(crate) fn combat_card_surcharge(&self, actor: MinionId, card: u16, mode: u8) -> u8 {
        if !LIBRARY[card as usize].types.contains(&CardType::Combat) {
            return 0;
        }
        self.combat
            .as_ref()
            .and_then(|c| {
                c.side_of_minion(actor)
                    .map(|i| c.sides[i].combat_card_surcharge)
            })
            .unwrap_or(0)
            + self.strike_card_toll(actor, card, mode)
    }

    /// "In combat, strike cards cost the opposing minion +1 blood or life"
    /// (Djeneba): `actor`'s card played in a strike mode (a dodge
    /// included), printed on the minion in combat with it — an ally's paid
    /// in life (rules-qa 42.2).
    pub(crate) fn strike_card_toll(&self, actor: MinionId, card: u16, mode: u8) -> u8 {
        let Some(c) = self.combat.as_ref() else {
            return 0;
        };
        let Some(i) = c.side_of_minion(actor) else {
            return 0;
        };
        if !is_strike_mode(card, mode) {
            return 0;
        }
        let ctx = crate::state::PrintedCtx {
            opponent: Some(c.sides[1 - i].minion),
            ..Default::default()
        };
        self.state
            .printed_mod(actor, cards::Stat::StrikeCardToll, ctx)
            .max(0) as u8
            + c.sides[i].strike_card_toll
    }

    /// The blood a card with no pool cost to pay (a bleed or political
    /// action card) costs `actor` as it is paid: its cost with X, its
    /// surcharges, less the actor's printed reductions (Roger de Camden).
    pub(crate) fn card_blood(&self, actor: MinionId, play: PlayCtx, x: u8) -> u8 {
        let mut blood = cards::blood_cost(&self.state, play.card, x)
            + self.card_surcharge(actor, play.card, play.mode);
        self.state
            .printed_cost_off_x(actor, play.card, play.mode, x)
            .apply(&mut 0, &mut blood);
        blood
    }

    /// What `actor`'s card costs more (mode `mode`), action cards paid at
    /// resolution included: "cards requiring a Discipline cost this
    /// vampire +n blood" (Enhanced Coagulant become: played using one,
    /// rules-qa 38.4), redirecting a bleed (Narrow Minds), superior
    /// (The Slow Withering), played in torpor (Torpid Blood).
    pub(crate) fn card_surcharge(&self, actor: MinionId, card: u16, mode: u8) -> u8 {
        let st = &self.state;
        let tags = played_disciplines(card, mode);
        let disc = tags.iter().any(|t| cards::discipline_key(t) != "?");
        let superior = tags
            .iter()
            .any(|t| cards::discipline_key(t) != "?" && t.chars().all(|c| c.is_ascii_uppercase()));
        let v = &st.minions[actor.index()];
        let def = cards::lookup(card);
        let effects = def.map_or(&[][..], |d| d.modes[mode as usize].effects);
        st.in_play_layers()
            .map(|l| match l.modifier {
                cards::Modifier::HostDisciplineCardsCost(n)
                    if disc && st.cards[l.source.index()].host == Some(actor) =>
                {
                    n
                }
                // Narrow Minds.
                cards::Modifier::RedirectCardsCost(n)
                    if effects.contains(&Effect::RedirectBleed) =>
                {
                    n
                }
                // The Slow Withering, paid with the normal cost (ANK
                // 20190625).
                cards::Modifier::SuperiorCardsCost(n)
                    if superior && !v.diablerie_shield && !st.gehenna_immune(actor) =>
                {
                    n
                }
                // Torpid Blood: an action card played in torpor.
                cards::Modifier::TorpidBlood
                    if v.in_torpor
                        && effects
                            .iter()
                            .any(|e| matches!(e, Effect::DeclareAction(_))) =>
                {
                    1
                }
                _ => 0,
            })
            .sum::<u8>()
            // Printed on the vampire (Scout Youngwood's reactions).
            + st
                .printed_mod(
                    actor,
                    cards::Stat::BloodCostMore,
                    crate::state::PrintedCtx {
                        card: Some((card, mode)),
                        // Not the acting minion (no action at all
                        // included): played as a reaction.
                        reacting: self.action.as_ref().is_none_or(|a| a.acting != actor),
                        ..Default::default()
                    },
                )
                .max(0) as u8
    }

    /// Blood a card requiring one of some Disciplines costs `actor` more
    /// while a minion bearing the surcharge is acting, attempting to block
    /// or in combat, not the bearer itself (Libertas); a failed block
    /// attempt no longer counts (LSJ 20080818).
    pub(crate) fn discipline_surcharge(&self, actor: MinionId, card: u16, mode: u8) -> u8 {
        let st = &self.state;
        // The Disciplines the card is played using (rules-qa 38.4).
        let discs = played_disciplines(card, mode);
        let own = self.card_surcharge(actor, card, mode);
        st.in_play_layers()
            .filter_map(|l| match l.modifier {
                cards::Modifier::DisciplineSurchargeAround(ds, n) => {
                    Some((st.cards[l.source.index()].host?, ds, n))
                }
                _ => None,
            })
            .filter(|&(b, ds, _)| {
                let busy = self.action.as_ref().is_some_and(|a| {
                    a.acting == b || a.state == crate::actions::ActionState::Attempt(b)
                }) || self
                    .combat
                    .as_ref()
                    .is_some_and(|c| c.side_of_minion(b).is_some());
                // "Requiring" = played using (rules-qa 38.4): the played
                // mode's tags — every "[x][y]" card in the pool is a combo,
                // using both.
                let used = discs
                    .iter()
                    .any(|d| ds.contains(&d.to_ascii_lowercase().as_str()));
                b != actor && busy && used
            })
            .map(|(_, _, n)| n)
            .sum::<u8>()
            + own
    }

    /// "Burn this card after this acting vampire leaves the ready region"
    /// (Slaughtering the Herd [DOM]): the cards `m` played that say so.
    pub(crate) fn by_left_ready(&mut self, m: MinionId) {
        let on = |c: &CardInPlay| c.by == Some(m);
        for (id, r) in self.fired_riders(Hook::ByLeavesReady, &HookAt::on(&on)) {
            for &e in r.effects {
                self.apply_rider_effect(id, e, Hook::ByLeavesReady);
            }
        }
    }

    /// `seat`'s own minion phase, at any time of it: between actions or
    /// during one, its combats included (owner, 2026-09-26: rules-qa 26.5 —
    /// Rutor's Hand unlocking a vampire during another's action).
    fn in_own_minion_phase(&self, seat: SeatId) -> bool {
        self.state.active == seat
            && (self.action.is_some()
                || matches!(self.flow, Flow::MinionPhase)
                || matches!(&self.flow, Flow::PhaseRotation { resume, .. }
                    if matches!(**resume, Flow::MinionPhase)))
    }

    /// Another Methuselah's minion phase, `seat` holding an impulse in it:
    /// during an action (only minion phase actions exist) or the phase's
    /// rotation (rules-qa 7.9).
    fn in_others_minion_phase(&self, seat: SeatId) -> bool {
        self.state.active != seat
            && (self.action.is_some()
                || matches!(&self.flow, Flow::PhaseRotation { resume, .. }
                    if matches!(**resume, Flow::MinionPhase)))
    }

    /// The vampire of `seat`'s taking Enkil Cog's action, its choice of
    /// action under way (`Flow::CogAction`, under any play or terms).
    pub(crate) fn cog_minion(&self, seat: SeatId) -> Option<MinionId> {
        fn walk(flow: &Flow, seat: SeatId) -> Option<MinionId> {
            match flow {
                Flow::CogAction { seat: s, minion } if *s == seat => Some(*minion),
                Flow::PlayTerms { resume, .. }
                | Flow::CostTerms { resume, .. }
                | Flow::AsPlayed { resume, .. }
                | Flow::AbilityTerms { resume, .. } => walk(resume, seat),
                _ => None,
            }
        }
        walk(&self.flow, seat)
    }

    /// May `host` take the action an ability grants it: in its
    /// Methuselah's minion phase, no action under way, ready and unlocked;
    /// one bleed a turn, and the same granted action once a turn (rules-qa
    /// 1.4, spec §3.8.1).
    fn may_take_granted(
        &self,
        seat: SeatId,
        host: MinionId,
        kind: ActionKind,
        id: ActionId,
    ) -> bool {
        let st = &self.state;
        let v = &st.minions[host.index()];
        self.action.is_none()
            && (st.active == seat || self.cog_minion(seat) == Some(host))
            && !v.locked
            && !v.recruited
            && v.ready()
            && !(kind == ActionKind::Bleed && v.bled_this_turn)
            && !v.acted.iter().any(|a| a.same(id))
    }

    /// Use an in-play ability, `seat` the one that chose to. Returns true
    /// when the flow moved on: a granted action was declared, or the user
    /// paid themselves out (`PoolToHostBlood` at 1 pool: ousted, the turn
    /// passed) — callers must not restore their flow then.
    pub(crate) fn use_ability(
        &mut self,
        seat: SeatId,
        source: AbilitySource,
        ability: u8,
        actor: Option<MinionId>,
        target: Target,
        resume: Flow,
    ) -> bool {
        let mode = self.ability_mode(source, ability);
        // Enkil Cog: locked, its vampire takes an action of its
        // Methuselah's choice (rules-qa 49.6).
        if mode.effects.contains(&Effect::TakeAnyAction) {
            let card = source.card();
            self.lock_card(card);
            let minion = self.state.cards[card.index()]
                .host
                .expect("the Cog's vampire");
            self.flow = Flow::CogAction {
                seat: self.state.minions[minion.index()].owner,
                minion,
            };
            return true;
        }
        // A granted action: declared like a card action, the card in play
        // as its source (its referendum and cost are read at resolution).
        if let Some(Effect::DeclareAction(kind)) = mode
            .effects
            .iter()
            .find(|e| matches!(e, Effect::DeclareAction(_)))
        {
            let m = actor.expect("acting minion");
            self.mark_limit(mode.limit, source, ability);
            // "Burn this card to attempt to …" (Hunting the Quarry): burned
            // as the action is declared, not a cost (rules-qa 54.4); its
            // host remembered.
            let burned_host = if mode.effects.contains(&Effect::BurnSourceOnDeclare) {
                let card = source.card();
                let host = self.state.cards[card.index()].host;
                self.burn_card_in_play(card);
                host
            } else {
                None
            };
            // "Can lock this location to … as a … action" (Mobile HQ): a
            // cost, locked even if the action fails (ANK 20221102).
            if mode.effects.contains(&Effect::LockSelf) {
                self.lock_card(source.card());
            }
            self.declare_action(m, *kind, None, None, Some(source));
            // The stealth of a granted action is the card's own ("as a +1
            // stealth political action"), not the rulebook's for political
            // action cards (rules-qa 6.2).
            let stealth = crate::actions::printed_stealth(mode.effects);
            let a = self.action.as_mut().expect("action");
            a.ability = Some(ability);
            a.target = target;
            a.burned_source_host = burned_host;
            a.card_hunt = mode.effects.contains(&Effect::HuntAction);
            if a.card_hunt {
                self.state.minions[m.index()].hunted = true;
            }
            a.mandatory |= mode.effects.contains(&Effect::MandatoryAction);
            // The target decides whom the action is directed at: the
            // blocking seats and the sequencing order follow (spec §3.1.5).
            if let Some(t) = a.directed_seat(&self.state) {
                let seat = a.seat;
                let order = self.action_order(seat, Some(t));
                let a = self.action.as_mut().expect("action");
                a.order = order.clone();
                a.window = crate::window::OrderedWindow::new(order);
            }
            let a = self.action.as_mut().expect("action");
            // Added to the rulebook's default for the kind (none for a
            // granted action, rules-qa 6.2: the printed stealth only).
            a.base_stealth += stealth;
            // The cards exchanged are named with the announcement, before
            // the as-announced window (LSJ 20000222).
            if mode.effects.contains(&Effect::ExchangeHandForAsh) {
                self.flow = Flow::Exchange { ash: None };
            }
            // The Sargon Fragment, Drozodny: the card named with the
            // announcement (LSJ 20010627).
            let of = mode.effects.iter().find_map(|e| match e {
                Effect::AshCardToHandNamed => Some(crate::cards::CardFilter::Any),
                Effect::AshCardOfToHand(of) => Some(*of),
                _ => None,
            });
            if let Some(of) = of {
                let seat = self.state.minions[m.index()].owner;
                self.interrupts.push(Interrupt::NameOwnAshCard { seat, of });
            }
            // Malgorzata: the vampire to remove named with the announcement.
            if let (true, Target::Uncontrolled(t)) = (
                mode.effects
                    .iter()
                    .any(|e| matches!(e, Effect::RemoveUncontrolledForBlood(_))),
                target,
            ) {
                let seat = self.state.minions[m.index()].owner;
                self.interrupts
                    .push(Interrupt::NameUncontrolled { seat, target: t });
            }
            // Lázár Dobrescu: the two uncontrolled vampires named with the
            // announcement.
            if let (true, Target::Seat(of)) =
                (mode.effects.contains(&Effect::PreySlotToOwnSlot), target)
            {
                let seat = self.state.minions[m.index()].owner;
                self.interrupts.push(Interrupt::NameSlotPair {
                    seat,
                    of,
                    from: None,
                });
            }
            // Giovanni del Georgio: the pair named with the announcement
            // (removed at resolution, the action's cost).
            if mode.terms == Some(Terms::PreyAshPair) {
                let seat = self.state.minions[m.index()].owner;
                let of = self.state.prey(seat);
                self.interrupts.push(Interrupt::NameAshPair { seat, of });
            }
            // The ash heap's ally too (Hel-Blá, rules-qa 41.6).
            if let Some(of) = mode.effects.iter().find_map(|e| match e {
                Effect::AllyFromAshLocked(of) => Some(*of),
                _ => None,
            }) {
                let seat = self.state.minions[m.index()].owner;
                self.interrupts.push(Interrupt::AllyFromAsh {
                    seat,
                    of,
                    named: true,
                });
            }
            return true;
        }
        // Ilonka looks at the acting minion's controller's hand, then
        // chooses whether to discard at random: the look comes before
        // the count.
        if let (AbilitySource::Vampire(m), Some(a)) = (source, self.action.as_ref()) {
            if mode
                .effects
                .contains(&Effect::ActingControllerDiscardsAtRandom)
            {
                let viewer = self.state.minions[m.index()].owner;
                self.state.peeks.push((viewer, a.seat));
            }
        }
        // Terms announced with the use (the master cards to discard, a
        // number of cards): the ability resolves once they are chosen.
        if let Some(
            Terms::Discard { .. }
            | Terms::DiscardAny { .. }
            | Terms::Count { .. }
            | Terms::LifeFromTarget
            | Terms::AshCard
            | Terms::VampireToHoldTarget
            | Terms::XPool
            | Terms::XCounters
            | Terms::DirectedHandCard,
        ) = mode.terms
        {
            self.flow = Flow::AbilityTerms {
                source,
                ability,
                target,
                seat,
                user: actor,
                picked: Vec::new(),
                resume: Box::new(resume),
            };
            return true;
        }
        self.resolve_ability(source, ability, target, 0, seat, actor)
    }

    /// Is stealth needed on the action `m` performs (rules-qa 3.2)?
    fn stealth_needed(&self, seat: SeatId, m: MinionId) -> bool {
        cards::holds(
            cards::Requirement::StealthNeeded,
            &self.state,
            seat,
            Some(m),
            None,
            self.action.as_ref(),
            None,
            None,
            false,
        )
    }

    /// Who uses an ability of `source`: `seat` the Methuselah asked, `user`
    /// the minion using it where the ability does not say (any ready
    /// vampire, the bearer's opponent, a granted action's candidate); read
    /// alike when offered, at terms and at resolution.
    pub(crate) fn ability_user(
        &self,
        source: AbilitySource,
        ability: u8,
        seat: SeatId,
        user: Option<MinionId>,
    ) -> AbilityUser {
        let st = &self.state;
        let mode = self.ability_mode(source, ability);
        let owner = |m: MinionId| st.minions[m.index()].owner;
        let (seat, host) = match source {
            // A vampire's own text: the vampire is its host.
            AbilitySource::Vampire(m) if mode.actor == Actor::HostControllersPrey => {
                (st.prey(owner(m)), Some(m))
            }
            AbilitySource::Vampire(m) => (owner(m), Some(m)),
            AbilitySource::Card(id) => {
                let c = &st.cards[id.index()];
                match mode.actor {
                    Actor::Controller => (st.card_controller(id), None),
                    Actor::ChosenController => (owner(c.chosen[0]), None),
                    // The vampire using it pays, as its host would; the
                    // controller of the vampire using it is the user, whoever
                    // controls the card (Carver's Meat Packing and Storage).
                    Actor::AnyReadyVampire | Actor::OpposingCombatant => {
                        let m = user.expect("the vampire using it");
                        (owner(m), Some(m))
                    }
                    Actor::ActingMinion => {
                        let acting = self.action.as_ref().expect("action").acting;
                        (owner(acting), Some(acting))
                    }
                    // "Methuselahs can …": the one asked, its requirements
                    // read for them (Ventrue Headquarters: "you control").
                    Actor::Methuselah => (seat, None),
                    // The minion declaring the action it grants.
                    Actor::ActingCandidate => (owner(user.expect("the actor")), c.bearer()),
                    // Unhosted (Dance of the Dead): its controller's.
                    Actor::ForOwnCombatant if c.bearer().is_none() => {
                        (st.card_controller(id), None)
                    }
                    _ => {
                        let host = c.bearer().expect("host");
                        (owner(host), Some(host))
                    }
                }
            }
        };
        // Combat effects apply to the host, or to the user's minion in combat
        // for an ability used from out of the combat (Opikun).
        let actor = match (mode.actor, self.combat.as_ref()) {
            (Actor::ForOwnCombatant, Some(cb)) => cb.side_of(seat).map(|i| cb.sides[i].minion),
            (Actor::ActingCandidate, _) => user,
            _ => host,
        };
        AbilityUser { seat, host, actor }
    }

    /// Would `seat` using `choice` gain nothing, every effect (costs aside)
    /// scaling with something zero now (Powerbase: Madrid's counters,
    /// rules-qa 73.3; an abstaining vampire's votes, 73.4)? Still offered
    /// (rules-qa 17.4): for an interface hiding such options.
    pub fn gains_nothing(&self, seat: SeatId, choice: &Choice) -> bool {
        let Choice::UseAbility {
            source,
            ability,
            actor,
            target,
        } = *choice
        else {
            return false;
        };
        let st = &self.state;
        let mode = self.ability_mode(source, ability);
        let voter = |m: Option<MinionId>| {
            self.referendum
                .as_ref()
                .is_some_and(|r| m.is_some_and(|m| !r.may_vote(st, m)))
        };
        // Read only for a vote: who uses it then.
        let user = || self.ability_user(source, ability, seat, actor).actor;
        let target_minion = match target {
            Target::Minion(m) => Some(m),
            _ => None,
        };
        let no_counters =
            matches!(source, AbilitySource::Card(id) if st.cards[id.index()].counters == 0);
        let mut nil = false;
        for e in mode.effects {
            match e {
                Effect::DeclareAction(_)
                | Effect::LockSelf
                | Effect::RemoveCounter(_)
                | Effect::RemoveCountersX => {}
                Effect::RemoveAllSourceCounters if no_counters => nil = true,
                Effect::AddVotes(_) | Effect::AddVotesX if voter(user()) => nil = true,
                Effect::TargetVotesPerCounter if voter(target_minion) => nil = true,
                _ => return false,
            }
        }
        nil
    }

    /// The definition of an ability, wherever it is printed.
    fn ability_mode(&self, source: AbilitySource, ability: u8) -> &'static cards::ModeDef {
        source.mode(&self.state, ability)
    }

    /// Terms of an ability: the next master card to discard, among those
    /// not picked yet.
    pub(crate) fn ability_terms_request(
        &self,
        source: AbilitySource,
        ability: u8,
        target: Target,
        seat: SeatId,
        user: Option<MinionId>,
        picked: &[u8],
    ) -> DecisionRequest {
        let seat = self.ability_user(source, ability, seat, user).seat;
        let options = match self.ability_mode(source, ability).terms {
            Some(Terms::Discard { of, .. } | Terms::DiscardAny { of }) => {
                let any = matches!(
                    self.ability_mode(source, ability).terms,
                    Some(Terms::DiscardAny { .. })
                );
                (any && !picked.is_empty())
                    .then_some(Choice::Pass)
                    .into_iter()
                    .chain(
                        self.state
                            .hand_picks(seat, of)
                            .into_iter()
                            .filter(|i| !picked.contains(i))
                            // One per card: the first copy not picked yet.
                            .filter(|&i| {
                                let hand = &self.state.seats[seat.index()].hand;
                                !(0..i).any(|j| {
                                    !picked.contains(&j) && hand[j as usize] == hand[i as usize]
                                })
                            })
                            .map(Choice::Discard),
                    )
                    .collect()
            }
            Some(Terms::Count { min, max }) => (min..=max).map(Choice::SetX).collect(),
            Some(Terms::XPool) => {
                let pool = self.state.seats[seat.index()].pool.clamp(0, u8::MAX as i32) as u8;
                (1..=pool).map(Choice::SetX).collect()
            }
            Some(Terms::XCounters) => {
                let n = self.state.cards[source.card().index()].counters;
                (1..=n).map(Choice::SetX).collect()
            }
            Some(Terms::LifeFromTarget) => {
                let st = &self.state;
                let host = match source {
                    AbilitySource::Card(c) => st.cards[c.index()].bearer(),
                    AbilitySource::Vampire(m) => Some(m),
                }
                .expect("host");
                let t = target.minion();
                let room = st
                    .ally_body(host)
                    .map_or(0, |b| b.life.saturating_sub(st.minions[host.index()].blood));
                let max = room.min(st.minions[t.index()].blood);
                (1..=max).map(Choice::SetX).collect()
            }
            // One choice per card name: copies are alike.
            Some(Terms::AshCard) => distinct(&self.state.seats[seat.index()].ash, |_| true)
                .into_iter()
                .map(Choice::FindCard)
                .collect(),
            Some(Terms::DirectedHandCard) => {
                let t = self
                    .action
                    .as_ref()
                    .and_then(|a| a.directed_seat(&self.state))
                    .expect("directed action");
                distinct(&self.state.seats[t.index()].hand, |_| true)
                    .into_iter()
                    .map(Choice::FindCard)
                    .collect()
            }
            Some(Terms::VampireToHoldTarget) => self
                .vampires_to_hold(seat, target)
                .into_iter()
                .map(Choice::ChooseMinion)
                .collect(),
            _ => unreachable!("ability terms without a choice"),
        };
        DecisionRequest::new(seat, WindowLabel::PlayTerms, options)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ability_terms_apply(
        &mut self,
        source: AbilitySource,
        ability: u8,
        target: Target,
        seat: SeatId,
        user: Option<MinionId>,
        mut picked: Vec<u8>,
        resume: Box<Flow>,
        pick: u8,
    ) {
        picked.push(pick);
        let n = match self.ability_mode(source, ability).terms {
            Some(Terms::Discard { n, .. }) => n,
            Some(Terms::DiscardAny { .. }) => u8::MAX,
            _ => unreachable!("ability terms without discards"),
        };
        if picked.len() < n as usize {
            self.flow = Flow::AbilityTerms {
                source,
                ability,
                target,
                seat,
                user,
                picked,
                resume,
            };
            return;
        }
        self.ability_discards_done(source, ability, target, seat, user, picked, resume);
    }

    /// The discards picked for an ability's terms paid; then it resolves —
    /// X the number discarded for "discard X" (Angelica).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ability_discards_done(
        &mut self,
        source: AbilitySource,
        ability: u8,
        target: Target,
        seat: SeatId,
        user: Option<MinionId>,
        mut picked: Vec<u8>,
        resume: Box<Flow>,
    ) {
        // The cost: discards to the ash heap, replaced as any card leaving
        // the hand — or the cards put on the card (Fleshforge Chamber).
        let seat = self.ability_user(source, ability, seat, user).seat;
        picked.sort_unstable_by(|a, b| b.cmp(a));
        let discarded = picked.len();
        // The cards' sameness, read by the effect (Babalawo Alafin).
        let hand = &self.state.seats[seat.index()].hand;
        let same = picked
            .windows(2)
            .all(|w| hand[w[0] as usize] == hand[w[1] as usize]);
        let store = self
            .ability_mode(source, ability)
            .effects
            .contains(&Effect::StorePickedFromHand);
        for i in picked {
            let s = &mut self.state.seats[seat.index()];
            let (c, copy) = s.hand.take(i as usize);
            if store {
                self.state.cards[source.card().index()].stored.put(c, copy);
            } else {
                s.ash.put(c, copy);
            }
        }
        // Put on the Chamber, the card is replaced (owner, 2026-09-26:
        // rules-qa 37.1) — as a discard is.
        self.replace(seat, discarded, None);
        self.flow = *resume;
        let x = match self.ability_mode(source, ability).terms {
            Some(Terms::DiscardAny { .. }) => discarded as u8,
            _ => same as u8,
        };
        self.resolve_ability(source, ability, target, x, seat, user);
    }

    /// The vampires of `seat` the target card could move onto: another than
    /// the one it is on, in torpor or not, able to hold it (Luciano
    /// Carvalho).
    pub(crate) fn vampires_to_hold(&self, seat: SeatId, target: Target) -> Vec<MinionId> {
        let st = &self.state;
        let c = &st.cards[target.card().index()];
        st.minions_of(seat)
            .filter(|&m| {
                st.minions[m.index()].is_vampire()
                    && c.host != Some(m)
                    && st.can_hold(m, c.card, &[])
            })
            .collect()
    }

    /// The vampire chosen by an ability's terms (`VampireToHoldTarget`):
    /// the target card moves onto it; then the ability resolves.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ability_minion_apply(
        &mut self,
        source: AbilitySource,
        ability: u8,
        target: Target,
        seat: SeatId,
        user: Option<MinionId>,
        resume: Flow,
        m: MinionId,
    ) {
        self.state.cards[target.card().index()].host = Some(m);
        self.flow = resume;
        self.resolve_ability(source, ability, target, 0, seat, user);
    }

    /// The card chosen by name for an ability's terms: from the ash heap to
    /// the bottom of the library (Mora, the Death Seer), or discarded from
    /// the hand of the Methuselah the action is directed at and replaced
    /// (Richard de Worde); then the ability resolves.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ability_card_apply(
        &mut self,
        source: AbilitySource,
        ability: u8,
        target: Target,
        seat: SeatId,
        user: Option<MinionId>,
        resume: Flow,
        card: u16,
    ) {
        match self.ability_mode(source, ability).terms {
            Some(Terms::AshCard) => {
                let seat = self.ability_user(source, ability, seat, user).seat;
                let s = &mut self.state.seats[seat.index()];
                let i = s
                    .ash
                    .iter()
                    .position(|&c| c == card)
                    .expect("card in the ash heap");
                let (card, copy) = s.ash.take(i);
                s.library.put_at(0, card, copy);
            }
            Some(Terms::DirectedHandCard) => {
                let t = self
                    .action
                    .as_ref()
                    .and_then(|a| a.directed_seat(&self.state))
                    .expect("directed action");
                let s = &mut self.state.seats[t.index()];
                let i = s
                    .hand
                    .iter()
                    .position(|&c| c == card)
                    .expect("card in hand");
                let (_, copy) = s.hand.take(i);
                s.ash.put(card, copy);
                self.replace(t, 1, None);
            }
            other => unreachable!("no card terms: {other:?}"),
        }
        self.flow = resume;
        self.resolve_ability(source, ability, target, 0, seat, user);
    }

    /// The number chosen for an ability's count terms (Heart of
    /// Nizchetus): the ability resolves with it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ability_count_apply(
        &mut self,
        source: AbilitySource,
        ability: u8,
        target: Target,
        seat: SeatId,
        user: Option<MinionId>,
        resume: Flow,
        x: u8,
    ) {
        self.flow = resume;
        self.resolve_ability(source, ability, target, x, seat, user);
    }

    /// Resolve an in-play ability's effects (its terms, if any, chosen:
    /// `x` for a count). Returns true when the flow moved on (the user
    /// ousted).
    fn resolve_ability(
        &mut self,
        source: AbilitySource,
        ability: u8,
        target: Target,
        x: u8,
        seat: SeatId,
        user: Option<MinionId>,
    ) -> bool {
        let mode = self.ability_mode(source, ability);
        let AbilityUser { seat, host, actor } = self.ability_user(source, ability, seat, user);
        // A vampire's own text has no library card (`u16::MAX`: no effect
        // of a vampire reads it).
        let card = match source {
            AbilitySource::Vampire(_) => u16::MAX,
            AbilitySource::Card(id) => self.state.cards[id.index()].card,
        };
        let ctx = PlayCtx::new(seat, card, ability, actor, target, x);
        self.mark_limit(mode.limit, source, ability);
        if mode.windows.iter().any(|w| w.is_phase())
            && !mode.effects.contains(&Effect::SpendMasterPhaseAction)
        {
            self.state
                .phase_uses
                .push(phase_key(source, ability, mode.limit));
        }
        // The flow moved on for a Methuselah other than the user (an
        // oust on their turn).
        let mut moved = false;
        for e in mode.effects {
            match *e {
                Effect::SpendMasterPhaseAction => self.state.spend_mpa(ctx.seat),
                Effect::BurnSelf => self.burn_card_in_play(source.card()),
                Effect::LockSelf => self.lock_card(source.card()),
                Effect::HuntStealthUntilUnlock(n) => {
                    self.state.seats[seat.index()].timed.add(
                        TimedStat::HuntStealth,
                        Expiry::UnlockPhase,
                        n as i8,
                    );
                }
                Effect::AddCounter(n) => self.state.cards[source.card().index()].counters += n,
                Effect::BurnSelfAtCounters(n) => {
                    if self.state.cards[source.card().index()].counters >= n {
                        self.burn_card_in_play(source.card());
                    }
                }
                Effect::SlaveTakesCombat => {
                    if let (AbilitySource::Vampire(m), Some(a)) = (source, self.action.as_mut()) {
                        a.slave = Some(m);
                    }
                }
                Effect::SearchMageAlly => {
                    if let AbilitySource::Vampire(m) = source {
                        self.interrupts.push(Interrupt::AllySearch {
                            seat: ctx.seat,
                            minion: m,
                        });
                    }
                }
                Effect::CounterToTarget(clan) => {
                    if let Target::Minion(t) = target {
                        let c = &mut self.state.cards[source.card().index()];
                        if c.counters > 0 {
                            c.counters -= 1;
                            self.state.minions[t.index()].nightmare += 1;
                        }
                        if self.state.cards[source.card().index()].counters == 0 {
                            self.nightmares_end(source.card(), clan);
                        }
                    }
                }
                Effect::BloodPerHostage => {
                    let n = self
                        .state
                        .minions
                        .iter()
                        .filter(|m| m.in_play && m.hostage > 0)
                        .count();
                    if let Target::Minion(t) = target {
                        if self.state.minions[t.index()].present() {
                            self.state.gain_blood(t, n.min(u8::MAX as usize) as u8);
                        }
                    }
                }
                Effect::BurnTargetHostages => {
                    if let Target::Minion(t) = target {
                        self.state.minions[t.index()].hostage = 0;
                    }
                }
                Effect::HostLeavesTorpor => {
                    if let Some(h) = host {
                        if self.state.minions[h.index()].present()
                            && self.state.minions[h.index()].in_torpor
                        {
                            self.leave_torpor(h);
                        }
                    }
                }
                Effect::OwnClanBurnsBlood(clan) => {
                    self.interrupts.push(Interrupt::MinionOf {
                        seat: ctx.seat,
                        of: ctx.seat,
                        purpose: crate::draws::MinionPurpose::BurnBloodOfClan(clan),
                    });
                }
                // An ability's pool cost ("lock this location and burn 1
                // pool"), paid down to 0: its user is ousted, nothing more
                // resolves (Michael Harris's range, its combat gone); the
                // active Methuselah's oust passed the turn.
                Effect::PayPool(n) => {
                    let active = self.state.active;
                    self.pay_pool_for_ability(ctx.seat, n as i32);
                    if self.state.seats[ctx.seat.index()].ousted {
                        return ctx.seat == active;
                    }
                }
                Effect::ExchangeForAnarchCard => {
                    self.interrupts.push(Interrupt::ExchangeAnarch {
                        seat: ctx.seat,
                        ash: None,
                    });
                }
                Effect::UnlockActingIfSuccessful(clan) => {
                    if let Some(a) = self.action.as_mut() {
                        a.unlock_if_successful = Some(clan);
                    }
                }
                Effect::BloodToChosen(n) => {
                    if let Some(&m) = self.state.cards[source.card().index()].chosen.first() {
                        if self.state.minions[m.index()].present() {
                            self.state.gain_blood(m, n);
                        }
                    }
                }
                Effect::GiveToPrey => {
                    let prey = self.state.prey(ctx.seat);
                    self.give_card_control(source.card(), prey);
                }
                Effect::LockPreyVampires(n) => {
                    let prey = self.state.prey(ctx.seat);
                    for _ in 0..n {
                        self.interrupts.push(Interrupt::MinionOf {
                            seat: ctx.seat,
                            of: prey,
                            purpose: crate::draws::MinionPurpose::LockVampire,
                        });
                    }
                }
                Effect::BloodToCard(n) => {
                    let t = target.minion();
                    let moved = n.min(self.state.minions[t.index()].blood);
                    self.state.minions[t.index()].blood -= moved;
                    self.state.cards[source.card().index()].counters += moved;
                }
                Effect::CountersToUncontrolledTarget => {
                    let Target::Uncontrolled(i) = target else {
                        unreachable!("counters to the uncontrolled region without a slot");
                    };
                    let n = std::mem::take(&mut self.state.cards[source.card().index()].counters);
                    self.state.add_uncontrolled_blood(ctx.seat, i as usize, n);
                }
                Effect::DrawCryptThenRemove => {
                    let seat = &mut self.state.seats[ctx.seat.index()];
                    if let Some((card, copy)) = seat.crypt.take_top() {
                        seat.uncontrolled.push(crate::state::UncontrolledSlot {
                            card,
                            copy: crate::copies::Untracked(copy),
                            counters: 0,
                            banished: None,
                        });
                    }
                    self.interrupts
                        .push(crate::draws::Interrupt::RemoveUncontrolled { seat: ctx.seat });
                }
                // Puppet Master [DOM]: unlock the attached vampire, then
                // take control of it.
                Effect::TakeControlOfHost => {
                    if let Some(h) = self.state.cards[source.card().index()].host {
                        if self.state.minions[h.index()].present() {
                            self.effect_unlock(h);
                            self.take_control(h, ctx.seat);
                        }
                    }
                }
                // To its owner's uncontrolled region: the Methuselah it
                // returns to, if held temporarily.
                Effect::TargetToUncontrolled => {
                    let t = target.minion();
                    if self.state.minions[t.index()].present() {
                        let owner = self.state.minions[t.index()].owner;
                        self.banish(t, owner);
                    }
                }
                // Yawp Court: the chosen Sabbat vampire into combat with
                // the acting vampire, which acts first, before the
                // referendum.
                Effect::YawpCourtCombat => {
                    let m = actor.expect("the Sabbat vampire");
                    // The action's cost first (LSJ 20080201), then the
                    // location and the vampire lock.
                    let paid = self.pay_political_cost_now();
                    self.lock_card(source.card());
                    self.state.minions[m.index()].locked = true;
                    if let Some(acting) = self.action.as_ref().map(|a| a.acting).filter(|_| paid) {
                        if self.state.minions[acting.index()].ready()
                            && self.state.minions[m.index()].ready()
                        {
                            self.start_combat(acting, m);
                            self.combat.as_mut().expect("combat").yawp = Some(m);
                            return true;
                        }
                    }
                }
                // The Status Perfectus: the acting minion first, barred
                // from striking the first round.
                Effect::TargetEntersCombatWithActing => {
                    let t = target.minion();
                    if let Some(acting) = self.action.as_ref().map(|a| a.acting) {
                        if self.state.minions[acting.index()].ready()
                            && self.state.minions[t.index()].ready()
                        {
                            self.start_combat_after_resolution(acting, t);
                            self.combat.as_mut().expect("combat").sides[0].no_strike_round =
                                Some(1);
                            return true;
                        }
                    }
                }
                // "Lock after resolution of a successful bleed against you
                // to enter combat with the acting minion" (The Guardian):
                // both still ready, the acting minion acting first.
                Effect::EnterCombatWithActing => {
                    let h = host.expect("host");
                    if let Some(acting) = self.action.as_ref().map(|a| a.acting) {
                        if self.state.minions[acting.index()].ready()
                            && self.state.minions[h.index()].ready()
                        {
                            self.start_combat_after_resolution(acting, h);
                            return true;
                        }
                    }
                }
                Effect::HostBloodToTarget(n) => {
                    let (h, t) = (host.expect("host"), target.minion());
                    let have = self.state.minions[h.index()].blood;
                    let was = self.state.minions[t.index()].blood;
                    let now = self.state.cap_starting(t, was.saturating_add(n.min(have)));
                    self.state.set_blood(t, now);
                    self.state.minions[h.index()].blood -= now - was;
                }
                // Blood above the host's capacity drains off as the engine
                // settles (rules-qa 41.8).
                Effect::TargetBloodToHost(n) => {
                    let (h, t) = (host.expect("host"), target.minion());
                    let give = n.min(self.state.minions[t.index()].blood);
                    let b = &mut self.state.minions[h.index()].blood;
                    *b = b.saturating_add(give);
                    let v = &mut self.state.minions[t.index()];
                    v.blood -= give;
                    if v.is_ally() && v.blood == 0 {
                        self.burn_minion(t);
                    }
                }
                Effect::HandSizeBonusUntilEndOfTurn(n) => {
                    self.state.seats[ctx.seat.index()].timed.add(
                        TimedStat::HandSize,
                        Expiry::EndOfTurn,
                        n as i8,
                    );
                    self.draw_up(ctx.seat);
                }
                Effect::TargetBurnLife(n) => {
                    let t = target.minion();
                    let v = &mut self.state.minions[t.index()];
                    v.blood -= n.min(v.blood);
                    if v.is_ally() && v.blood == 0 {
                        self.burn_minion(t);
                    }
                }
                Effect::HuntingGroundBlood(n) => {
                    let t = target.minion();
                    self.state.gain_blood(t, n);
                    self.state.minions[t.index()].hunting_ground_fed = true;
                }
                Effect::HuntingGroundTwoOldest => {
                    self.interrupts.push(Interrupt::MinionOf {
                        seat: ctx.seat,
                        of: ctx.seat,
                        purpose: crate::draws::MinionPurpose::OldestHuntingGround(None),
                    });
                }
                // Burial Site Hunting Ground: life to a wraith or zombie
                // ally, "not to exceed its starting life".
                Effect::HuntingGroundLife(n) => {
                    let t = target.minion();
                    if let Some(body) = self.state.ally_body(t) {
                        let v = &self.state.minions[t.index()];
                        let b = (v.blood + n).min(body.life.max(v.blood));
                        self.state.set_blood(t, b);
                    }
                }
                Effect::HostBurnBloodToUnlock => {
                    let h = host.expect("host");
                    self.state.minions[h.index()].blood -= 1;
                    self.effect_unlock(h);
                }
                Effect::HostBloodToPool(n) => {
                    let h = &mut self.state.minions[host.expect("host").index()];
                    let moved = n.min(h.blood);
                    h.blood -= moved;
                    self.state.seats[ctx.seat.index()].pool += moved as i32;
                }
                Effect::PoolToHostBlood(n) => {
                    let cap = self.state.capacity_of(host.expect("host"));
                    let h = &mut self.state.minions[host.expect("host").index()];
                    let room = cap.map_or(u8::MAX, |c| c.saturating_sub(h.blood));
                    let moved = n
                        .min(room)
                        .min(self.state.seats[ctx.seat.index()].pool.max(0) as u8);
                    let b = h.blood + moved;
                    self.state.set_blood(host.expect("host"), b);
                    self.state.seats[ctx.seat.index()].pool -= moved as i32;
                    if self.state.seats[ctx.seat.index()].pool == 0 {
                        self.oust_and_pass(ctx.seat);
                    }
                }
                Effect::HostBurnBlood(n) | Effect::HostBurnLife(n) => {
                    let host = host.expect("host");
                    if self.state.burn_blood_or_life(host, n) {
                        self.burn_minion(host);
                    }
                }
                // Removed from the game, revealed (public); the Anarch picked
                // next, before any other decision.
                Effect::RemoveCryptTopAnarch(clan) => {
                    let seat = &mut self.state.seats[ctx.seat.index()];
                    if let Some((card, copy)) = seat.crypt.take_top() {
                        seat.removed_crypt.put(card, copy);
                        if crate::crypt::CRYPT[card as usize].clan == clan {
                            self.interrupts
                                .push(Interrupt::BecomesAnarch { seat: ctx.seat });
                        }
                    }
                }
                // Their last pool ousts them, on their own turn: the turn
                // passes (once the oust applies, with a Boon in some hand).
                Effect::StealPoolFromBoonTarget => {
                    let st = &self.state;
                    let from = st.active;
                    if st.cards[source.card().index()].against.contains(from) {
                        self.state.seats[from.index()].pool -= 1;
                        self.state.seats[ctx.seat.index()].pool += 1;
                        if self.state.seats[from.index()].pool <= 0 {
                            self.oust_and_pass(from);
                            moved |= self.state.seats[from.index()].ousted;
                        }
                    }
                }
                Effect::LockTargetCard => {
                    if let Target::Card(c) = target {
                        self.lock_card(c);
                    }
                }
                Effect::BloodToTargetX => {
                    if let Target::Minion(t) = target {
                        if self.state.minions[t.index()].present() {
                            self.state.gain_blood(t, x);
                        }
                    }
                }
                Effect::AddInterceptX => {
                    if let Some(a) = self.action.as_mut() {
                        a.intercept_given.push((actor.expect("actor"), x as i8));
                    }
                }
                Effect::TargetInterceptThisAction(n) => {
                    if let (Some(a), Target::Minion(t)) = (self.action.as_mut(), target) {
                        a.intercept_given.push((t, n));
                    }
                }
                Effect::LockHost => {
                    self.state.minions[host.expect("host").index()].locked = true;
                }
                Effect::UnlockHost => self.effect_unlock(host.expect("host")),
                Effect::BurnEdge => self.state.seats[ctx.seat.index()].has_edge = false,
                Effect::ToggleLockTarget => {
                    if let Target::Minion(m) = ctx.target {
                        if self.state.minions[m.index()].locked {
                            self.effect_unlock(m);
                        } else {
                            self.state.minions[m.index()].locked = true;
                        }
                    }
                }
                // The revealed card stays on top (Gathii).
                Effect::RevealTopCard { burn, stealth } => {
                    let top = self.state.seats[ctx.seat.index()].library.last().copied();
                    let master = top.is_some_and(|c| {
                        crate::library::LIBRARY[c as usize]
                            .types
                            .contains(&crate::library::CardType::Master)
                    });
                    let h = host.expect("host");
                    if master {
                        self.state.burn_blood(h, burn);
                    } else if top.is_some() && self.stealth_needed(ctx.seat, h) {
                        self.apply_action_effect(Effect::AddStealth(stealth), &ctx);
                    }
                }
                Effect::BurnOwnLife(n) => {
                    let id = source.card();
                    match self.state.cards[id.index()].body {
                        Some(m) => {
                            let v = &mut self.state.minions[m.index()];
                            v.blood = v.blood.saturating_sub(n);
                            if v.blood == 0 {
                                self.burn_minion(m);
                            }
                        }
                        None => {
                            let c = &mut self.state.cards[id.index()];
                            c.counters = c.counters.saturating_sub(n);
                            if c.counters == 0 {
                                self.burn_card_in_play(id);
                            }
                        }
                    }
                }
                Effect::StealBloodToHostX => {
                    if let (Target::Minion(t), Some(h)) = (target, host) {
                        let n = ctx.x.min(self.state.minions[t.index()].blood);
                        self.state.minions[t.index()].blood -= n;
                        self.state.gain_blood(h, n);
                    }
                }
                Effect::ReduceStealth(n) => {
                    self.action.as_mut().expect("action context").stealth_reduce += n;
                }
                Effect::UnlockSelf => self.unlock_card(source.card()),
                Effect::ReduceActionCost(n) => {
                    self.action.as_mut().expect("action context").cost_reduction += n;
                }
                Effect::HostAsIfUnlockedForAuspex => {
                    if let (Some(h), Some(a)) = (host, self.action.as_mut()) {
                        a.aus_unlocked.push(h);
                    }
                }
                Effect::CountersToPool => {
                    let c = source.card();
                    let n = std::mem::take(&mut self.state.cards[c.index()].counters);
                    self.state.seats[ctx.seat.index()].pool += n as i32;
                }
                Effect::CounterToPool => {
                    let c = &mut self.state.cards[source.card().index()];
                    if c.counters > 0 {
                        c.counters -= 1;
                        self.state.seats[ctx.seat.index()].pool += 1;
                    }
                }
                // Donatello Giovanni: the chosen vampire, until his
                // controller's next unlock phase.
                Effect::MarkBleedToll => {
                    if let (Target::Minion(t), Some(h)) = (ctx.target, host) {
                        self.state.bleed_tolls.push((t, h));
                    }
                }
                Effect::HostBleedThisTurn(n) => {
                    if let Some(h) = host {
                        let v = &mut self.state.minions[h.index()];
                        v.bleed_bonus_turn = v.bleed_bonus_turn.saturating_add(n);
                    }
                }
                // The Baron: the hands of those with a ready Giovanni.
                Effect::PeekHandsOfReadyClan(clan) => {
                    for i in 0..self.state.seats.len() {
                        let t = SeatId(i as u8);
                        let has = self.state.minions_of(t).any(|m| {
                            let v = &self.state.minions[m.index()];
                            v.ready() && v.clan() == Some(clan)
                        });
                        if t != ctx.seat && has && !self.state.seats[i].ousted {
                            self.state.peeks.push((ctx.seat, t));
                        }
                    }
                }
                // Babalawo Alafin: the two discarded cards were the same.
                Effect::OthersHandSmallerIfSame(n) => {
                    if ctx.x == 1 {
                        for i in 0..self.state.seats.len() {
                            let seat = &mut self.state.seats[i];
                            if i != ctx.seat.index() && !seat.ousted {
                                seat.timed.add(
                                    TimedStat::HandSize,
                                    Expiry::UnlockPhase,
                                    -(n as i8),
                                );
                            }
                        }
                    }
                }
                // Emerald Legionnaire: a copy from the ash heap, if one is
                // still there.
                Effect::RaiseNamedAllyFromAsh(name) => {
                    let card = self.state.seats[ctx.seat.index()]
                        .ash
                        .iter()
                        .copied()
                        .find(|&c| LIBRARY[c as usize].name == name);
                    if let Some(card) = card {
                        self.raise_ash_ally(ctx.seat, card, false);
                    }
                }
                // Nicomedes: the draws are replacements (Infernal Pursuit's
                // extra cards with each; ANK 20180925-1).
                Effect::ShuffleHandDrawUp => {
                    let seat = &mut self.state.seats[ctx.seat.index()];
                    let hand = std::mem::take(&mut seat.hand);
                    let n = hand.len();
                    seat.library.put_all(hand);
                    self.state.seats[ctx.seat.index()]
                        .library
                        .shuffle(&mut self.rng);
                    self.replace(ctx.seat, n, None);
                }
                Effect::TargetBurnsForInterceptThisAction => {
                    if let (Some(a), Target::Minion(t)) = (self.action.as_mut(), target) {
                        a.burn_for_intercept.push(t);
                    }
                }
                Effect::PreyUncontrolledBloodToPool(n) => {
                    let of = self.state.prey(ctx.seat);
                    for _ in 0..n {
                        self.interrupts
                            .push(Interrupt::PreySlotToPool { seat: ctx.seat, of });
                    }
                }
                // Smear Campaign: to the controller of the card's host,
                // read before the card burns.
                Effect::TargetHostControllerTakesControl => {
                    if let Target::Card(t) = ctx.target {
                        if let Some(h) = self.state.cards[t.index()].host {
                            let to = self.state.minions[h.index()].owner;
                            self.give_card_control(source.card(), to);
                        }
                    }
                }
                Effect::ActingControllerDiscardsAtRandom => {
                    let s = self.action.as_ref().expect("action").seat;
                    let k = (ctx.x as usize).min(self.state.seats[s.index()].hand.len());
                    self.discard_at_random(s, k);
                    self.replace(s, k, None);
                }
                Effect::TargetSeatDiscardsAtRandom(n) => {
                    let Target::Seat(s) = target else {
                        unreachable!("a random discard without a Methuselah");
                    };
                    let k = (n as usize).min(self.state.seats[s.index()].hand.len());
                    self.discard_at_random(s, k);
                    self.replace(s, k, None);
                }
                Effect::SpendDiscardPhaseAction => self.state.seats[ctx.seat.index()].dpa -= 1,
                Effect::GrantDiscardAction => {
                    let seat = &mut self.state.seats[ctx.seat.index()];
                    seat.dpa += 1;
                    seat.granted_dpa += 1;
                }
                Effect::AddHostBlood(n) => {
                    self.state.gain_blood(host.expect("host"), n);
                }
                // A strike from a card in play (a weapon, Treasured
                // Samadji's dodge) remembers its card: unavailable when the
                // strike resolves, it has no effect (spec §4.10.4).
                Effect::Strike(k) => {
                    self.apply_combat_effect(Effect::Strike(k), &ctx);
                    let cb = self.combat.as_mut().expect("combat context");
                    let i = cb.side_of_minion(host.expect("host")).expect("combatant");
                    // A vampire's own strike (Flávio Gonçalves' dodge) has
                    // no card to lose; nor the opposing minion's on the
                    // bearer's card (Kaymakli Fragment: the steal checks it).
                    cb.sides[i].strike_source = match source {
                        AbilitySource::Card(_) if mode.actor == Actor::OpposingCombatant => None,
                        AbilitySource::Card(id) => Some(id),
                        AbilitySource::Vampire(_) => None,
                    };
                }
                Effect::CommitStrike => {
                    let cb = self.combat.as_mut().expect("combat context");
                    let i = cb.side_of_minion(host.expect("host")).expect("combatant");
                    cb.sides[i].committed = Some(source.card());
                }
                // One point a use; the first use opens the combat's
                // allowance (rules-qa 10.3).
                Effect::PreventGunStrikeAllowance(n) => {
                    let id = source.card();
                    let cb = self.combat.as_mut().expect("combat context");
                    let i = cb.side_of_minion(host.expect("host")).expect("combatant");
                    let k = match cb.gun_prevention_left.iter().position(|&(c, _)| c == id) {
                        Some(k) => k,
                        None => {
                            cb.gun_prevention_left.push((id, n));
                            cb.gun_prevention_left.len() - 1
                        }
                    };
                    cb.gun_prevention_left[k].1 -= 1;
                    // A gun point is strike damage.
                    let gun = cb.sides[i].pending.gun.min(1);
                    cb.sides[i].pending.prevent_gun(1);
                    let sd = &mut cb.sides[i].strike_damage;
                    sd.normal -= gun.min(sd.normal);
                    sd.gun -= gun.min(sd.gun);
                }
                Effect::ContinuePressEachCombat => {
                    let id = source.use_key(ability);
                    let cb = self.combat.as_mut().expect("combat context");
                    let i = cb.side_of_minion(host.expect("host")).expect("combatant");
                    if !cb.abilities_used.contains(&id) {
                        cb.abilities_used.push(id);
                        cb.sides[i].continue_presses += 1;
                    }
                }
                // A cost: cards discarded at random from the user's hand,
                // then replaced (Phaibun).
                Effect::DiscardAtRandom(n) => {
                    let k = (n as usize).min(self.state.seats[ctx.seat.index()].hand.len());
                    self.discard_at_random(ctx.seat, k);
                    self.replace(ctx.seat, k, None);
                }
                // The number picked, capped by the library as the ability
                // resolves; drawn with no discard-down before the same
                // number goes back to the bottom (rules-qa 9.5, 11.1).
                Effect::DrawThenBottom => {
                    let library = self.state.seats[ctx.seat.index()].library.len();
                    let n = ctx.x.min(library.min(u8::MAX as usize) as u8);
                    if n > 0 {
                        self.interrupts.push(Interrupt::Draws {
                            seat: ctx.seat,
                            left: n,
                            drawn: 0,
                        });
                    }
                }
                // No votes once the cost ousted the user (LSJ 20081105).
                Effect::MethuselahVotes { n, favor } => {
                    if !self.state.seats[ctx.seat.index()].ousted && self.referendum.is_some() {
                        self.methuselah_votes(source.card(), ctx.seat, n, favor)
                    }
                }
                Effect::TakeBleedGain => {
                    let id = source.card();
                    if let Some(a) = self.action.as_mut() {
                        if let Some(k) = a.momentum.iter().position(|&p| p == (id, true)) {
                            a.momentum.remove(k);
                            self.state.cards[id.index()].counters += 1;
                        }
                    }
                }
                Effect::RemoveCounter(n) => {
                    let c = &mut self.state.cards[source.card().index()];
                    c.counters -= n.min(c.counters);
                }
                // Major Boon burned: the Methuselah it remembers pays.
                Effect::BleedToRemembered => {
                    let against = self.state.cards[source.card().index()].against;
                    let t = (0..self.state.seats.len())
                        .map(|i| SeatId(i as u8))
                        .find(|&t| against.contains(t) && !self.state.seats[t.index()].ousted);
                    if let (Some(a), Some(t)) = (self.action.as_mut(), t) {
                        a.bleed_payer = Some((t, None));
                    }
                }
                Effect::HostVotes(n) => {
                    if let Some(h) = host {
                        let play = PlayCtx {
                            actor: Some(h),
                            ..ctx
                        };
                        self.apply_vote_effect(Effect::AddVotes(n), &play);
                    }
                }
                // "+1 vote for each counter on this card" (Powerbase:
                // Madrid), to its target.
                Effect::TargetVotesPerCounter => {
                    if let Target::Minion(t) = target {
                        let n = self.state.cards[source.card().index()].counters;
                        let play = PlayCtx {
                            actor: Some(t),
                            ..ctx
                        };
                        self.apply_vote_effect(Effect::AddVotes(n), &play);
                    }
                }
                Effect::RaiseAllyWhenEmpty(of) => {
                    let id = source.card();
                    if self.state.cards[id.index()].in_play
                        && self.state.cards[id.index()].counters == 0
                    {
                        self.burn_card_in_play(id);
                        self.interrupts.push(Interrupt::AllyFromAsh {
                            seat: ctx.seat,
                            of,
                            named: false,
                        });
                    }
                }
                // "Move 1 counter from their pool to this card" (Open War):
                // paid down to 0, its user is ousted.
                Effect::PoolToCounter(n) => {
                    self.state.seats[ctx.seat.index()].pool -= n as i32;
                    self.state.cards[source.card().index()].counters += n;
                    if self.state.seats[ctx.seat.index()].pool <= 0 {
                        self.oust_and_pass(ctx.seat);
                    }
                }
                Effect::AtCountersBurnAndGain { at, pool } => {
                    let id = source.card();
                    if self.state.cards[id.index()].in_play
                        && self.state.cards[id.index()].counters >= at
                    {
                        let controller = self.state.card_controller(id);
                        self.burn_card_in_play(id);
                        self.state.seats[controller.index()].pool += pool as i32;
                    }
                }
                Effect::GainMasterPhaseAction => self.state.seats[ctx.seat.index()].mpa += 1,
                Effect::WatchDamagesActing(n) => {
                    if let Some(a) = self.action.as_mut() {
                        a.watch_damaged = true;
                        let acting = a.acting;
                        let v = &self.state.minions[acting.index()];
                        if v.present() && v.is_vampire() {
                            self.damage_out_of_combat(acting, n, false);
                        }
                    }
                }
                Effect::UnlockActing => {
                    if let Some(a) = self.action.as_mut() {
                        let m = a.acting;
                        // British Museum: once each turn per minion (its
                        // mark only, not another source's unlock).
                        if a.kind == ActionKind::Equip
                            && mode
                                .requires
                                .contains(&Requirement::PlainEquipFromHandOncePerTurn)
                        {
                            self.state
                                .turn_uses
                                .push((AbilitySource::Vampire(m), u8::MAX - 1));
                        }
                        self.effect_unlock(m);
                    }
                }
                Effect::ReduceActionBlood(n) => {
                    if let Some(a) = self.action.as_mut() {
                        a.blood_reduction += n;
                    }
                }
                Effect::PayActionWithCountersX => {
                    if let Some(a) = self.action.as_mut() {
                        a.counter_payments.push((source.card(), ctx.x));
                    }
                }
                Effect::PoolToCounterPlusBank { pool, bank } => {
                    self.pay_pool_for_ability(ctx.seat, pool as i32);
                    if !self.state.seats[ctx.seat.index()].ousted {
                        self.state.cards[source.card().index()].counters += pool + bank;
                    }
                }
                Effect::BurnSelfWithoutCounters => {
                    if self.state.cards[source.card().index()].counters == 0 {
                        self.burn_card_in_play(source.card());
                    }
                }
                // The Erciyes Fragments: the card named next.
                Effect::StoreFromPreyAsh => self.interrupts.push(Interrupt::StorePreyAsh {
                    seat: ctx.seat,
                    card: source.card(),
                }),
                Effect::StoreFromOwnAsh => self.interrupts.push(Interrupt::StoreOwnAsh {
                    seat: ctx.seat,
                    card: source.card(),
                }),
                Effect::StoredToLibraryTop => self.interrupts.push(Interrupt::StoredToLibraryTop {
                    seat: ctx.seat,
                    card: source.card(),
                }),
                Effect::RemoveCountersX => {
                    let c = &mut self.state.cards[source.card().index()];
                    c.counters -= ctx.x.min(c.counters);
                }
                // Illusory Resources: "reduce a bleed against you by X".
                Effect::ReduceBleedX => self.apply_action_effect(Effect::ReduceBleed(ctx.x), &ctx),
                Effect::AddVotesX => self.apply_vote_effect(Effect::AddVotes(ctx.x), &ctx),
                Effect::TargetBloodToPool(n) => {
                    if let Target::Minion(t) = target {
                        let v = &mut self.state.minions[t.index()];
                        let moved = n.min(v.blood);
                        v.blood -= moved;
                        let owner = v.owner;
                        self.state.seats[owner.index()].pool += moved as i32;
                    }
                }
                Effect::PoolToTargetBlood(n) => {
                    if let Target::Minion(t) = target {
                        self.pay_pool_for_ability(ctx.seat, n as i32);
                        if !self.state.seats[ctx.seat.index()].ousted {
                            self.state.gain_blood(t, n);
                        }
                    }
                }
                Effect::CounterToTargetBlood => {
                    if let Target::Minion(t) = target {
                        let c = &mut self.state.cards[source.card().index()];
                        if c.counters > 0 {
                            c.counters -= 1;
                            self.state.gain_blood(t, 1);
                        }
                    }
                }
                Effect::CryptNamedToUncontrolled(name) => {
                    let seat = &mut self.state.seats[ctx.seat.index()];
                    if let Some(k) = seat
                        .crypt
                        .iter()
                        .position(|&c| crate::crypt::CRYPT[c as usize].name == name)
                    {
                        let (card, copy) = seat.crypt.take(k);
                        seat.uncontrolled.push(crate::state::UncontrolledSlot {
                            card,
                            copy: crate::copies::Untracked(copy),
                            counters: 0,
                            banished: None,
                        });
                        self.state.seats[ctx.seat.index()]
                            .crypt
                            .shuffle(&mut self.rng);
                    }
                }
                Effect::PayPoolX => self.pay_pool_for_ability(ctx.seat, ctx.x as i32),
                Effect::GhoulFromLibraryTop(body) => {
                    if let Some((card, copy)) =
                        self.state.seats[ctx.seat.index()].library.take_top()
                    {
                        self.state.minions.push(crate::state::Minion {
                            copy: crate::copies::Untracked(copy),
                            ..crate::state::Minion::new(
                                ctx.seat,
                                crate::state::MinionKind::Token {
                                    card,
                                    body,
                                    source: source.card(),
                                },
                                body.life,
                            )
                        });
                    }
                }
                Effect::PeekHand => {
                    if let Target::Seat(t) = target {
                        self.state.peeks.push((ctx.seat, t));
                    }
                }
                Effect::MoveBetweenVampires => {
                    if let Target::Minion(from) = target {
                        self.interrupts.push(Interrupt::MoveBetween {
                            seat: ctx.seat,
                            from,
                            to: None,
                            blood_done: false,
                        });
                    }
                }
                e @ (Effect::GiveBloodToOthers | Effect::GiveBloodToOwnAnarch) => {
                    if let Target::Minion(t) = target {
                        self.interrupts.push(Interrupt::GiveBlood {
                            seat: ctx.seat,
                            from: t,
                            anarchs: e == Effect::GiveBloodToOwnAnarch,
                        });
                    }
                }
                Effect::BurnUnlessControllerPays(n) => {
                    if let Target::Card(c) = target {
                        let seat = self.state.card_controller(c);
                        self.interrupts.push(Interrupt::PayOrBurn {
                            seat,
                            card: c,
                            pool: n,
                        });
                    }
                }
                Effect::DamageTarget(n) => {
                    if let Target::Minion(t) = target {
                        // Chosen by a Gehenna event (The New Inquisition),
                        // Karen takes none (LSJ 20081120-2).
                        let gehenna = matches!(source, AbilitySource::Card(c)
                            if LIBRARY[self.state.cards[c.index()].card as usize]
                                .labels
                                .contains(&Label::Gehenna));
                        if self.state.minions[t.index()].present()
                            && !(gehenna && self.state.gehenna_immune(t))
                        {
                            self.damage_out_of_combat(t, n, false);
                        }
                    }
                }
                Effect::PayPoolPerOtherEvent => {
                    let n = self.state.other_events(source.card()) as i32;
                    self.pay_pool_for_ability(ctx.seat, n);
                }
                // No votes once the cost ousted the user (LSJ 20081105).
                Effect::MethuselahVotesX { per, favor } => {
                    if !self.state.seats[ctx.seat.index()].ousted && self.referendum.is_some() {
                        self.methuselah_votes(source.card(), ctx.seat, per * ctx.x, favor)
                    }
                }
                Effect::ContinuePressForOwnCombatant => {
                    let cb = self.combat.as_mut().expect("combat context");
                    if let Some(i) = cb.side_of(ctx.seat) {
                        cb.sides[i].continue_presses += 1;
                    }
                }
                // Discarded down afterward by the engine's discard-down.
                Effect::DrawThenDiscardDown(n) => {
                    for _ in 0..n {
                        let seat = &mut self.state.seats[ctx.seat.index()];
                        if let Some((c, copy)) = seat.library.take_top() {
                            seat.hand.put(c, copy);
                            self.offer_redraw(ctx.seat, c);
                        }
                    }
                }
                // Scourge of the Enochians: to the predator at once (ANK
                // 20200508-1).
                Effect::PredatorTakesControl => {
                    let c = source.card();
                    if self.state.cards[c.index()].present() {
                        self.predator_takes_control(c, ctx.seat);
                    }
                }
                // Its picks come next, before any other decision.
                Effect::RemoveOwnAshCards(n) => self.interrupts.push(Interrupt::RemoveFromAsh {
                    seat: ctx.seat,
                    of: ctx.seat,
                    left: n,
                    pool: 0,
                }),
                e => match e.family() {
                    Family::Shared => self.apply_shared_effect(e, &ctx, By::Ability(source)),
                    // An action modifier printed on a vampire (Larissa
                    // Moreira, Daria, Aksinya Daclau) or a card in play's
                    // effect on the action (47th Street Royals, Szlachta
                    // Bodyguard, Under Siege, Fever Pitch), +n intercept
                    // for the rest of the action (rules-qa 15.2); offered
                    // in an action's windows only (`cards::families`'
                    // test).
                    Family::Action => match self.action {
                        Some(_) => self.apply_action_effect(e, &ctx),
                        None => unreachable!("an action effect outside an action: {e:?}"),
                    },
                    Family::Combat => self.apply_combat_effect(e, &ctx),
                    // A referendum ability's votes and abstentions (Alexa
                    // Draper, Queen Anne, Yoruba Shrine's failure).
                    Family::Vote => self.apply_vote_effect(e, &ctx),
                    Family::Marker => {}
                    Family::Ability => unreachable!("an ability effect without its arm: {e:?}"),
                    f @ (Family::Play | Family::Success | Family::Entry | Family::Rider) => {
                        unreachable!("{f:?} effect in an ability: {e:?}")
                    }
                },
            }
        }
        moved || self.state.seats[ctx.seat.index()].ousted
    }

    /// An ability's pool cost, paid down to 0: its user is ousted — on
    /// their turn the turn passes, else it goes on without them.
    pub(crate) fn pay_pool_for_ability(&mut self, seat: SeatId, n: i32) {
        self.state.seats[seat.index()].pool -= n;
        if n > 0 {
            self.note_pool_burned(seat);
        }
        if self.state.seats[seat.index()].pool <= 0 {
            if seat == self.state.active {
                self.oust_and_pass(seat);
            } else {
                self.oust(seat);
            }
        }
    }

    /// The unlock-phase riders printed on vampire `m` due in its
    /// controller's unlock phase, their requirements read on it now.
    pub(crate) fn printed_unlock_riders(&self, m: MinionId) -> Vec<&'static cards::Rider> {
        if !self.state.minions[m.index()].present() {
            return Vec::new();
        }
        self.printed_riders(m, Hook::UnlockPhase, Some(self.state.active))
    }

    /// Unlock-phase effects: an ordered window clockwise from the active
    /// seat; its Methuselah resolves every pending mandatory rider (Anarch
    /// Revolt), ordered among their optional abilities, before passing
    /// (rules-qa 6.1); then the others use theirs (Homunculus).
    pub(crate) fn abilities_request(
        &self,
        window: &OrderedWindow,
        pending: &[Pending],
        edge_taken: bool,
    ) -> Option<DecisionRequest> {
        let seat = window.current()?;
        let mut options = Vec::new();
        // A contest whose rivals are gone since the window opened (another
        // unlock effect removed them) is no longer due: it is released as
        // the impulse passes.
        let live: Vec<&Pending> = pending
            .iter()
            .filter(|p| !matches!(p, Pending::Contest(i) if self.contest_choices(*i).is_empty()))
            .collect();
        if seat == self.state.active && !live.is_empty() {
            for p in live {
                match *p {
                    Pending::Rider(c) => options.push(Choice::ResolveRider { card: c.0 }),
                    Pending::Printed(m) => options.push(Choice::ResolvePrinted { minion: m }),
                    // Re-read: a title's vampire may have lost its blood
                    // to another unlock effect since (a forced yield now).
                    Pending::Contest(item) => options.extend(self.contest_choices(item)),
                }
            }
        } else {
            options.push(Choice::Pass);
        }
        // The Edge's pool ("if you have the Edge, you may gain 1 pool"), once,
        // ordered among the other unlock effects (Dreams of the Sphinx's
        // Edge lock first, then the pool).
        if seat == self.state.active && self.state.seats[seat.index()].has_edge && !edge_taken {
            options.push(Choice::BurnEdge);
        }
        // An infernal minion locked: 1 pool to unlock it, any time in the
        // phase — again if locked again (advanced rules §5.7.3).
        if seat == self.state.active && self.state.seats[seat.index()].pool > 0 {
            options.extend(
                self.state
                    .minions_of(seat)
                    .filter(|&m| {
                        self.state.minions[m.index()].locked && self.state.unlocks_as_infernal(m)
                    })
                    .map(|minion| Choice::UnlockInfernal { minion }),
            );
        }
        options.extend(self.ability_options(seat, Window::UnlockPhaseAny));
        if seat == self.state.active {
            options.extend(self.ability_options(seat, Window::UnlockPhaseOwn));
        }
        options.extend(self.ability_options(seat, Window::AnyImpulse));
        options.extend(self.burn_option_discards(seat));
        Some(DecisionRequest::new(
            seat,
            WindowLabel::UnlockAbilities,
            options,
        ))
    }

    /// Rulebook (advanced) "Burn option": a Methuselah with no minion
    /// meeting the card's requirements may discard and replace it in any
    /// unlock phase, once each; a unique card's copy in play changes
    /// nothing (LSJ 20100203-2).
    fn burn_option_discards(&self, seat: SeatId) -> Vec<Choice> {
        let st = &self.state;
        if st.seats[seat.index()].burn_option_used {
            return Vec::new();
        }
        let hand = &st.seats[seat.index()].hand;
        hand.iter()
            .enumerate()
            .filter(|&(i, _)| crate::state::first_copy(hand, i))
            .filter(|&(_, &card)| {
                let lib = &LIBRARY[card as usize];
                lib.burn_option
                    && !st.minions_of(seat).any(|m| {
                        let v = &st.minions[m.index()];
                        // Ready minions only (owner, 2026-09-26: rules-qa
                        // 33.8).
                        let master = lib.types.contains(&CardType::Master);
                        v.ready()
                            && (lib.clans.is_empty() || st.meets_clan(m, lib.clans, master))
                            && (lib.disciplines.is_empty()
                                || lib.disciplines.iter().any(|d| st.has_discipline(m, d)))
                    })
            })
            .map(|(i, _)| Choice::Discard(i as u8))
            .collect()
    }

    pub(crate) fn abilities_apply(
        &mut self,
        mut window: OrderedWindow,
        mut pending: Vec<Pending>,
        mut edge_taken: bool,
        choice: &Choice,
    ) {
        match choice {
            Choice::Pass => {
                // Contests left without a rival: released now.
                let stale: Vec<crate::game::Contest> = pending
                    .iter()
                    .filter_map(|p| match *p {
                        Pending::Contest(i) if self.contest_choices(i).is_empty() => Some(i),
                        _ => None,
                    })
                    .collect();
                for i in stale {
                    pending.retain(|p| *p != Pending::Contest(i));
                    self.release_contest(i);
                }
                window.pass()
            }
            Choice::PayContest(item) | Choice::YieldContest(item) => {
                pending.retain(|p| *p != Pending::Contest(*item));
                if self.settle_contest(*item, matches!(choice, Choice::PayContest(_))) {
                    return; // ousted: the turn has passed
                }
                window.play();
            }
            // The Edge's pool (the Edge is kept).
            Choice::BurnEdge => {
                let s = self.state.active;
                self.state.seats[s.index()].pool += 1;
                edge_taken = true;
                window.play();
            }
            // Infernal: 1 pool, a last one included (a cost may oust).
            Choice::UnlockInfernal { minion } => {
                let s = self.state.active;
                self.state.seats[s.index()].pool -= 1;
                self.state.unlock(*minion);
                if self.state.seats[s.index()].pool <= 0 && self.oust_and_pass(s) {
                    return;
                }
                window.play();
            }
            // A burn-option discard, replaced.
            Choice::Discard(i) => {
                let s = window.current().expect("impulse holder");
                let seat = &mut self.state.seats[s.index()];
                let (card, copy) = seat.hand.take(*i as usize);
                seat.ash.put(card, copy);
                seat.burn_option_used = true;
                self.replace(s, 1, None);
                window.play();
            }
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                let seat = window.current().expect("impulse holder");
                window.play();
                let resume = Flow::Abilities {
                    window: window.clone(),
                    pending: pending.clone(),
                    edge_taken,
                };
                if self.use_ability(seat, *source, *ability, *actor, *target, resume) {
                    return; // ousted (the turn has passed) or terms pending
                }
            }
            Choice::ResolvePrinted { minion } => {
                pending.retain(|p| *p != Pending::Printed(*minion));
                window.play();
                // Its requirements read again as it resolves.
                for r in self.printed_unlock_riders(*minion) {
                    for e in r.effects {
                        match *e {
                            // Arika, in her controller's prey's (the active
                            // Methuselah's) unlock phase: each location they
                            // control now.
                            Effect::PreyPaysOrBurnsLocations(n) => {
                                let s = self.state.active;
                                for ci in 0..self.state.cards.len() {
                                    let id = CardId(ci as u32);
                                    let c = &self.state.cards[ci];
                                    if c.present()
                                        && self.state.card_controller(id) == s
                                        && LIBRARY[c.card as usize]
                                            .labels
                                            .contains(&Label::Location)
                                    {
                                        self.interrupts.push(Interrupt::PayOrBurn {
                                            seat: s,
                                            card: id,
                                            pool: n,
                                        });
                                    }
                                }
                            }
                            // Leandro, in each other Methuselah's unlock
                            // phase: theirs to choose.
                            Effect::BurnPoolOrNoTransfers(n) => {
                                self.interrupts.push(Interrupt::PoolOrNoTransfers {
                                    seat: self.state.active,
                                    pool: n,
                                });
                            }
                            Effect::PreyChoosesMinionDamaged(n) => {
                                let prey = self.state.prey(self.state.active);
                                self.interrupts.push(Interrupt::MinionOf {
                                    seat: prey,
                                    of: prey,
                                    purpose: crate::draws::MinionPurpose::Damage(n),
                                });
                            }
                            other => unreachable!("not a printed unlock rider effect: {other:?}"),
                        }
                    }
                }
            }
            Choice::ResolveRider { card } => {
                pending.retain(|p| *p != Pending::Rider(CardId(*card)));
                let id = CardId(*card);
                // Web of Knives Recruit: two at most each unlock phase.
                if self.state.cards[id.index()].training {
                    self.train(id);
                    let seat = &mut self.state.seats[self.state.active.index()];
                    seat.training_burned += 1;
                    if seat.training_burned >= 2 {
                        pending.retain(|p| {
                            !matches!(p, Pending::Rider(o) if self.state.cards[o.index()].training)
                        });
                    }
                    window.play();
                    self.flow = Flow::Abilities {
                        window,
                        pending,
                        edge_taken,
                    };
                    return;
                }
                // Once a turn by name: the other copies' are no longer due.
                let once = cards::lookup(self.state.cards[id.index()].card).is_some_and(|d| {
                    d.riders.iter().any(|r| {
                        r.hook == Hook::UnlockPhase
                            && r.requires.contains(&Requirement::OneCopyPerTurn)
                    })
                });
                if once {
                    let (name, controller) = (
                        self.state.cards[id.index()].card,
                        self.state.card_controller(id),
                    );
                    pending.retain(|p| {
                        !matches!(p, Pending::Rider(o)
                            if self.state.cards[o.index()].card == name
                                && self.state.card_controller(*o) == controller)
                    });
                }
                window.play();
                let resume = Flow::Abilities {
                    window,
                    pending,
                    edge_taken,
                };
                // Sets the flow: the resume, a choice window the rider
                // needs, or the next turn (the active Methuselah ousted).
                self.resolve_unlock_rider(CardId(*card), resume);
                return;
            }
            other => unreachable!("illegal ability choice {other:?}"),
        }
        self.flow = Flow::Abilities {
            window,
            pending,
            edge_taken,
        };
    }

    /// A training counter burned from `id` (Web of Knives Recruit); the
    /// last one, it moves to the ready region as the vampire it becomes,
    /// its card that vampire — no action put it in play (LSJ 20060616-2).
    pub(crate) fn train(&mut self, id: CardId) {
        let c = &mut self.state.cards[id.index()];
        c.counters = c.counters.saturating_sub(1);
        if c.counters > 0 {
            return;
        }
        let (card, mode, owner) = (c.card, c.mode, c.controller);
        let Some(t) = cards::lookup(card).and_then(|d| {
            d.modes[mode as usize]
                .effects
                .iter()
                .find_map(|e| match *e {
                    Effect::TrainInUncontrolled { becomes, .. } => Some(becomes),
                    _ => None,
                })
        }) else {
            unreachable!("a training card without its vampire")
        };
        let c = &mut self.state.cards[id.index()];
        c.training = false;
        c.in_play = true;
        let m = MinionId(self.state.minions.len() as u32);
        self.state.minions.push(crate::state::Minion::new(
            owner,
            crate::state::MinionKind::Created {
                card,
                id,
                clan: t.clan,
                sect: t.sect,
                born: &t.born,
            },
            t.blood,
        ));
        self.state.cards[id.index()].body = Some(m);
    }

    /// Resolve a card in play's unlock-phase rider on the active
    /// Methuselah's turn, then set the flow: `resume`, a choice window the
    /// rider needs (Smiling Jack's payments), or the next turn if the
    /// active Methuselah is ousted (or the game over).
    fn resolve_unlock_rider(&mut self, id: CardId, resume: Flow) {
        let s = self.state.active;
        let c = &self.state.cards[id.index()];
        if !c.present() {
            self.flow = resume;
            return;
        }
        let def = cards::lookup(c.card).expect("definition");
        let mut own_loss = 0i32;
        let mut to_counters = 0u8;
        let mut add_counters = 0u8;
        let mut per_counter = false;
        let mut burn_self = false;
        let mut loss = vec![0i32; self.state.seats.len()];
        let controller = self.state.card_controller(id);
        let at = HookAt::of(s);
        let riders: Vec<&cards::Rider> = def
            .riders
            .iter()
            .filter(|r| r.hook == Hook::UnlockPhase && self.rider_fires(id, r, &at))
            .collect();
        let mut lifeless = None;
        for r in riders {
            for e in r.effects {
                match *e {
                    Effect::BurnPool(n) => own_loss += n as i32,
                    // Augury of Doom, in its controller's prey's phase: 1
                    // pool for each of their vampires in torpor; none, the
                    // card burns (rules-qa 54.2).
                    Effect::PreyBurnsPoolPerTorpor => {
                        let n = self
                            .state
                            .minions_of(s)
                            .filter(|&m| self.state.minions[m.index()].in_torpor)
                            .count();
                        if n == 0 {
                            burn_self = true;
                        } else {
                            own_loss += n as i32;
                        }
                    }
                    Effect::PoolToCounter(n) => {
                        own_loss += n as i32;
                        to_counters += n;
                    }
                    Effect::PoolOrBloodPerCounter
                    | Effect::PoolOrRandomCardsPerCounter
                    | Effect::PoolOrAshCardsPerCounter => per_counter = true,
                    Effect::AddCounter(n) => add_counters += n,
                    Effect::UnlockBurnPoolUnlessReady { sect, pool } => {
                        let has = self.state.minions_of(s).any(|m| {
                            self.state.minions[m.index()].ready()
                                && self.state.sect_of(m) == Some(sect)
                        });
                        if !has {
                            own_loss += pool as i32;
                        }
                    }
                    Effect::PreyBurnsPool(n) => {
                        loss[self.state.prey(s).index()] += n as i32;
                    }
                    Effect::TortureBurnPool(n) => {
                        for (seat, l) in loss.iter_mut().enumerate() {
                            let tortured = self.state.minions_of(SeatId(seat as u8)).any(|m| {
                                let v = &self.state.minions[m.index()];
                                v.is_vampire() && v.torture > 0
                            });
                            if tortured {
                                *l += n as i32;
                            }
                        }
                    }
                    // "During your unlock phase, burn this ally" (Spectral
                    // Servitor).
                    Effect::BurnSelf => burn_self = true,
                    Effect::BurnSelfUnlessPool(_) => per_counter = true,
                    // Antediluvian Awakening: the active Methuselah's pool,
                    // or one of their vampires to burn it.
                    Effect::BurnPoolUnlessBurnVampire { pool, cap } => {
                        self.interrupts.push(Interrupt::PoolOrVampire {
                            seat: s,
                            card: id,
                            pool,
                            cap,
                        });
                    }
                    // Waiting Game: every Methuselah's anarchs and allies.
                    Effect::AnarchsBloodOrCamarillaAlliesBurn => {
                        for i in 0..self.state.minions.len() {
                            let m = MinionId(i as u32);
                            let v = &self.state.minions[i];
                            if !v.present() {
                                continue;
                            }
                            if v.is_ally() {
                                let v = &mut self.state.minions[i];
                                v.blood = v.blood.saturating_sub(1);
                                if v.blood == 0 {
                                    self.burn_minion(m);
                                }
                            } else if self.state.sect_of(m) == Some(crate::crypt::Sect::Anarch) {
                                self.interrupts.push(Interrupt::BloodOrCamarilla {
                                    seat: v.owner,
                                    minion: m,
                                });
                            }
                        }
                    }
                    // Bleeding the Vine: its controller locks it or burns
                    // the pool.
                    Effect::LockOrBurnPool(n) => {
                        self.interrupts.push(Interrupt::LockOrPay {
                            seat: controller,
                            card: id,
                            pool: n,
                        });
                    }
                    Effect::LockUnlessDiscard => {
                        let n = self.state.gehenna_in_play();
                        let ms: Vec<MinionId> = self.state.minions_of(s).collect();
                        for m in ms {
                            let v = &self.state.minions[m.index()];
                            let mortal = v.kind.ally_card().is_some_and(|c| {
                                LIBRARY[c as usize].labels.contains(&Label::Mortal)
                            });
                            let big = v.is_vampire()
                                && self.state.capacity_of(m).is_some_and(|c| c as usize > n);
                            if !mortal && !big {
                                self.interrupts
                                    .push(Interrupt::LockOrDiscard { seat: s, minion: m });
                            }
                        }
                    }
                    Effect::BurnOfCrowdedClan => {
                        if !self.state.seats[s.index()].recalled_ignored {
                            self.interrupts.push(Interrupt::MinionOf {
                                seat: s,
                                of: s,
                                purpose: crate::draws::MinionPurpose::BurnOfCrowdedClan,
                            });
                        }
                    }
                    // Fear of Mekhet: 5 blood or the vampire; then the
                    // card moves on (rules-qa 49.4).
                    Effect::HostBurnsBloodOrBurnedThenMoves(n) => {
                        let host = self.state.cards[id.index()].bearer().expect("host");
                        if self.state.minions[host.index()].blood >= n {
                            self.state.burn_blood(host, n);
                            self.interrupts.push(Interrupt::MoveCardToTitled {
                                seat: self.state.minions[host.index()].owner,
                                card: id,
                            });
                        } else {
                            lifeless = Some(host);
                        }
                    }
                    // Society of Leopold: the controller's choice.
                    Effect::HostBurnsBloodOrIsBurned(n) => {
                        let host = self.state.cards[id.index()].bearer().expect("host");
                        self.interrupts.push(Interrupt::BloodOrBurned {
                            seat: self.state.minions[host.index()].owner,
                            minion: host,
                            n,
                            card: None,
                        });
                    }
                    // Robert Carter: the employer's blood, or the retainer.
                    Effect::HostBurnsBloodOrThisBurns(n) => {
                        let host = self.state.cards[id.index()].bearer().expect("host");
                        self.interrupts.push(Interrupt::BloodOrBurned {
                            seat: self.state.minions[host.index()].owner,
                            minion: host,
                            n,
                            card: Some(id),
                        });
                    }
                    // "He gains 1 life" (Double Deuce): the ally its card is.
                    Effect::AddHostBlood(n) => {
                        let h = self.state.cards[id.index()].bearer().expect("host");
                        self.state.gain_blood(h, n);
                    }
                    // Baltimore Purge: burned, then the picks.
                    Effect::Purge { chooser_clan } => {
                        let host = self.state.cards[id.index()].bearer().expect("host");
                        self.burn_card_in_play(id);
                        self.start_purge(host, chooser_clan);
                    }
                    // "The attached minion burns 1 blood or life" (Sculpt the
                    // Flesh [PRO]): an ally with no life left is burned.
                    Effect::HostBurnBlood(n) => {
                        let host = self.state.cards[id.index()].bearer().expect("host");
                        if self.state.burn_blood_or_life(host, n) {
                            lifeless = Some(host);
                        }
                    }
                    other => unreachable!("not a rider effect: {other:?}"),
                }
            }
        }
        if loss.iter().any(|&n| n > 0) {
            // Another Methuselah's loss: ousted at once if at 0, the active
            // seat scoring; the unlock phase goes on.
            self.burn_pool_batch(&loss);
            if self.state.living() <= 1 {
                return;
            }
        }
        if own_loss > 0 {
            let seat = &mut self.state.seats[s.index()];
            let moved = own_loss.min(seat.pool) as u8;
            seat.pool = (seat.pool - own_loss).max(0);
            // Counters move even as the last pool goes (the card leaves
            // with its ousted controller anyway).
            self.state.cards[id.index()].counters += to_counters.min(moved);
            if self.state.seats[s.index()].pool == 0 && self.oust_and_pass(s) {
                return;
            }
        }
        if let Some(host) = lifeless {
            self.burn_minion(host);
            self.flow = resume;
            return;
        }
        if burn_self {
            self.burn_card_in_play(id);
            self.flow = resume;
            return;
        }
        // Burn 1 pool or the ally (Spectral Servitor [OBL]): one choice.
        if per_counter && self.rider_effect(id, Effect::BurnSelfUnlessPool(1)) {
            self.flow = Flow::RiderTerms {
                card: id,
                left: 1,
                resume: Box::new(resume),
            };
            return;
        }
        self.state.cards[id.index()].counters += add_counters;
        let counters = self.state.cards[id.index()].counters;
        if per_counter && counters > 0 {
            self.flow = Flow::RiderTerms {
                card: id,
                left: counters,
                resume: Box::new(resume),
            };
            return;
        }
        self.flow = resume;
    }

    /// One unit of a rider's payment (Smiling Jack): 1 pool, or 1 blood
    /// from a vampire the active Methuselah controls that has some (LSJ
    /// 19971212: an empty vampire does not lessen the obligation).
    pub(crate) fn rider_terms_request(&self, card: CardId, left: u8) -> DecisionRequest {
        let s = self.state.active;
        let mut options = Vec::new();
        if self.rider_effect(card, Effect::BurnSelfUnlessPool(1)) {
            options.push(Choice::BurnPool);
            options.push(Choice::BurnInPlay { card: card.0 });
        } else if self.rider_effect(card, Effect::PoolOrRandomCardsPerCounter) {
            // Constant Revolution: how many cards (the rest is pool), at
            // most the hand (LSJ 20090618) — one announcement.
            let hand = self.state.seats[s.index()].hand.len() as u8;
            options.extend((0..=left.min(hand)).map(Choice::SetX));
        } else if self.rider_effect(card, Effect::PoolOrAshCardsPerCounter) {
            // The Gate of Acheron: as many ash heap cards as there are, the
            // rest pool — cards at random, so one announcement.
            let ash = self.state.seats[s.index()].ash.len().min(255) as u8;
            options.extend((0..=left.min(ash)).map(Choice::SetX));
        } else {
            options.push(Choice::BurnPool);
            for m in self.state.minions_of(s) {
                if self.state.minions[m.index()].blood > 0 {
                    options.push(Choice::BurnBloodFrom { minion: m });
                }
            }
        }
        DecisionRequest::new(s, WindowLabel::RiderTerms, options)
    }

    /// The riders at `hook` of the cards in play that fire, in arena order:
    /// present, text not lost, on the cards `at` names, scope read against
    /// `at.seat`, requirements met (the action's at a `Hook::reads_action`
    /// hook). A site applying them one by one re-checks `present`.
    pub(crate) fn fired_riders(&self, hook: Hook, at: &HookAt) -> Vec<(CardId, &'static Rider)> {
        self.state
            .card_riders(hook)
            .into_iter()
            .filter(|&(id, r)| self.rider_fires(id, r, at))
            .collect()
    }

    /// `fired_riders`' test of card `id`'s rider `r`, also read as a
    /// rider chosen earlier resolves.
    pub(crate) fn rider_fires(&self, id: CardId, r: &Rider, at: &HookAt) -> bool {
        let st = &self.state;
        let c = &st.cards[id.index()];
        if !c.present() || !(at.on)(c) {
            return false;
        }
        if at
            .seat
            .is_some_and(|s| !r.scope.applies(st, st.card_controller(id), s))
        {
            return false;
        }
        if !r.hook.reads_action(false) {
            return st.rider_holds(r, id);
        }
        let (seat, acting) = at.acting.expect("an action hook's acting minion");
        !st.text_suppressed(id)
            && r.requires.iter().all(|&req| {
                st.rider_card_holds(req, id).unwrap_or(true)
                    && cards::holds(
                        req,
                        st,
                        seat,
                        Some(acting),
                        None,
                        self.action.as_ref(),
                        None,
                        None,
                        false,
                    )
            })
    }

    /// The riders at `hook` printed on vampire `m` that fire, in printed
    /// order: scope read against `seat`, requirements met on `m` (at a
    /// `Hook::reads_action` hook, in the action's context, `m` the actor);
    /// whether `m` must be ready or present is the site's.
    pub(crate) fn printed_riders(
        &self,
        m: MinionId,
        hook: Hook,
        seat: Option<SeatId>,
    ) -> Vec<&'static Rider> {
        let st = &self.state;
        let v = &st.minions[m.index()];
        cards::vampire_of(v)
            .map_or(&[][..], |d| d.riders)
            .iter()
            .filter(|r| {
                r.hook == hook
                    && seat.is_none_or(|s| r.scope.applies(st, v.owner, s))
                    && r.requires.iter().all(|&req| {
                        if hook.reads_action(true) {
                            cards::holds(
                                req,
                                st,
                                v.owner,
                                Some(m),
                                None,
                                self.action.as_ref(),
                                None,
                                None,
                                false,
                            )
                        } else {
                            cards::holds_for_host(req, st, m)
                        }
                    })
            })
            .collect()
    }

    /// The rider effects resolved the same at every hook, on the rider's
    /// card `id`; any other is not a rider effect of `hook`. A site
    /// batching its burns (the riders of one event, at once) takes
    /// `BurnSelf` first.
    pub(crate) fn apply_rider_effect(&mut self, id: CardId, e: Effect, hook: Hook) {
        match e {
            // Unless an earlier rider of the event took it out of play.
            Effect::BurnSelf => {
                if self.state.cards[id.index()].present() {
                    self.burn_card_in_play(id);
                }
            }
            Effect::AddCounter(n) => self.state.cards[id.index()].counters += n,
            Effect::RemoveCounter(n) => {
                let c = &mut self.state.cards[id.index()];
                c.counters = c.counters.saturating_sub(n);
            }
            Effect::BurnSelfWithoutCounters => {
                if self.state.cards[id.index()].counters == 0 {
                    self.burn_card_in_play(id);
                }
            }
            Effect::BurnSelfWithoutStored => {
                if self.state.cards[id.index()].stored.is_empty() {
                    self.burn_card_in_play(id);
                }
            }
            other => unreachable!("not a {hook:?} rider effect: {other:?}"),
        }
    }

    /// Whether a card in play carries a rider with this effect.
    fn rider_effect(&self, card: CardId, effect: Effect) -> bool {
        cards::lookup(self.state.cards[card.index()].card)
            .is_some_and(|d| d.riders.iter().any(|r| r.effects.contains(&effect)))
    }

    pub(crate) fn rider_terms_apply(
        &mut self,
        card: CardId,
        left: u8,
        resume: Box<Flow>,
        choice: &Choice,
    ) {
        let s = self.state.active;
        match choice {
            Choice::SetX(k) if self.rider_effect(card, Effect::PoolOrAshCardsPerCounter) => {
                // k ash heap cards at random removed from the game, then
                // the rest from the pool.
                for _ in 0..*k {
                    let seat = &mut self.state.seats[s.index()];
                    let i = self.rng.below(seat.ash.len());
                    let (c, copy) = seat.ash.take(i);
                    seat.removed.put(c, copy);
                }
                let seat = &mut self.state.seats[s.index()];
                seat.pool = (seat.pool - (left - k) as i32).max(0);
                if seat.pool == 0 && self.oust_and_pass(s) {
                    return;
                }
                self.flow = *resume;
                return;
            }
            Choice::SetX(k) => {
                // k cards burned at random, all at once, then the rest of
                // the units from the pool; the hand is drawn up afterward.
                self.discard_at_random(s, *k as usize);
                let seat = &mut self.state.seats[s.index()];
                seat.pool = (seat.pool - (left - k) as i32).max(0);
                if seat.pool == 0 && self.oust_and_pass(s) {
                    return;
                }
                self.replace(s, *k as usize, None);
                self.flow = *resume;
                return;
            }
            Choice::BurnPool => {
                let seat = &mut self.state.seats[s.index()];
                seat.pool -= 1;
                // Ousted on their own unlock phase: the turn passes.
                if seat.pool == 0 && self.oust_and_pass(s) {
                    return;
                }
            }
            Choice::BurnBloodFrom { minion } => {
                self.state.minions[minion.index()].blood -= 1;
            }
            Choice::BurnInPlay { card: c } => self.burn_card_in_play(CardId(*c)),
            other => unreachable!("illegal rider terms choice {other:?}"),
        }
        if left > 1 {
            self.flow = Flow::RiderTerms {
                card,
                left: left - 1,
                resume,
            };
        } else {
            self.flow = *resume;
        }
    }

    /// "During your discard phase" riders the seat holding the impulse
    /// must resolve in the active Methuselah's discard phase: The Coven's
    /// transfer is the predator's, taken when they get the opportunity to
    /// play effects (LSJ 20100723).
    fn discard_pending(&self, seat: SeatId) -> Vec<CardId> {
        let s = self.state.active;
        // The Coven's control change, its predator's to take (its
        // controller's own riders are `discard_dues`).
        let mut out: Vec<CardId> = self
            .fired_riders(Hook::DiscardPhase, &HookAt::of(s))
            .into_iter()
            .filter(|&(id, r)| {
                r.effects.contains(&Effect::PredatorTakesControl)
                    && self.state.predator(self.state.card_controller(id)) == seat
            })
            .map(|(id, _)| id)
            .collect();
        out.dedup();
        out
    }

    fn resolve_discard_rider(&mut self, id: CardId) {
        let ci = id.index();
        let def = cards::lookup(self.state.cards[ci].card).expect("definition");
        let controller = self.state.card_controller(id);
        let effects: Vec<Effect> = def
            .riders
            .iter()
            .filter(|r| r.hook == Hook::DiscardPhase)
            .flat_map(|r| r.effects.iter().copied())
            .collect();
        for e in effects {
            match e {
                Effect::PredatorTakesControl => self.predator_takes_control(id, controller),
                other => self.apply_rider_effect(id, other, Hook::DiscardPhase),
            }
        }
    }

    /// `controller`'s predator takes control of card `id` (The Coven,
    /// Scourge of the Enochians).
    pub(crate) fn predator_takes_control(&mut self, id: CardId, controller: SeatId) {
        let ci = id.index();
        let predator = self.state.predator(controller);
        // A forced contest with a copy the predator already controls burns
        // the incoming copy (spec §1.13.1).
        let name = LIBRARY[self.state.cards[ci].card as usize].name;
        let held = self.state.cards.iter().enumerate().any(|(j, o)| {
            j != ci
                && o.present()
                && LIBRARY[o.card as usize].name == name
                && self.state.card_controller(CardId(j as u32)) == predator
        });
        if held {
            self.burn_card_in_play(id);
        } else {
            self.state.cards[ci].controller = predator;
        }
    }

    /// The impulse round the other seats at the end of a phase (rules-qa
    /// 7.9): mandatory effects first (Pass masked), then untimed abilities.
    pub(crate) fn phase_rotation_request(
        &self,
        resume: &Flow,
        window: &OrderedWindow,
    ) -> Option<DecisionRequest> {
        let seat = window.current()?;
        let mut options = Vec::new();
        let pending = match resume {
            Flow::Discard => self.discard_pending(seat),
            _ => Vec::new(),
        };
        if pending.is_empty() {
            options.push(Choice::Pass);
            options.extend(self.ability_options(seat, Window::AnyImpulse));
            if matches!(resume, Flow::Discard) {
                options.extend(self.ability_options(seat, Window::OthersDiscardPhase));
            }
            // Enkil Cog, as the minion phase is about to end (rules-qa
            // 49.6): offered while its vampire has an action to take.
            if matches!(resume, Flow::MinionPhase) {
                options.extend(
                    self.ability_options(seat, Window::OthersMinionPhaseEnd)
                        .into_iter()
                        .filter(|o| match *o {
                            Choice::UseAbility { source, .. } => {
                                let m = self.state.cards[source.card().index()]
                                    .host
                                    .expect("the Cog's vampire");
                                self.action_options(seat, Some(m)).len() > 1
                            }
                            _ => true,
                        }),
                );
            }
        } else {
            options.extend(pending.iter().map(|c| Choice::ResolveRider { card: c.0 }));
        }
        Some(DecisionRequest::new(
            seat,
            WindowLabel::PhaseImpulse,
            options,
        ))
    }

    pub(crate) fn phase_rotation_apply(
        &mut self,
        resume: Box<Flow>,
        mut window: OrderedWindow,
        choice: &Choice,
    ) {
        match choice {
            Choice::Pass => {
                window.pass();
                if window.is_closed() {
                    self.close_phase(*resume);
                } else {
                    self.flow = Flow::PhaseRotation { resume, window };
                }
            }
            Choice::ResolveRider { card } => {
                self.resolve_discard_rider(CardId(*card));
                self.flow = Flow::PhaseRotation { resume, window };
            }
            Choice::UseAbility {
                source,
                ability,
                actor,
                target,
            } => {
                // An effect: the impulse returns to the active Methuselah,
                // back in their phase.
                let seat = window.current().expect("impulse holder");
                let phase = *resume;
                self.flow = phase.clone();
                if !self.use_ability(seat, *source, *ability, *actor, *target, phase.clone()) {
                    self.flow = phase;
                }
            }
            other => unreachable!("illegal phase-rotation choice {other:?}"),
        }
    }

    /// Riders of `hook` fired for each minion of `subjects` that left the
    /// ready region: a host's own (`HostToTorpor`) or every card in play's
    /// (`ReadyMinionBurnedOrTorpor`). Pool losses are batched, a Methuselah
    /// at 0 ousted here; the caller tears the combat or action down.
    pub(crate) fn leave_riders(&mut self, hook: Hook, subjects: &[MinionId]) {
        if hook == Hook::ReadyMinionBurnedOrTorpor {
            for &m in subjects {
                self.by_left_ready(m);
                self.dead_pool_counters(m);
                self.flames_offers(m);
                self.torture_offers(m);
            }
        }
        let mut loss = vec![0i32; self.state.seats.len()];
        for &subject in subjects {
            let controller = self.state.minions[subject.index()].owner;
            let on = |c: &CardInPlay| hook != Hook::HostToTorpor || c.host == Some(subject);
            for (id, r) in self.fired_riders(hook, &HookAt::on(&on)) {
                // Out of play by an earlier rider's burn.
                if !self.state.cards[id.index()].present() {
                    continue;
                }
                for &e in r.effects {
                    match e {
                        Effect::TheirControllerBurnsPool(n) => {
                            loss[controller.index()] += n as i32;
                        }
                        // A vampire of capacity n or less gone to torpor,
                        // not burned (Carver's Meat Packing and Storage).
                        Effect::HostageOnTorpor(n) => {
                            let cap = self.state.capacity_of(subject);
                            let v = &mut self.state.minions[subject.index()];
                            if v.in_play && v.in_torpor && cap.is_some_and(|c| c <= n) {
                                v.hostage += 1;
                            }
                        }
                        // "You gain 1 pool after a ready minion controlled
                        // by your prey is burned or sent to torpor"
                        // (Frontal Assault): each copy.
                        Effect::ControllerGainsPoolIfPrey(n) => {
                            let me = self.state.card_controller(id);
                            if controller == self.state.prey(me) && me != controller {
                                self.state.seats[me.index()].pool += n as i32;
                            }
                        }
                        other => self.apply_rider_effect(id, other, hook),
                    }
                }
            }
            // Printed on the vampire in combat with the subject (Egidia
            // Arrú), even if it left in the same event (advanced rules
            // §6.4.4).
            if hook == Hook::ReadyMinionBurnedOrTorpor {
                let opponent = self.combat.as_ref().and_then(|cb| {
                    cb.side_of_minion(subject)?;
                    cb.sides.iter().map(|s| s.minion).find(|&o| o != subject)
                });
                let riders = opponent.map_or(Vec::new(), |o| {
                    self.printed_riders(o, Hook::CombatOpponentLeftReady, None)
                });
                for r in riders {
                    for e in r.effects {
                        match *e {
                            Effect::TheirControllerBurnsPool(n) => {
                                loss[controller.index()] += n as i32;
                            }
                            Effect::MayGainPool(n) => {
                                let o = opponent.expect("the vampire in combat");
                                let seat = self.state.minions[o.index()].owner;
                                self.interrupts
                                    .push(Interrupt::MayGainPool { seat, minion: o, n });
                            }
                            other => unreachable!("not a leave rider effect: {other:?}"),
                        }
                    }
                }
            }
        }
        if loss.iter().any(|&n| n > 0) {
            self.burn_pool_batch(&loss);
        }
    }

    /// "After a vampire in combat with a Lasombra you control leaves the
    /// ready region, you can add 1 counter to this card" (Dead Pool):
    /// added, never a loss.
    fn dead_pool_counters(&mut self, subject: MinionId) {
        if !self.state.minions[subject.index()].is_vampire() {
            return;
        }
        let Some(opponent) = self.combat.as_ref().and_then(|cb| {
            cb.side_of_minion(subject)?;
            cb.sides.iter().map(|s| s.minion).find(|&o| o != subject)
        }) else {
            return;
        };
        let o = &self.state.minions[opponent.index()];
        let owner = o.owner;
        let Some(clan) = o.clan() else {
            return;
        };
        let hook = Hook::CombatOpponentOfOwnClanLeft(clan);
        for (id, r) in self.fired_riders(hook, &HookAt::of(owner)) {
            for &e in r.effects {
                self.apply_rider_effect(id, e, hook);
            }
        }
    }

    /// "When a minion opposing an anarch in combat leaves the ready region,
    /// the controller of that anarch may move a counter from the bank to a
    /// vampire in his or her uncontrolled region" (Flames of
    /// Insurrection): one offer for each copy in play; both Anarchs of a
    /// combat leaving, both controllers (ANK 20180627-2).
    fn flames_offers(&mut self, subject: MinionId) {
        let Some(opponent) = self.combat.as_ref().and_then(|cb| {
            cb.side_of_minion(subject)?;
            cb.sides.iter().map(|s| s.minion).find(|&o| o != subject)
        }) else {
            return;
        };
        if self.state.sect_of(opponent) != Some(crate::crypt::Sect::Anarch) {
            return;
        }
        let seat = self.state.minions[opponent.index()].owner;
        for (_, r) in self.fired_riders(Hook::CombatOpponentOfAnarchLeft, &HookAt::ANY) {
            for e in r.effects {
                match *e {
                    Effect::AnarchControllerMayBloodToUncontrolled(n) => {
                        self.interrupts.push(Interrupt::UncontrolledBlood {
                            seat,
                            clan: None,
                            n,
                            may: true,
                        })
                    }
                    other => unreachable!("not an Anarch-combat rider effect: {other:?}"),
                }
            }
        }
    }

    /// "When a non-Camarilla vampire you control sends an opposing vampire
    /// to torpor in combat, you may lock this card to put a torture counter
    /// on the opposing vampire" (The Path of Lilith): offered at once.
    fn torture_offers(&mut self, subject: MinionId) {
        let v = &self.state.minions[subject.index()];
        if !v.is_vampire() || !v.in_torpor {
            return;
        }
        let Some(opponent) = self.combat.as_ref().and_then(|cb| {
            cb.side_of_minion(subject)?;
            cb.sides.iter().map(|s| s.minion).find(|&o| o != subject)
        }) else {
            return;
        };
        let o = &self.state.minions[opponent.index()];
        if !o.is_vampire() || self.state.sect_of(opponent) == Some(crate::crypt::Sect::Camarilla) {
            return;
        }
        let seat = o.owner;
        let on = |c: &CardInPlay| !c.locked;
        let at = HookAt {
            seat: Some(seat),
            ..HookAt::on(&on)
        };
        for (id, r) in self.fired_riders(Hook::OwnNonCamarillaSendsToTorpor, &at) {
            for e in r.effects {
                match *e {
                    Effect::MayLockForTorture => self.interrupts.push(Interrupt::MayTorture {
                        seat,
                        card: id,
                        minion: subject,
                    }),
                    other => unreachable!("not a torture rider effect: {other:?}"),
                }
            }
        }
    }

    /// The acting Methuselah's `clan` vampire bled for `amount`: the cards
    /// counting it (Alamut: "put the pool lost by that Methuselah on this
    /// card").
    pub(crate) fn own_bleed_riders(
        &mut self,
        seat: SeatId,
        acting: MinionId,
        bled: SeatId,
        amount: u8,
    ) {
        // Printed on the acting vampire (Fotini Katsikaris: "+1 hand size
        // until your next discard phase").
        for r in self.printed_riders(acting, Hook::OwnBleedSuccessful, None) {
            for e in r.effects {
                match *e {
                    Effect::HandSizeUntilDiscard(n) => {
                        self.state.seats[seat.index()].timed.add(
                            TimedStat::HandSize,
                            Expiry::DiscardPhase,
                            n as i8,
                        );
                    }
                    Effect::DamageBledAnarchs(n) => {
                        let anarchs: Vec<MinionId> = self
                            .state
                            .minions_of(bled)
                            .filter(|&m| {
                                self.state.minions[m.index()].is_vampire()
                                    && self.state.sect_of(m) == Some(crate::crypt::Sect::Anarch)
                            })
                            .collect();
                        for m in anarchs {
                            if self.state.minions[m.index()].present() {
                                self.damage_out_of_combat(m, n, false);
                            }
                        }
                    }
                    other => unreachable!("not an own-bleed rider effect: {other:?}"),
                }
            }
        }
        let clan = self.state.minions[acting.index()].clan();
        for (id, r) in self.fired_riders(Hook::OwnBleedSuccessful, &HookAt::of(seat)) {
            for e in r.effects {
                match *e {
                    Effect::CountersFromOwnBleed(k) if clan == Some(k) => {
                        self.state.cards[id.index()].counters += amount;
                    }
                    Effect::CountersFromOwnBleed(_) => {}
                    other => unreachable!("not an own-bleed rider effect: {other:?}"),
                }
            }
        }
    }
}

/// Use ability `ai` of card `id` in play.
fn use_card(id: CardId, ai: usize, actor: Option<MinionId>, target: Target) -> Choice {
    Choice::UseAbility {
        source: AbilitySource::Card(id),
        ability: ai as u8,
        actor,
        target,
    }
}

/// Charisma's discount for `m` playing `mode`: a mode declaring a recruit
/// ally action only, not a recruit outside an action (LSJ 20090115-1). The
/// mode of the card played, never of a card it names (Piper's ally).
fn recruit_off(st: &crate::state::GameState, mode: &cards::ModeDef, m: MinionId) -> u8 {
    if mode
        .effects
        .contains(&Effect::DeclareAction(ActionKind::Recruit))
    {
        st.recruit_discount(m)
    } else {
        0
    }
}

/// A reduction "by n blood or pool": off the cost's pool if it has any,
/// else off its blood.
pub(crate) fn reduce(pool: &mut i32, blood: &mut u8, n: u8) {
    if *pool > 0 {
        *pool = (*pool - n as i32).max(0);
    } else {
        *blood = blood.saturating_sub(n);
    }
}

/// Is a cost of `(pool, blood)` payable out of `have`, `off` reducing it
/// (`reduce`)? A pool cost paid down to 0 counts; with `pool_may_oust`, any
/// pool cost does.
fn payable(cost: (i32, u8), off: u8, have: (i32, u8), pool_may_oust: bool) -> bool {
    let (pool, blood) = cost;
    if pool > 0 {
        (pool_may_oust || (pool - off as i32).max(0) <= have.0) && blood <= have.1
    } else {
        blood.saturating_sub(off) <= have.1
    }
}

/// The tags a card is played with in definition mode `mode` (its library
/// mode's: the Disciplines used, card-type markers of a combo card).
pub(crate) fn played_disciplines(card: u16, mode: u8) -> &'static [&'static str] {
    LIBRARY[card as usize]
        .modes
        .get(cards::library_mode(card, mode as usize))
        .map_or(&[], |m| m.tags)
}

/// What follows the cost reductions announced for a card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostThen {
    /// The card's terms, then its as-played window.
    Terms,
    /// Its as-played window: the reductions were asked after the card's
    /// terms named the card they are for (Piper's ally, Contraband's
    /// equipment).
    AsPlayed,
    /// The equipment a library search found is paid and equipped (Magic of
    /// the Smith, Michael Harris: offered once it is chosen, rules-qa 38.5).
    EquipFound,
}

/// A strike card, as played in `mode`: a strike — a dodge included — played
/// at strike declaration or at the maneuver (Slam), or a card using a melee
/// weapon's strike (Anticipation [aus]).
pub(crate) fn is_strike_mode(card: u16, mode: u8) -> bool {
    cards::lookup(card).is_some_and(|d| {
        d.modes[mode as usize].effects.iter().any(|e| {
            matches!(
                e,
                Effect::Strike(_) | Effect::CommittedStrike(_) | Effect::WeaponStrikePlus(_)
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{find_card, find_library};

    // The offer, the terms and the resolution read one user. Carver's
    // Meat Packing and Storage: "any Methuselah's vampire", no "you
    // control": the user is the controller of the vampire using it, not
    // the card's.
    #[test]
    fn any_ready_vampire_ability_is_used_by_the_vampires_controller() {
        let mut e = Engine::bare(3);
        let m = e.add_minion(SeatId(1), find_card("Abraham DuSable"), 5, false);
        let card = find_library("Carver's Meat Packing and Storage");
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), None, 0));
        let id = CardId(e.state.cards.len() as u32 - 1);
        let source = AbilitySource::Card(id);
        let ai = source
            .abilities(&e.state)
            .iter()
            .position(|a| a.actor == Actor::AnyReadyVampire)
            .expect("its vampire ability") as u8;
        let u = e.ability_user(source, ai, SeatId(1), Some(m));
        assert_eq!(u.seat, SeatId(1));
        assert_eq!((u.host, u.actor), (Some(m), Some(m)));
    }

    // A "Methuselahs can …" ability is the asked seat's, its
    // requirements read for them — not the active Methuselah's. Judgment:
    // Camarilla Segregation prints none: asked of a seat out of its turn,
    // the gate lets it through for that seat.
    #[test]
    fn methuselah_ability_is_used_by_the_seat_asked() {
        let mut e = Engine::bare(3);
        let card = find_library("Judgment: Camarilla Segregation");
        e.state
            .cards
            .push(CardInPlay::new(card, SeatId(0), None, 0));
        let source = AbilitySource::Card(CardId(e.state.cards.len() as u32 - 1));
        assert_eq!(e.state.active, SeatId(0));
        for seat in [SeatId(1), SeatId(2)] {
            let u = e
                .ability_gate(seat, Window::MasterPhase, source, 0, None)
                .expect("usable by the seat asked");
            assert_eq!((u.seat, u.host, u.actor), (seat, None, None));
        }
    }
}
