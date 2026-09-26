//! The decision-request engine: flow states, the run/choose loop with
//! window elision (DESIGN §4.3), turn structure, ousting and scoring. Phase
//! and window logic lives in the sibling modules (`actions`, `contests`,
//! `influence`, `referendum`), which extend `Engine`.

use crate::actions::ActionCtx;
use crate::cards::{self, Hook, PlayCtx};
use crate::combat::CombatCtx;
use crate::crypt::PRECONS;
use crate::decision::{ActionKind, Choice, DecisionRequest, WindowLabel};
use crate::draws::Interrupt;
use crate::ids::{MinionId, SeatId};
use crate::influence::transfers_for_turn;
use crate::library::PRECON_LIBRARIES;
use crate::plays::PendingPlay;
use crate::referendum::ReferendumCtx;
use crate::rng::Rng;
use crate::state::{
    CardId, GameState, Held, Minion, MinionKind, Seat, UncontrolledSlot, HAND_SIZE, START_POOL,
    UNCONTROLLED_START,
};
use crate::window::OrderedWindow;

/// Time-limit stand-in (owner, 2026-09-02): a tournament game rarely exceeds
/// 20 full rounds, so 100 player turns is ample. Checked at the turn
/// boundary, which stands in for the tournament rule "the game ends after the
/// current action / at the end of the current phase".
pub const TURN_LIMIT: u32 = 100;

/// One contested object of the active seat, resolved during its unlock
/// phase (rulebook "Contested Cards" / "Contested Titles").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contest {
    Card(MinionId),
    Title(MinionId),
    /// A unique library card in play (rulebook "Contested Cards").
    Library(CardId),
}

/// A mandatory unlock-phase effect the active Methuselah has yet to
/// settle, in the order they choose (rules-qa 6.1, 7.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    /// A card in play's "during their unlock phase" rider.
    Rider(CardId),
    /// Contest upkeep: pay or yield.
    Contest(Contest),
}

#[derive(Clone, Debug)]
pub(crate) enum Flow {
    /// Master phase: an offer window for the active seat's master phase
    /// actions (rulebook "Master Phase").
    MasterPhase,
    /// Numeric terms of a play, chosen as it is announced (rules-qa 3.5).
    PlayTerms {
        ctx: PlayCtx,
        resume: Box<Flow>,
    },
    /// As-played windows of pending plays, innermost last; `resume` is the
    /// flow to return to once the stack empties.
    AsPlayed {
        stack: Vec<PendingPlay>,
        resume: Box<Flow>,
    },
    /// Terms of an in-play ability being used (the master cards Tension in
    /// the Ranks' burn discards), chosen one at a time; then the ability
    /// resolves and `resume` goes on.
    AbilityTerms {
        source: crate::cards::AbilitySource,
        ability: u8,
        /// The target chosen with the use (Alicia Cortez).
        target: crate::cards::Target,
        picked: Vec<u8>,
        resume: Box<Flow>,
    },
    /// The impulse rotation closing one of the active Methuselah's phases
    /// (rules-qa 7.9): `resume` is the phase flow the impulse returns to
    /// when another seat plays an effect, and names the phase to end when
    /// everyone passes.
    PhaseRotation {
        resume: Box<Flow>,
        window: OrderedWindow,
    },
    /// A mandatory rider resolving with choices (Smiling Jack): `left`
    /// units still to pay by the active Methuselah, then `resume`.
    RiderTerms {
        card: CardId,
        left: u8,
        resume: Box<Flow>,
    },
    /// In-play abilities usable during any Methuselah's unlock phase.
    Abilities {
        window: OrderedWindow,
        /// Mandatory unlock-phase effects the active seat has yet to settle.
        pending: Vec<Pending>,
    },
    /// Minion phase: offer actions for the active seat's ready minions.
    MinionPhase,
    /// An equip action from another minion being announced: the
    /// equipment `minion` takes from `giver`, picked one at a time.
    ActionTerms {
        minion: MinionId,
        giver: MinionId,
        picked: Vec<CardId>,
    },
    /// A successful action's library search (Magic of the Smith): the
    /// equipment `minion` equips with, chosen at resolution (spec §1.14.1).
    LibrarySearch {
        minion: MinionId,
        /// A card of this kind moved to hand (Sakhar), not equipped.
        to_hand: Option<crate::cards::CardFilter>,
    },
    /// The cards an exchange action names as it is announced (Lenelle):
    /// the ash card, then the hand card.
    Exchange {
        ash: Option<u16>,
    },
    /// An action is under way (`Engine::action`).
    Action,
    /// A combat is under way inside the action (`Engine::combat`).
    Combat,
    /// A blocked leave-torpor: the blocking vampire may diablerise the
    /// acting vampire (rulebook "Leave Torpor").
    TorporBlock,
    /// Rescue resolved unblocked: split the 2-blood cost (paid at resolution).
    RescueCost,
    /// A referendum is under way (`Engine::referendum`): the blood hunt
    /// after a diablerie, or a political action's (inside the action).
    Referendum,
    Influence {
        left: u8,
    },
    /// Discard phase: an offer window for the discard phase action.
    Discard,
    GameOver,
}

pub enum Step {
    Decision(DecisionRequest),
    /// Final VPs per seat, in half-point units.
    Over(Vec<u32>),
}

pub struct Engine {
    pub state: GameState,
    /// Chosen option indices for real (non-elided) decisions only.
    pub log: Vec<usize>,
    pub decisions: u64,
    pub(crate) flow: Flow,
    /// The action under way, if any (`Flow::Action` and its sub-windows).
    pub(crate) action: Option<ActionCtx>,
    /// The combat under way inside the action, if any (`Flow::Combat`).
    pub(crate) combat: Option<CombatCtx>,
    /// The referendum under way, if any (`Flow::Referendum`).
    pub(crate) referendum: Option<ReferendumCtx>,
    /// Set when a play's resolution ousted its player: the as-played stack
    /// must not resume the ousted seat's phase.
    pub(crate) ousted_during_play: bool,
    /// Choices about cards drawn, moved or stored, served in order before
    /// any other decision (`draws`).
    pub(crate) interrupts: Vec<Interrupt>,
    /// How the most recent action ended (tests, and after-resolution
    /// riders to come).
    pub last_outcome: Option<crate::actions::Outcome>,
    /// The game's random source after setup (random discards): replay is
    /// the seed plus the decision log.
    pub(crate) rng: Rng,
}

impl Engine {
    /// A new game: each seat gets a random full-size V5 precon crypt,
    /// shuffled, with the top four cards dealt face down to the uncontrolled
    /// region (rulebook "Setup").
    pub fn new(seed: u64, n_seats: usize) -> Self {
        assert!((2..=6).contains(&n_seats));
        let mut rng = Rng::new(seed);
        let seats = (0..n_seats)
            .map(|_| {
                let precon = rng.below(PRECONS.len());
                let (_, list) = PRECONS[precon];
                let mut crypt: Vec<u16> = list
                    .iter()
                    .flat_map(|&(card, copies)| std::iter::repeat_n(card, copies as usize))
                    .collect();
                rng.shuffle(&mut crypt);
                let uncontrolled = (0..UNCONTROLLED_START)
                    .map(|_| UncontrolledSlot {
                        card: crypt.pop().expect("crypt larger than initial draw"),
                        counters: 0,
                        banished: None,
                    })
                    .collect();
                // The matching precon library, shuffled; hand of 7.
                let (_, list) = PRECON_LIBRARIES[precon];
                let mut library: Vec<u16> = list
                    .iter()
                    .flat_map(|&(card, copies)| std::iter::repeat_n(card, copies as usize))
                    .collect();
                rng.shuffle(&mut library);
                let hand = library.split_off(library.len() - HAND_SIZE);
                Seat {
                    pool: START_POOL,
                    ousted: false,
                    half_vps: 0,
                    has_edge: false,
                    crypt,
                    uncontrolled,
                    library,
                    hand,
                    ash: Vec::new(),
                    removed: Vec::new(),
                    ash_crypt: Vec::new(),
                    removed_crypt: Vec::new(),
                    mpa: 0,
                    hand_bonus: 0,
                    trifle_used: false,
                    out_of_turn: None,
                    mpa_next: 0,
                    veil_left: 0,
                    dpa: 0,
                    granted_dpa: 0,
                    master_played: false,
                    turns_taken: 0,
                    held: Held::default(),
                    forbidden_political: Vec::new(),
                }
            })
            .collect();
        let mut engine = Self {
            state: GameState {
                seats,
                minions: Vec::new(),
                cards: Vec::new(),
                phase_uses: Vec::new(),
                turn_uses: Vec::new(),
                veil_superior: false,
                phase_actions: 0,
                optional_action: false,
                once_played: Vec::new(),
                layers: Vec::new(),
                active: SeatId(0),
                turn: 0,
                combatants: None,
            },
            log: Vec::new(),
            decisions: 0,
            flow: Flow::GameOver, // replaced by start_turn
            action: None,
            combat: None,
            referendum: None,
            ousted_during_play: false,
            interrupts: Vec::new(),
            rng,
            last_outcome: None,
        };
        engine.start_turn();
        engine
    }

    /// Test fixture: `n_seats` seats with empty crypts, uncontrolled regions,
    /// libraries and hands, no minions, positioned at seat 0's minion phase
    /// on turn 1. Populate with [`Engine::add_minion`] and by pushing cards
    /// into hands.
    pub fn bare(n_seats: usize) -> Self {
        Self::bare_seeded(0, n_seats)
    }

    /// As `bare`, from a chosen seed: random effects (a coin flip, a random
    /// discard) differ per seed.
    pub fn bare_seeded(seed: u64, n_seats: usize) -> Self {
        let mut e = Self::new(seed, n_seats);
        for seat in &mut e.state.seats {
            seat.crypt.clear();
            seat.uncontrolled.clear();
            seat.library.clear();
            seat.hand.clear();
        }
        e.flow = Flow::MinionPhase;
        e
    }

    /// Test fixture: put the vampire `card` (a CRYPT index) into play for
    /// `owner`, ready and unlocked (or in torpor), with `blood`. Contests
    /// are *not* entered — use [`Engine::stage_contests`] to stage one.
    pub fn add_minion(&mut self, owner: SeatId, card: u16, blood: u8, in_torpor: bool) -> MinionId {
        self.state.minions.push(Minion {
            in_torpor,
            ..Minion::new(owner, MinionKind::Vampire(card), blood)
        });
        MinionId(self.state.minions.len() as u32 - 1)
    }

    /// Test fixture: put the ally `card` (a LIBRARY index) into play for
    /// `owner`, ready and unlocked, with its printed life and able to act
    /// (as one put in play other than by recruiting). Contests are entered.
    pub fn add_ally(&mut self, owner: SeatId, card: u16) -> MinionId {
        self.add_ally_version(owner, card, 0)
    }

    /// As [`Engine::add_ally`], in the version of `mode` (its superior
    /// version with 1).
    pub fn add_ally_version(&mut self, owner: SeatId, card: u16, mode: u8) -> MinionId {
        let m = self.recruit_ally(owner, card, mode);
        self.state.minions[m.index()].recruited = false;
        m
    }

    /// Test fixture: position the game at the active seat's master phase
    /// (master phase actions granted per rules-qa 3.3).
    pub fn at_master_phase(&mut self) {
        self.start_master_phase();
    }

    /// Test fixture: enter the card and title contests `m` would have
    /// entered on coming into play.
    pub fn stage_contests(&mut self, m: MinionId) {
        self.enter_card_contest(m);
        self.enter_title_contest(m);
    }

    /// The action under way, if any (read-only).
    pub fn action(&self) -> Option<&ActionCtx> {
        self.action.as_ref()
    }

    /// The referendum under way, if any (read-only).
    pub fn referendum(&self) -> Option<&ReferendumCtx> {
        self.referendum.as_ref()
    }

    /// Advance until a real decision is needed or the game is over. Windows
    /// with a single legal option are elided (DESIGN §4.3): auto-applied and
    /// not logged.
    pub fn run(&mut self) -> Step {
        loop {
            self.settle_interrupts();
            self.state.drain_all_excess();
            self.state.release_hostages();
            if matches!(self.flow, Flow::GameOver) {
                return Step::Over(self.state.seats.iter().map(|s| s.half_vps).collect());
            }
            match self.request() {
                None => self.advance(),
                Some(req) => {
                    debug_assert!(
                        !req.options.is_empty(),
                        "empty {:?} {:?}",
                        req.window,
                        self.flow
                    );
                    if req.options.len() == 1 {
                        let choice = req.options[0];
                        self.apply(&choice);
                    } else {
                        return Step::Decision(req);
                    }
                }
            }
        }
    }

    /// Answer the pending decision request by option index.
    pub fn choose(&mut self, idx: usize) {
        let req = self.request().expect("no pending decision");
        assert!(idx < req.options.len(), "option index out of range");
        self.log.push(idx);
        self.decisions += 1;
        let choice = req.options[idx];
        self.apply(&choice);
    }

    /// The first Methuselah, clockwise from the active one, holding more
    /// cards than their hand size (rulebook "Drawing Cards": "whenever an
    /// effect changes your hand size or adds or removes cards from your
    /// hand, immediately discard down"): an Elder Library turned contested,
    /// a hand-size bonus ending, a political action card returned to hand.
    fn discard_down_seat(&self) -> Option<SeatId> {
        let st = &self.state;
        st.clockwise_from(st.active)
            .into_iter()
            .find(|&s| !st.seats[s.index()].hand.is_empty() && st.hand_excess(s) > 0)
    }

    fn request(&self) -> Option<DecisionRequest> {
        let st = &self.state;
        // Draws first: a hand is discarded down once its draws are done
        // (Heart of Nizchetus: "without discarding").
        if let Some(req) = self.interrupt_request() {
            return Some(req);
        }
        if let Some(s) = self.discard_down_seat() {
            return Some(DecisionRequest {
                seat: s,
                window: WindowLabel::DiscardDown,
                options: (0..st.seats[s.index()].hand.len())
                    .map(|i| Choice::Discard(i as u8))
                    .collect(),
            });
        }
        match &self.flow {
            Flow::MasterPhase => Some(self.master_request()),
            Flow::PlayTerms { ctx, .. } => Some(self.terms_request(ctx)),
            Flow::AsPlayed { stack, .. } => self.as_played_request(stack),
            Flow::MinionPhase => Some(DecisionRequest {
                seat: st.active,
                window: WindowLabel::MinionPhase,
                options: self.minion_phase_options(),
            }),
            Flow::AbilityTerms {
                source,
                ability,
                picked,
                target,
                ..
            } => Some(self.ability_terms_request(*source, *ability, *target, picked)),
            Flow::RiderTerms { card, left, .. } => Some(self.rider_terms_request(*card, *left)),
            Flow::ActionTerms {
                minion,
                giver,
                picked,
            } => Some(self.action_terms_request(*minion, *giver, picked)),
            Flow::PhaseRotation { resume, window } => self.phase_rotation_request(resume, window),
            Flow::Abilities { window, pending } => self.abilities_request(window, pending),
            Flow::LibrarySearch { minion, to_hand } => Some(self.search_request(*minion, *to_hand)),
            Flow::Exchange { ash } => Some(self.exchange_request(*ash)),
            Flow::Action => self.action_request(),
            Flow::Combat => self.combat_request(),
            Flow::TorporBlock => Some(self.torpor_block_request()),
            Flow::RescueCost => Some(self.rescue_request()),
            Flow::Referendum => self.referendum_request(),
            Flow::Influence { left } => Some(self.influence_request(*left)),
            Flow::Discard => Some(self.discard_request()),
            Flow::GameOver => None,
        }
    }

    /// Deterministic transitions for states with no decision to offer.
    fn advance(&mut self) {
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match flow {
            Flow::AsPlayed { stack, resume } => self.as_played_advance(stack, resume),
            Flow::PhaseRotation { resume, .. } => self.close_phase(*resume),
            Flow::Abilities { window, .. } => {
                debug_assert!(window.is_closed());
                self.start_master_phase();
            }
            Flow::Action => self.action_advance(),
            Flow::Combat => self.combat_advance(),
            Flow::Referendum => self.referendum_advance(),
            other => unreachable!("advance() with a pending decision: {other:?}"),
        }
    }

    fn apply(&mut self, choice: &Choice) {
        if !self.interrupts.is_empty() {
            self.interrupt_apply(choice);
            return;
        }
        if let Some(s) = self.discard_down_seat() {
            let Choice::Discard(i) = choice else {
                unreachable!("illegal discard-down choice {choice:?}");
            };
            let seat = &mut self.state.seats[s.index()];
            let card = seat.hand.remove(*i as usize);
            seat.ash.push(card);
            return;
        }
        let flow = std::mem::replace(&mut self.flow, Flow::GameOver);
        match (flow, choice) {
            (Flow::MasterPhase, c) => self.master_apply(c),
            (Flow::PlayTerms { ctx, resume }, c) => self.terms_apply(ctx, *resume, c),
            (Flow::AsPlayed { stack, resume }, c) => self.as_played_apply(stack, resume, c),
            (Flow::Discard, c) => self.discard_apply(c),
            (Flow::MinionPhase, Choice::Pass) => self.open_phase_rotation(Flow::MinionPhase),
            (
                Flow::MinionPhase,
                Choice::TakeAction {
                    minion,
                    action,
                    target,
                },
            ) => match action {
                // The equipment taken is announced with the action.
                ActionKind::EquipFrom => {
                    self.flow = Flow::ActionTerms {
                        minion: *minion,
                        giver: target.expect("minion to equip from"),
                        picked: Vec::new(),
                    }
                }
                _ => self.declare_action(*minion, *action, *target, None, None),
            },
            (
                Flow::ActionTerms {
                    minion,
                    giver,
                    picked,
                },
                c,
            ) => self.action_terms_apply(minion, giver, picked, c),
            (
                Flow::MinionPhase,
                Choice::UseAbility {
                    source,
                    ability,
                    actor,
                    target,
                },
            ) => {
                // A granted action opens its action; an untimed ability
                // (The Coven) leaves the minion phase where it was.
                if !self.use_ability(*source, *ability, *actor, *target, Flow::MinionPhase) {
                    self.flow = Flow::MinionPhase;
                }
            }
            (
                Flow::MinionPhase,
                Choice::Play {
                    card,
                    mode,
                    actor,
                    target,
                },
            ) => {
                let s = self.state.active;
                self.announce_play(s, *card, *mode, *actor, *target, Flow::MinionPhase);
            }
            (
                Flow::AbilityTerms {
                    source,
                    ability,
                    target,
                    picked,
                    resume,
                },
                Choice::Discard(i),
            ) => self.ability_terms_apply(source, ability, target, picked, resume, *i),
            (
                Flow::AbilityTerms {
                    source,
                    ability,
                    target,
                    resume,
                    ..
                },
                Choice::SetX(x),
            ) => self.ability_count_apply(source, ability, target, *resume, *x),
            (
                Flow::AbilityTerms {
                    source,
                    ability,
                    target,
                    resume,
                    ..
                },
                Choice::FindCard(card),
            ) => self.ability_card_apply(source, ability, target, *resume, *card),
            (Flow::RiderTerms { card, left, resume }, c) => {
                self.rider_terms_apply(card, left, resume, c)
            }
            (Flow::PhaseRotation { resume, window }, c) => {
                self.phase_rotation_apply(resume, window, c)
            }
            (Flow::Abilities { window, pending }, c) => self.abilities_apply(window, pending, c),
            (Flow::LibrarySearch { minion, to_hand }, c) => self.search_apply(minion, to_hand, c),
            (Flow::Exchange { ash }, c) => self.exchange_apply(ash, c),
            (Flow::Action, c) => self.action_apply(c),
            (Flow::Combat, c) => self.combat_apply(c),
            (Flow::TorporBlock, c) => self.torpor_block_apply(c),
            (Flow::RescueCost, Choice::PayRescue { from_acting }) => {
                self.rescue_apply(*from_acting)
            }
            (Flow::Referendum, c) => self.referendum_apply(c),
            (Flow::Influence { left }, c) => self.influence_apply(left, c),
            (flow, choice) => unreachable!("illegal choice {choice:?} in {flow:?}"),
        }
    }

    /// The unlock-phase effects window (rules-qa 3.7, 6.1, 7.8): one
    /// ordered impulse sequence over the seats clockwise from the active
    /// one. The active Methuselah settles their mandatory effects — contest
    /// upkeep and every pending rider (each copy of Anarch Revolt in play,
    /// whoever controls it, spec §1.15) — in the order they choose, among
    /// their optional ones (the Edge's pool, abilities), and cannot pass
    /// the impulse before; then the other seats' abilities. Contests with
    /// no rival left and forced yields settle at once (no decision).
    pub(crate) fn start_unlock_effects(&mut self) {
        let s = self.state.active;
        let mut pending = Vec::new();
        for item in self.collect_contests() {
            match self.contest_choices(item).len() {
                0 => self.release_contest(item),
                1 => {
                    if self.settle_contest(item, false) {
                        return; // ousted: the turn has passed
                    }
                }
                _ => pending.push(Pending::Contest(item)),
            }
        }
        pending.extend(
            self.state
                .cards
                .iter()
                .enumerate()
                .filter(|(i, c)| {
                    c.present()
                        && cards::lookup(c.card).is_some_and(|d| {
                            let controller = self.state.card_controller(CardId(*i as u32));
                            d.riders.iter().any(|r| {
                                r.hook == Hook::UnlockPhase
                                    && r.scope.applies(&self.state, controller, s)
                                    && self.state.rider_holds(r, CardId(*i as u32))
                            })
                        })
                })
                .map(|(i, _)| Pending::Rider(CardId(i as u32))),
        );
        let order = self.state.clockwise_from(s);
        self.flow = Flow::Abilities {
            window: OrderedWindow::new(order),
            pending,
        };
    }

    /// A minion is burned: it leaves play with the cards on it burned
    /// (spec §1.12.1, to their owners' ash heaps), an ally's card with them; "after a ready minion
    /// is burned" riders fire if it was ready (a minion burned from torpor
    /// or from a contest is not).
    pub(crate) fn burn_minion(&mut self, m: MinionId) {
        let was_ready = self.state.minions[m.index()].ready();
        self.about_to_leave_ready(&[m]);
        {
            let v = &mut self.state.minions[m.index()];
            v.in_play = false;
            v.blood = 0;
            v.corruption.clear();
            // A burned vampire goes to its owner's ash heap, whoever
            // controls it (an ally's card goes with the cards on it, below).
            if let Some(card) = v.crypt_card() {
                let owner = v.origin.or(v.temp_from).unwrap_or(v.owner);
                self.state.seats[owner.index()].ash_crypt.push(card);
            }
        }
        // The cards on it, and an ally's own card, to their owners' ash
        // heaps.
        for ci in 0..self.state.cards.len() {
            let c = &self.state.cards[ci];
            if c.host == Some(m) || c.body == Some(m) {
                self.burn_card_in_play(CardId(ci as u32));
            }
        }
        if was_ready {
            self.leave_riders(Hook::ReadyMinionBurnedOrTorpor, &[m]);
        }
    }

    /// Minions go to torpor (combat damage): the cards on them stay; the
    /// hosts' own riders (Guardian Angel, Fame) and "after a ready minion
    /// is sent to torpor" riders fire at once (rules-qa 6.4).
    /// An ally has no torpor: sent there, it is burned instead (advanced
    /// rules 5.3.3).
    pub(crate) fn minions_to_torpor(&mut self, ms: &[MinionId]) {
        let (allies, ms): (Vec<MinionId>, Vec<MinionId>) = ms
            .iter()
            .partition(|m| self.state.minions[m.index()].is_ally());
        for m in allies {
            self.burn_minion(m);
        }
        let ms = &ms[..];
        self.about_to_leave_ready(ms);
        for &m in ms {
            self.state.minions[m.index()].in_torpor = true;
        }
        self.leave_riders(Hook::HostToTorpor, ms);
        self.leave_riders(Hook::ReadyMinionBurnedOrTorpor, ms);
    }

    /// Unlock a card in play — an ally's own card unlocks the ally.
    pub(crate) fn unlock_card(&mut self, id: CardId) {
        let barred = self.action.as_ref().is_some_and(|a| a.no_unlock);
        match self.state.cards[id.index()].body {
            Some(_) if barred => {}
            Some(m) => self.state.minions[m.index()].locked = false,
            None => self.state.cards[id.index()].locked = false,
        }
    }

    /// Lock a card in play — an ally's own card locks the ally.
    pub(crate) fn lock_card(&mut self, id: CardId) {
        match self.state.cards[id.index()].body {
            Some(m) => self.state.minions[m.index()].locked = true,
            None => self.state.cards[id.index()].locked = true,
        }
    }

    /// Burn a card in play: to its owner's ash heap with the cards on it
    /// (LSJ 20021128), its layers dropped.
    pub(crate) fn burn_card_in_play(&mut self, id: CardId) {
        // An ally's card burned is the ally burned (its card with it).
        if let Some(m) = self.state.cards[id.index()].body {
            if self.state.minions[m.index()].in_play {
                self.burn_minion(m);
                return;
            }
        }
        let c = &mut self.state.cards[id.index()];
        if !c.in_play {
            return;
        }
        c.in_play = false;
        let (owner, card) = (c.owner, c.card);
        let stored = std::mem::take(&mut c.stored);
        // A lingering effect ends; its card is already in the ash heap.
        if c.effect_only {
            self.state
                .layers
                .retain(|l| l.scope == crate::cards::Scope::Action || l.source != id);
            return;
        }
        let ash = &mut self.state.seats[owner.index()].ash;
        ash.push(card);
        ash.extend(stored);
        self.state
            .layers
            .retain(|l| l.scope == crate::cards::Scope::Action || l.source != id);
    }

    /// A card in play shuffled into its owner's library (Aranthebes, The
    /// Immortal); the cards stored on it to the ash heap.
    pub(crate) fn shuffle_into_library(&mut self, id: CardId) {
        let c = &mut self.state.cards[id.index()];
        if !c.in_play {
            return;
        }
        c.in_play = false;
        let (owner, card) = (c.owner, c.card);
        let stored = std::mem::take(&mut c.stored);
        let seat = &mut self.state.seats[owner.index()];
        seat.ash.extend(stored);
        seat.library.push(card);
        self.rng
            .shuffle(&mut self.state.seats[owner.index()].library);
        self.state
            .layers
            .retain(|l| l.scope == crate::cards::Scope::Action || l.source != id);
    }

    /// The active Methuselah passed out of a phase: the impulse goes round
    /// the other seats (rules-qa 7.9) before the phase ends.
    pub(crate) fn open_phase_rotation(&mut self, phase: Flow) {
        let mut window = OrderedWindow::new(self.state.clockwise_from(self.state.active));
        window.pass();
        self.flow = Flow::PhaseRotation {
            resume: Box::new(phase),
            window,
        };
    }

    /// Frontal Assault: "During your influence phase, burn this card and
    /// burn 1 pool for each ready minion controlled by your prey" — a
    /// mandatory effect of the phase, resolved when its controller chooses
    /// but before they pass (owner, 2026-09-26: rules-qa 28.9): the cards
    /// still due.
    pub(crate) fn influence_burns_due(&self) -> Vec<CardId> {
        let s = self.state.active;
        (0..self.state.cards.len())
            .map(|ci| CardId(ci as u32))
            .filter(|&id| {
                let c = &self.state.cards[id.index()];
                c.present()
                    && self.state.card_controller(id) == s
                    && crate::cards::lookup(c.card).is_some_and(|d| {
                        d.modes[c.mode as usize]
                            .effects
                            .contains(&crate::cards::Effect::BurnAtInfluencePerPreyReady)
                    })
            })
            .collect()
    }

    /// Resolve one Frontal Assault's influence-phase burn, the prey's ready
    /// minions counted now. True if it ousted the active Methuselah (the
    /// turn passed).
    pub(crate) fn influence_burn(&mut self, id: CardId) -> bool {
        let s = self.state.active;
        let prey = self.state.prey(s);
        let loss = self
            .state
            .minions_of(prey)
            .filter(|m| self.state.minions[m.index()].ready())
            .count() as i32;
        self.burn_card_in_play(id);
        self.state.seats[s.index()].pool -= loss;
        if loss > 0 && self.state.seats[s.index()].pool <= 0 {
            self.oust_and_pass(s);
            return true;
        }
        false
    }

    /// Everyone passed: the phase ends.
    pub(crate) fn close_phase(&mut self, phase: Flow) {
        match phase {
            Flow::MasterPhase => {
                self.state.phase_actions = 0;
                self.state.optional_action = false;
                self.flow = Flow::MinionPhase
            }
            Flow::MinionPhase => {
                // "During your influence phase" abilities: once per phase.
                self.state.phase_uses.clear();
                self.flow = Flow::Influence {
                    left: transfers_for_turn(self.state.turn)
                        + self.state.transfer_bonus(self.state.active),
                }
            }
            Flow::Influence { .. } => self.end_influence(),
            Flow::Discard => self.end_turn(),
            other => unreachable!("not a phase: {other:?}"),
        }
    }

    /// End of the influence phase: on to the discard phase (one discard
    /// phase action; events are a later slice).
    pub(crate) fn end_influence(&mut self) {
        let s = self.state.active;
        // "Do not replace until your next discard phase": replaced as the
        // phase begins, ahead of anything else in it (spec §1.9.2).
        let held = std::mem::take(&mut self.state.seats[s.index()].held.discard);
        if held > 0 {
            self.replace(s, held as usize, None);
        }
        let printed = self.state.printed_discard_actions(s);
        self.state.seats[s.index()].dpa = (1 + printed).max(0) as u8;
        self.state.seats[s.index()].granted_dpa = 0;
        // "Unlock this vampire during the next discard phase" (Feral Hound).
        for m in &mut self.state.minions {
            if std::mem::take(&mut m.unlock_at_discard) {
                m.locked = false;
            }
        }
        self.flow = Flow::Discard;
    }

    /// End of the discard phase: "until the end of the turn" effects end
    /// (the hand size bonus, LSJ 20100812; cards over size stay), and the
    /// turn passes.
    pub(crate) fn end_turn(&mut self) {
        let s = self.state.active;
        // Temporary control "until the end of your turn" (Puppet Master
        // [DOM]) ends: the minion returns as it is, in torpor included (LSJ
        // 20010119) — removed from the game if its Methuselah was ousted
        // meanwhile (ANK 20181110).
        for i in 0..self.state.minions.len() {
            let m = &self.state.minions[i];
            let Some(back) = m.temp_from.filter(|_| m.owner == s && m.in_play) else {
                continue;
            };
            self.state.minions[i].temp_from = None;
            if self.state.seats[back.index()].ousted {
                self.remove_minion(MinionId(i as u32));
            } else {
                self.state.minions[i].owner = back;
            }
        }
        self.state.seats[s.index()].hand_bonus = 0;
        self.next_turn();
    }

    /// Pool reached 0 on the seat's own turn (a transfer, a crypt draw, or
    /// contest upkeep): ousted at once, and the turn passes.
    pub(crate) fn oust_and_pass(&mut self, s: SeatId) {
        self.oust(s);
        if self.state.living() > 1 {
            self.next_turn();
        }
    }

    pub(crate) fn oust(&mut self, s: SeatId) {
        self.oust_batch(&[s]);
    }

    /// Oust these Methuselahs at once (a referendum's simultaneous pool
    /// losses, spec §6.5.4): each predator scores the victory point even if
    /// ousted in the same batch; the 6 pool goes only to surviving
    /// predators, and only for their own prey.
    pub(crate) fn oust_batch(&mut self, seats: &[SeatId]) {
        let preds: Vec<(SeatId, SeatId)> =
            seats.iter().map(|&s| (s, self.state.predator(s))).collect();
        for &s in seats {
            self.remove_seat(s);
        }
        for (_, pred) in &preds {
            self.state.seats[pred.index()].half_vps += 2;
            if !seats.contains(pred) {
                self.state.seats[pred.index()].pool += 6;
                // "If your prey is ousted, you gain 4 additional pool"
                // (Khabar: Glory), per copy.
                let extra: i32 = self
                    .state
                    .cards
                    .iter()
                    .enumerate()
                    .filter(|&(i, c)| {
                        c.present() && self.state.card_controller(CardId(i as u32)) == *pred
                    })
                    .filter_map(|(_, c)| cards::lookup(c.card))
                    .flat_map(|d| d.riders.iter().filter(|r| r.hook == Hook::PreyOusted))
                    .flat_map(|r| r.effects.iter())
                    .map(|e| match *e {
                        cards::Effect::GainPool(n) => n as i32,
                        other => unreachable!("not a prey-ousted rider effect: {other:?}"),
                    })
                    .sum();
                self.state.seats[pred.index()].pool += extra;
            }
        }
        match self.state.living() {
            // The last Methuselah standing gains one additional VP.
            1 => {
                let survivor = (0..self.state.seats.len())
                    .find(|&i| !self.state.seats[i].ousted)
                    .expect("one seat must survive");
                self.state.seats[survivor].half_vps += 2;
                self.flow = Flow::GameOver;
            }
            // Everyone ousted at once (rules-qa 6.9): the prey VPs stand,
            // nobody is last standing; the game win, if any, follows from
            // the VPs (`GameState::game_win`).
            0 => self.flow = Flow::GameOver,
            _ => {}
        }
    }

    /// A minion removed from the game (not burned: nothing triggers), the
    /// cards on it with it — burned to their owners' ash heaps if another
    /// Methuselah's (as an ousted Methuselah's minions, spec §1.10).
    pub(crate) fn remove_minion(&mut self, m: MinionId) {
        self.about_to_leave_ready(&[m]);
        let v = &mut self.state.minions[m.index()];
        v.in_play = false;
        v.corruption.clear();
        let owner = v.owner;
        for ci in 0..self.state.cards.len() {
            let c = &self.state.cards[ci];
            if c.in_play && c.host == Some(m) {
                if c.owner == owner {
                    self.state.cards[ci].in_play = false;
                } else {
                    self.burn_card_in_play(CardId(ci as u32));
                }
            }
        }
    }

    /// Rulebook "Ending the Game": all the cards the ousted Methuselah
    /// controls are removed from the game — with the layers they carried.
    fn remove_seat(&mut self, s: SeatId) {
        {
            let seat = &mut self.state.seats[s.index()];
            seat.ousted = true;
            seat.pool = 0;
            seat.has_edge = false;
            seat.crypt.clear();
            seat.uncontrolled.clear();
            seat.library.clear();
            seat.hand.clear();
            seat.ash.clear();
        }
        // An oust removes what the Methuselah controlled (spec §6.1.2):
        // their minions and the cards they control — their master cards
        // wherever they sit, the minion cards on their minions — removed,
        // not burned (§1.10: nothing triggers). Another Methuselah's card
        // on a removed minion is burned to its owner's ash heap (owner,
        // 2026-09-06).
        let leaving: Vec<MinionId> = self.state.minions_of(s).collect();
        self.about_to_leave_ready(&leaving);
        for m in &mut self.state.minions {
            if m.owner == s {
                m.in_play = false;
                m.corruption.clear();
            }
        }
        for ci in 0..self.state.cards.len() {
            let id = CardId(ci as u32);
            let c = &self.state.cards[ci];
            if !c.in_play {
                continue;
            }
            if self.state.card_controller(id) == s {
                let (owner, card, effect_only) = (c.owner, c.card, c.effect_only);
                self.state.cards[ci].in_play = false;
                // A lingering effect's card is already in the ash heap.
                if owner != s && !effect_only {
                    self.state.seats[owner.index()].removed.push(card);
                }
            } else if c
                .host
                .is_some_and(|h| self.state.minions[h.index()].owner == s)
            {
                self.burn_card_in_play(id);
            }
        }
        let cards = &self.state.cards;
        self.state
            .layers
            .retain(|l| l.scope == crate::cards::Scope::Action || cards[l.source.index()].in_play);
    }

    pub(crate) fn next_turn(&mut self) {
        if self.state.turn >= TURN_LIMIT {
            // Time limit (tournament rules 3.7): 0.5 VP per survivor.
            for seat in &mut self.state.seats {
                if !seat.ousted {
                    seat.half_vps += 1;
                }
            }
            self.flow = Flow::GameOver;
            return;
        }
        // An action cannot outlive the turn (the acting Methuselah ousted
        // by its own referendum or a cost): its scoped state goes with it.
        if self.action.take().is_some() {
            self.release_action_holds();
        }
        self.combat = None;
        self.state.combatants = None;
        self.referendum = None;
        self.state
            .layers
            .retain(|l| l.scope != crate::cards::Scope::Action);
        self.state.active = self.state.prey(self.state.active);
        self.start_turn();
    }

    fn start_turn(&mut self) {
        let s = self.state.active;
        self.state.turn += 1;
        self.state.seats[s.index()].turns_taken += 1;
        for seat in &mut self.state.seats {
            seat.forbidden_political.clear();
        }
        // Delayed replacements ("do not replace until your next unlock
        // phase") are drawn before unlocking (ANK 20200129).
        let held = std::mem::take(&mut self.state.seats[s.index()].held.unlock);
        self.state.phase_uses.clear();
        self.state.turn_uses.clear();
        self.state.veil_superior = false;
        self.replace(s, held as usize, None);
        // Veil the Legions lasts the turn.
        for seat in &mut self.state.seats {
            seat.veil_left = 0;
        }
        for m in &mut self.state.minions {
            // Per-turn NRA flags reset each new turn.
            m.bled_this_turn = false;
            m.political_this_turn = false;
            m.expelled = false;
            m.hunting_ground_fed = false;
            m.acted.clear();
            m.recruited = false;
            m.recruits_this_turn = 0;
            m.must_hunt = false;
            // Unlock phase: the active seat's minions unlock (torpor included).
            // Stolen Police Cruiser: "does not unlock as normal during their
            // next unlock phase".
            if m.owner == s && m.in_play {
                m.played_since_unlock.clear();
            }
        }
        for i in 0..self.state.minions.len() {
            let id = MinionId(i as u32);
            let m = &self.state.minions[i];
            if m.owner != s || !m.in_play {
                continue;
            }
            if m.skip_unlock {
                self.state.minions[i].skip_unlock = false;
                continue;
            }
            // "Does not unlock as normal" (Puppet Master [DOM]).
            if self.state.does_not_unlock(id) {
                continue;
            }
            // "Burns 1 counter from this card instead of unlocking as normal"
            // (Touch of Oblivion): a substitution, also for an unlocked
            // minion (advanced rules §5.2.2).
            let substitutes = self.state.counter_instead_of_unlock(id);
            if substitutes.is_empty() {
                self.state.minions[i].locked = false;
                continue;
            }
            for c in substitutes {
                let card = &mut self.state.cards[c.index()];
                card.counters = card.counters.saturating_sub(1);
                if card.counters == 0 {
                    self.burn_card_in_play(c);
                }
            }
        }
        // … and the cards they control.
        for ci in 0..self.state.cards.len() {
            if self.state.cards[ci].in_play && self.state.card_controller(CardId(ci as u32)) == s {
                self.state.cards[ci].locked = false;
            }
        }
        self.start_unlock_effects();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypt::{Sect, Title, CRYPT};
    use crate::decision::ActionKind;
    use crate::state::find_card;

    /// Untitled vampires for generic fixtures.
    fn plain(i: usize) -> u16 {
        find_card(["Abraham DuSable", "Abraham Mellon", "Abaddon"][i])
    }

    /// 3 seats. Seat 0 has two ready vampires (`MinionId(0)`, `MinionId(1)`)
    /// so `run()` always halts at seat 0's minion phase instead of eliding
    /// through to the next turn; seat 1 has one (`MinionId(2)`).
    fn bare() -> Engine {
        let mut e = Engine::bare(3);
        e.add_minion(SeatId(0), plain(0), 5, false);
        e.add_minion(SeatId(0), plain(1), 5, false);
        e.add_minion(SeatId(1), plain(2), 5, false);
        e
    }

    fn act(minion: MinionId, action: ActionKind) -> Choice {
        Choice::TakeAction {
            minion,
            action,
            target: None,
        }
    }

    fn request(e: &mut Engine) -> DecisionRequest {
        match e.run() {
            Step::Decision(req) => req,
            Step::Over(_) => panic!("game over while expecting a decision"),
        }
    }

    fn pick(e: &mut Engine, want: Choice) {
        let req = request(e);
        let idx = req
            .options
            .iter()
            .position(|o| *o == want)
            .unwrap_or_else(|| panic!("option {want:?} not in {:?}", req.options));
        e.choose(idx);
    }

    fn expect(e: &mut Engine, seat: SeatId, window: WindowLabel) -> DecisionRequest {
        let req = request(e);
        assert_eq!((req.seat, req.window), (seat, window), "{:?}", req.options);
        req
    }

    // A blocked rush continued as if unblocked (Form of Mist's shape, set
    // directly: no rush vampire has Protean) reaches its resolution and
    // enters combat with its target; the unblocked combat's after-combat
    // window offers no continuation and leads to after resolution
    // (rules-qa 17.3).
    #[test]
    fn a_blocked_rush_continued_as_if_unblocked_enters_combat() {
        let textless = |nth: usize| {
            (0..CRYPT.len() as u16)
                .filter(|&c| crate::cards::vampire(c).is_none() && CRYPT[c as usize].capacity >= 4)
                .nth(nth)
                .expect("textless vampire")
        };
        let mut e = Engine::bare(2);
        e.state.turn = 4;
        let theo = e.add_minion(SeatId(0), find_card("Theo Bell"), 5, false);
        e.add_minion(SeatId(0), textless(0), 4, false);
        let target = e.add_minion(SeatId(1), textless(1), 4, false);
        let blocker = e.add_minion(SeatId(1), textless(2), 4, false);
        pick(
            &mut e,
            Choice::UseAbility {
                source: crate::cards::AbilitySource::Vampire(theo),
                ability: 0,
                actor: Some(theo),
                target: crate::cards::Target::Minion(target),
            },
        );
        pick(&mut e, Choice::DeclareBlock { minion: blocker });
        e.action.as_mut().expect("action").continuation = Some((0, 0));
        let (mut continued, mut offered_after_rush) = (0, 0);
        loop {
            let req = request(&mut e);
            if req.window == WindowLabel::MinionPhase && req.seat == SeatId(0) {
                break;
            }
            if req.options.contains(&Choice::ContinueAction) {
                if e.action.as_ref().expect("action").resolved {
                    offered_after_rush += 1;
                } else {
                    continued += 1;
                    let i = req
                        .options
                        .iter()
                        .position(|o| *o == Choice::ContinueAction);
                    e.choose(i.expect("continue"));
                    continue;
                }
            }
            let i = req
                .options
                .iter()
                .position(|o| matches!(o, Choice::Pass | Choice::HandStrike))
                .unwrap_or(0);
            e.choose(i);
        }
        assert_eq!((continued, offered_after_rush), (1, 0));
        // Both combats fought: the blocker's, then the target's.
        assert!(e.state.minions[blocker.index()].blood < 4);
        assert!(e.state.minions[target.index()].blood < 4);
        assert_eq!(e.last_outcome, Some(crate::actions::Outcome::Successful));
    }

    #[test]
    fn crypt_data_is_complete() {
        assert_eq!(CRYPT.len(), 200);
        assert_eq!(PRECONS.len(), 14);
        for (name, list) in PRECONS.iter() {
            let n: u32 = list.iter().map(|&(_, k)| k as u32).sum();
            assert_eq!(n, 12, "{name}");
        }
        let horst = &CRYPT[find_card("Horst von Brühl") as usize];
        assert_eq!(horst.title, Title::Prince("Mannheim"));
        assert_eq!(horst.sect, Sect::Camarilla);
        let aline = &CRYPT[find_card("Aline Gädeke") as usize];
        assert_eq!(aline.title, Title::Baron("Mannheim"));
        assert_eq!(aline.sect, Sect::Anarch);
    }

    #[test]
    fn new_game_deals_four_crypt_cards_from_a_precon() {
        let e = Engine::new(3, 4);
        for s in &e.state.seats {
            assert_eq!(s.uncontrolled.len(), 4);
            assert_eq!(s.crypt.len(), 8);
        }
    }

    #[test]
    fn hunt_has_stealth_so_its_block_window_elides() {
        let mut e = bare();
        e.state.minions[0].blood = 3;
        let logged = e.log.len();
        pick(&mut e, act(MinionId(0), ActionKind::Hunt));
        // No seat is ever asked: the +1 stealth action resolves at once and
        // control returns to seat 0's minion phase.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged + 1);
        assert_eq!(e.state.minions[0].blood, 4);
    }

    #[test]
    fn leave_torpor_has_stealth_and_pays_at_resolution() {
        let mut e = bare();
        let t = e.add_minion(SeatId(0), find_card("Alice Chen"), 3, true);
        let poor = e.add_minion(SeatId(0), find_card("Alexa Draper"), 1, true);
        let req = request(&mut e);
        assert!(req.options.contains(&act(t, ActionKind::LeaveTorpor)));
        assert!(!req.options.contains(&act(poor, ActionKind::LeaveTorpor))); // cannot pay
        let logged = e.log.len();
        pick(&mut e, act(t, ActionKind::LeaveTorpor));
        // +1 stealth: no block window; the cost is paid at resolution.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged + 1);
        let m = &e.state.minions[t.index()];
        assert!((!m.in_torpor, m.blood, m.locked) == (true, 1, true));
    }

    #[test]
    fn mandatory_hunt_gates_the_whole_minion_phase() {
        let mut e = bare();
        e.state.minions[0].blood = 0;
        e.state.minions[1].blood = 0;
        // Two starving vampires: only their hunts, in either order, no Pass.
        let req = request(&mut e);
        assert_eq!(
            req.options,
            vec![
                act(MinionId(0), ActionKind::Hunt),
                act(MinionId(1), ActionKind::Hunt)
            ]
        );
        pick(&mut e, act(MinionId(1), ActionKind::Hunt));
        // One left: a single mandatory option is elided, not offered.
        let logged = e.log.len();
        let req = request(&mut e);
        assert_eq!(e.log.len(), logged);
        assert_eq!(e.state.minions[0].blood, 1);
        assert!(e.state.minions[0].locked);
        assert!(req.options.contains(&Choice::Pass));
    }

    #[test]
    fn rescue_of_another_controllers_vampire_is_directed_at_them() {
        let mut e = bare();
        let t = e.add_minion(SeatId(2), find_card("Alice Chen"), 3, true);
        let guard = e.add_minion(SeatId(2), find_card("Alexa Draper"), 5, false);
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Rescue,
                target: Some(t),
            },
        );
        // Directed at seat 2 (not the prey): seat 2 alone may block.
        let req = expect(&mut e, SeatId(2), WindowLabel::ActionBlock);
        assert!(req
            .options
            .contains(&Choice::DeclareBlock { minion: guard }));
        assert!(!req.options.contains(&Choice::DeclareBlock { minion: t })); // torpor cannot block
        pick(&mut e, Choice::Pass);
        // Cost split at resolution: 2/1/0 from the acting vampire.
        let req = expect(&mut e, SeatId(0), WindowLabel::RescueCost);
        assert_eq!(req.options.len(), 3);
        pick(&mut e, Choice::PayRescue { from_acting: 0 });
        assert!(!e.state.minions[t.index()].in_torpor);
        assert_eq!(e.state.minions[t.index()].blood, 1);
        assert_eq!(e.state.minions[0].blood, 5);
        assert!(!e.state.minions[t.index()].locked);
    }

    #[test]
    fn rescue_of_own_vampire_has_stealth_and_forced_split_elides() {
        let mut e = bare();
        let t = e.add_minion(SeatId(0), find_card("Alice Chen"), 1, true);
        e.state.minions[0].blood = 1;
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Rescue,
                target: Some(t),
            },
        );
        // +1 stealth, no block window; only split 1/1 is payable → elided.
        // The rescued vampire comes out ready at 0 blood, so its mandatory
        // hunt is elided too, straight away.
        let logged = e.log.len();
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged);
        assert_eq!(e.state.minions[0].blood, 0);
        assert!(!e.state.minions[t.index()].in_torpor);
        assert_eq!(e.state.minions[t.index()].blood, 1);
        assert!(e.state.minions[t.index()].locked);
    }

    #[test]
    fn diablerie_moves_blood_burns_victim_and_calls_the_blood_hunt() {
        let mut e = bare();
        let victim = e.add_minion(SeatId(1), find_card("Alice Chen"), 4, true);
        // Seat 1's ready vampire is a Prince: 2 votes.
        let prince = e.add_minion(SeatId(1), find_card("Horst von Brühl"), 5, false);
        e.state.minions[0].blood = 6;
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Diablerie,
                target: Some(victim),
            },
        );
        pick(&mut e, Choice::Pass); // seat 1 declines to block
                                    // Polling: seat 0 has no source (elided), seat 1 is asked.
        expect(&mut e, SeatId(1), WindowLabel::BloodHuntPolling);
        assert!(!e.state.minions[victim.index()].in_play);
        assert_eq!(e.state.minions[0].blood, 8); // 6 + 4 capped at capacity 8
        pick(
            &mut e,
            Choice::CastVotes {
                minion: prince,
                favor: true,
            },
        );
        // Full quiet rotation closes; 2 for > 0 against: diablerist burned.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[0].in_play);
    }

    #[test]
    fn blood_hunt_with_no_vote_source_fails_silently() {
        let mut e = bare();
        let victim = e.add_minion(SeatId(1), find_card("Alice Chen"), 0, true);
        let logged = e.log.len();
        pick(
            &mut e,
            Choice::TakeAction {
                minion: MinionId(0),
                action: ActionKind::Diablerie,
                target: Some(victim),
            },
        );
        pick(&mut e, Choice::Pass);
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged + 2); // the whole referendum elided
        assert!(e.state.minions[0].in_play);
    }

    #[test]
    fn transfers_ramp_by_game_turn_not_by_seat() {
        assert_eq!(transfers_for_turn(1), 1);
        assert_eq!(transfers_for_turn(2), 2);
        assert_eq!(transfers_for_turn(3), 3);
        assert_eq!(transfers_for_turn(4), 4);
        assert_eq!(transfers_for_turn(9), 4);
        // Seat 1's first influence phase is game turn 2: two transfers.
        // (Empty hands keep the master and discard phases elided.)
        let mut e = Engine::new(7, 3);
        for seat in &mut e.state.seats {
            seat.library.clear();
            seat.hand.clear();
        }
        let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert!(!req.options.contains(&Choice::DrawCrypt)); // needs 4 transfers
        pick(&mut e, Choice::Transfer { slot: 0 });
        expect(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
        pick(&mut e, Choice::Transfer { slot: 0 });
        let req = expect(&mut e, SeatId(1), WindowLabel::InfluenceTransfers);
        assert!(req.options.contains(&Choice::Transfer { slot: 1 }));
        assert!(!req.options.contains(&Choice::Withdraw { slot: 0 })); // needs 2 transfers
    }

    #[test]
    fn crypt_draw_costs_four_transfers_and_one_pool() {
        let mut e = bare();
        e.state.seats[0].crypt = vec![plain(2), find_card("Alice Chen")];
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // end minion phase: 4 transfers
        pick(&mut e, Choice::DrawCrypt);
        assert_eq!(e.state.seats[0].pool, START_POOL - 1);
        assert_eq!(e.state.seats[0].crypt, vec![plain(2)]);
        assert_eq!(
            e.state.seats[0].uncontrolled[0].card,
            find_card("Alice Chen")
        );
        // No transfers left: only Pass remains and the phase elides away.
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
    }

    #[test]
    fn influence_out_is_explicit_withdraw_costs_two_and_no_self_contest() {
        let mut e = bare();
        // Seat 0 already controls Abraham DuSable (MinionId(0)): a second
        // copy cannot be moved out (no voluntary self-contest).
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: plain(0),
            counters: CRYPT[plain(0) as usize].capacity,
            banished: None,
        });
        let alice = find_card("Alice Chen");
        let cap = CRYPT[alice as usize].capacity;
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: alice,
            counters: cap,
            banished: None,
        });
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // end minion phase
        let req = expect(&mut e, SeatId(0), WindowLabel::InfluenceTransfers);
        assert!(!req.options.contains(&Choice::InfluenceOut { slot: 0 }));
        assert!(req.options.contains(&Choice::InfluenceOut { slot: 1 }));
        assert!(req.options.contains(&Choice::Withdraw { slot: 1 }));
        // Full, but more can go on (rules-qa 28.3).
        assert!(req.options.contains(&Choice::Transfer { slot: 1 }));
        pick(&mut e, Choice::Withdraw { slot: 1 });
        assert_eq!(e.state.seats[0].pool, START_POOL + 1);
        let req = request(&mut e);
        assert!(!req.options.contains(&Choice::InfluenceOut { slot: 1 }));
        pick(&mut e, Choice::Transfer { slot: 1 });
        pick(&mut e, Choice::InfluenceOut { slot: 1 });
        let id = MinionId(e.state.minions.len() as u32 - 1);
        let m = &e.state.minions[id.index()];
        assert_eq!(m.name(), "Alice Chen");
        assert_eq!(
            (m.owner, m.blood, e.state.title_votes_of(id), m.in_torpor),
            (SeatId(0), cap, 2, false)
        );
        assert_eq!(e.state.seats[0].uncontrolled.len(), 1);
    }

    /// Rules-qa 28.3 (owner, 2026-09-26): any amount can go on an
    /// uncontrolled vampire; the excess drains off to the bank only as it
    /// is influenced out.
    #[test]
    fn transfers_go_over_capacity_and_drain_on_influence_out() {
        let mut e = bare();
        let alice = find_card("Alice Chen");
        let cap = CRYPT[alice as usize].capacity;
        e.state.seats[0].uncontrolled.push(UncontrolledSlot {
            card: alice,
            counters: cap,
            banished: None,
        });
        e.state.turn = 4;
        pick(&mut e, Choice::Pass);
        pick(&mut e, Choice::Transfer { slot: 0 });
        pick(&mut e, Choice::Transfer { slot: 0 });
        assert_eq!(e.state.seats[0].uncontrolled[0].counters, cap + 2);
        pick(&mut e, Choice::InfluenceOut { slot: 0 });
        let id = MinionId(e.state.minions.len() as u32 - 1);
        assert_eq!(e.state.minions[id.index()].blood, cap);
    }

    #[test]
    fn card_contest_pay_yield_and_return() {
        // Seat 1 brings out Abraham Mellon while seat 0 has one (MinionId(1)):
        // both go face down; seat 0 pays, seat 1 yields, seat 0's returns.
        let mut e = bare();
        e.state.seats[1].uncontrolled.push(UncontrolledSlot {
            card: plain(1),
            counters: CRYPT[plain(1) as usize].capacity,
            banished: None,
        });
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // seat 0 minion phase; its influence elides
        pick(&mut e, Choice::Pass); // seat 1 minion phase
        pick(&mut e, Choice::InfluenceOut { slot: 0 });
        let copy = MinionId(3);
        assert!(e.state.minions[1].contested && e.state.minions[copy.index()].contested);
        // Seat 2 has nothing and elides; seat 0's unlock phase: upkeep.
        let item = Contest::Card(MinionId(1));
        let req = expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        assert_eq!(
            req.options,
            vec![Choice::PayContest(item), Choice::YieldContest(item)]
        );
        pick(&mut e, Choice::PayContest(item));
        assert_eq!(e.state.seats[0].pool, START_POOL - 1);
        assert!(e.state.minions[1].contested);
        // Contested copies are out of play: seat 0 cannot act with it.
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!req
            .options
            .iter()
            .any(|o| matches!(o, Choice::TakeAction { minion, .. } if *minion == MinionId(1))));
        pick(&mut e, Choice::Pass);
        // Seat 1 yields: its copy is burned.
        expect(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        pick(&mut e, Choice::YieldContest(Contest::Card(copy)));
        assert!(!e.state.minions[copy.index()].in_play);
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 0's next unlock: no rival left, the copy returns for free.
        let req = expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[1].contested);
        assert!(req.options.contains(&act(MinionId(1), ActionKind::Hunt)));
    }

    #[test]
    fn title_contest_prince_and_baron_of_one_city() {
        let mut e = bare();
        let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 4, false);
        let aline = e.add_minion(SeatId(1), find_card("Aline Gädeke"), 2, false);
        e.stage_contests(aline);
        assert!(e.state.minions[horst.index()].title_contested);
        assert_eq!(e.state.title_votes_of(horst), 0);
        assert_eq!(e.state.title_votes_of(aline), 0);
        e.state.turn = 4;
        pick(&mut e, Choice::Pass); // seat 0 minion phase
                                    // Seat 1's unlock: Aline pays 1 blood.
        expect(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        pick(&mut e, Choice::PayContest(Contest::Title(aline)));
        assert_eq!(e.state.minions[aline.index()].blood, 1);
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 0's unlock: Horst pays too.
        expect(&mut e, SeatId(0), WindowLabel::UnlockAbilities);
        pick(&mut e, Choice::PayContest(Contest::Title(horst)));
        assert_eq!(e.state.minions[horst.index()].blood, 3);
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 1's unlock: Aline yields — titleless for the rest of the game.
        expect(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        pick(&mut e, Choice::YieldContest(Contest::Title(aline)));
        let a = &e.state.minions[aline.index()];
        assert!(a.title_lost && !a.title_contested && a.in_play);
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        pick(&mut e, Choice::Pass);
        // Seat 0's unlock: no rival left, Horst acquires the title for free.
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert!(!e.state.minions[horst.index()].title_contested);
        assert_eq!(e.state.title_votes_of(horst), 2);
    }

    #[test]
    fn title_contest_torpor_and_no_blood_force_a_yield() {
        let mut e = bare();
        let horst = e.add_minion(SeatId(0), find_card("Horst von Brühl"), 0, false);
        let aline = e.add_minion(SeatId(1), find_card("Aline Gädeke"), 3, true);
        e.stage_contests(aline);
        e.state.minions[0].blood = 0; // keep Horst from being the only starving one
        e.state.turn = 4;
        // Seat 0's minion phase: mandatory hunts for the two starving
        // vampires; take Horst's first.
        pick(&mut e, act(horst, ActionKind::Hunt));
        pick(&mut e, Choice::Pass);
        // Seat 1's unlock: Aline in torpor must yield (elided).
        let logged = e.log.len();
        expect(&mut e, SeatId(1), WindowLabel::MinionPhase);
        assert_eq!(e.log.len(), logged);
        assert!(e.state.minions[aline.index()].title_lost);
        // Horst (now at 1 blood, but that is beside the point: no rival) is
        // titled again at his next unlock phase.
        pick(&mut e, Choice::Pass);
        expect(&mut e, SeatId(0), WindowLabel::MinionPhase);
        assert_eq!(e.state.title_votes_of(horst), 2);
    }

    #[test]
    fn time_limit_awards_half_a_vp_per_survivor() {
        let mut e = bare();
        e.state.turn = TURN_LIMIT;
        e.state.seats[2].ousted = true;
        e.state.seats[0].half_vps = 2;
        pick(&mut e, Choice::Pass);
        match e.run() {
            Step::Over(v) => assert_eq!(v, vec![3, 1, 0]),
            Step::Decision(_) => panic!("expected time-out"),
        }
    }

    // rules-qa 6.9: the game win needs 2 VP and a strict lead, ousted or
    // not; ties are draws.
    #[test]
    fn game_win_needs_two_vps_and_a_strict_lead() {
        let mut e = Engine::bare(4);
        let set = |e: &mut Engine, vps: [u32; 4]| {
            for (s, v) in e.state.seats.iter_mut().zip(vps) {
                s.half_vps = v;
            }
        };
        set(&mut e, [6, 4, 0, 0]); // 3 VP (ousted) vs 2: the 3 wins
        e.state.seats[0].ousted = true;
        assert_eq!(e.state.game_win(), Some(SeatId(0)));
        set(&mut e, [4, 4, 0, 0]); // two at 2 VP: a draw
        assert_eq!(e.state.game_win(), None);
        set(&mut e, [2, 2, 2, 2]); // all at 1 VP: a draw
        assert_eq!(e.state.game_win(), None);
        set(&mut e, [3, 2, 1, 0]); // 1.5 VP leads but is under 2
        assert_eq!(e.state.game_win(), None);
        set(&mut e, [4, 2, 0, 0]);
        assert_eq!(e.state.game_win(), Some(SeatId(0)));
    }

    #[test]
    fn edge_burn_is_offered_on_unlock() {
        let mut e = bare();
        e.state.seats[0].has_edge = false;
        e.state.seats[1].has_edge = true;
        // Seat 0 passes its minion phase; the empty influence phase elides
        // and the turn passes to seat 1, whose unlock effects offer the
        // Edge's pool (rules-qa 7.8: an optional unlock effect).
        pick(&mut e, Choice::Pass);
        let req = expect(&mut e, SeatId(1), WindowLabel::UnlockAbilities);
        assert_eq!(req.options, vec![Choice::Pass, Choice::BurnEdge]);
        let pool = e.state.seats[1].pool;
        pick(&mut e, Choice::BurnEdge);
        assert_eq!(e.state.seats[1].pool, pool + 1);
        assert!(!e.state.seats[1].has_edge);
    }
}
