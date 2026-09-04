//! Referendums (rulebook "The Referendum"): terms chosen stepwise by the
//! caller once the action succeeds (rules-qa 3.5), then the polling step —
//! votes and ballots as irrevocable commitments on the polling primitive
//! (sources per rules-qa 1.1: each ready vampire with votes, the Edge, one
//! political action card per Methuselah), card plays interleaved as
//! commitments of the same rotation (DESIGN §4.2) — then the tally: more
//! votes for than against passes, ties fail. The blood hunt is the
//! restricted case: no cards, no terms (rulebook "The Blood Hunt").
//!
//! The context lives on the engine beside the action so that plays resolved
//! from the as-played stack can modify it (vote bonuses, abstentions).

use crate::cards::{self, Among, Points, RefEffect, ReferendumDef, Term, Window};
use crate::crypt::CRYPT;
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, LIBRARY};
use crate::state::title_votes;
use crate::window::PollingWindow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    /// Called automatically after a diablerie; passes: the diablerist is
    /// burned.
    BloodHunt { diablerist: MinionId },
    /// Called by a political action (part of its resolution).
    Political,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefStep {
    Terms,
    Polling,
}

/// A vote source (rules-qa 1.1): atomic, commits once, all its votes one
/// way. Methuselah ballots (the Edge, a political action card) are
/// independent of each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Vampire(MinionId),
    Edge(SeatId),
    Card(SeatId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoteRecord {
    pub source: Source,
    pub seat: SeatId,
    pub favor: bool,
    pub n: u8,
    /// Canceled by an abstention (Scalpel Tongue): not tallied.
    pub canceled: bool,
}

/// Terms under construction (rulebook "The Referendum" step 1).
#[derive(Clone, Debug, Default)]
pub struct TermsState {
    /// Index of the term being chosen.
    pub term: usize,
    /// Points allocated so far, in allocation order.
    pub alloc: Vec<(SeatId, u8)>,
    /// Seats still to allocate for the current `Allocate` term.
    pub pending: Vec<SeatId>,
    /// Chosen Methuselahs.
    pub seats: Vec<SeatId>,
    pub clan: Option<&'static str>,
}

#[derive(Clone, Debug)]
pub struct ReferendumCtx {
    pub kind: RefKind,
    /// The acting Methuselah (political) or the diablerist's controller.
    pub caller: SeatId,
    pub def: Option<&'static ReferendumDef>,
    pub step: RefStep,
    pub window: PollingWindow,
    pub records: Vec<VoteRecord>,
    /// Vote bonuses granted to vampires that have not cast yet.
    pub bonus: Vec<(MinionId, u8)>,
    /// Vampires forced to abstain: they cast nothing more.
    pub abstaining: Vec<MinionId>,
    /// Methuselahs who used their one political-action-card vote.
    pub card_voted: Vec<SeatId>,
    pub terms: TermsState,
    pub canceled: bool,
}

impl ReferendumCtx {
    /// Votes a vampire can cast right now: its title (ready holder,
    /// uncontested) plus bonuses granted this referendum; none while
    /// abstaining.
    pub fn votes_of(&self, st: &crate::state::GameState, m: MinionId) -> u8 {
        let v = &st.minions[m.index()];
        if !v.ready() || self.abstaining.contains(&m) {
            return 0;
        }
        let bonus: u8 = self
            .bonus
            .iter()
            .filter(|(b, _)| *b == m)
            .map(|(_, n)| *n)
            .sum();
        title_votes(v.title()) + bonus
    }

    /// Has this vampire cast votes or ballots that still stand?
    pub fn has_cast(&self, m: MinionId) -> bool {
        self.records
            .iter()
            .any(|r| r.source == Source::Vampire(m) && !r.canceled)
    }

    /// Has any vote or ballot been cast ("before votes and ballots are
    /// cast")?
    pub fn any_cast(&self) -> bool {
        !self.records.is_empty()
    }

    fn spent(&self, source: Source) -> bool {
        self.records.iter().any(|r| r.source == source)
    }

    /// (for, against) over the standing records.
    pub fn tally(&self) -> (u32, u32) {
        self.records
            .iter()
            .filter(|r| !r.canceled)
            .fold((0, 0), |(f, a), r| {
                if r.favor {
                    (f + r.n as u32, a)
                } else {
                    (f, a + r.n as u32)
                }
            })
    }

    fn current_term(&self) -> Option<Term> {
        self.def.and_then(|d| d.terms.get(self.terms.term).copied())
    }
}

/// Clans of the V5 crypt, in crypt order (Consanguineous Boon's choice).
pub fn clans() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in CRYPT.iter() {
        if !out.contains(&c.clan) {
            out.push(c.clan);
        }
    }
    out
}

impl Engine {
    /// Open a referendum. Rotations start at the caller, clockwise (DESIGN
    /// §4.2). Terms first, if the card has any.
    pub(crate) fn start_referendum(
        &mut self,
        kind: RefKind,
        caller: SeatId,
        def: Option<&'static ReferendumDef>,
    ) {
        let order = self.state.clockwise_from(caller);
        let mut ctx = ReferendumCtx {
            kind,
            caller,
            def,
            step: RefStep::Polling,
            window: PollingWindow::new(order),
            records: Vec::new(),
            bonus: Vec::new(),
            abstaining: Vec::new(),
            card_voted: Vec::new(),
            terms: TermsState::default(),
            canceled: false,
        };
        if def.is_some_and(|d| !d.terms.is_empty()) {
            ctx.step = RefStep::Terms;
            self.begin_term(&mut ctx);
        }
        self.referendum = Some(ctx);
        self.flow = Flow::Referendum;
    }

    /// Set up the current term's state (the seats an allocation runs over,
    /// clockwise from the caller).
    fn begin_term(&self, ctx: &mut ReferendumCtx) {
        ctx.terms.pending.clear();
        if let Some(Term::Allocate { among, .. }) = ctx.current_term() {
            ctx.terms.pending = self
                .state
                .clockwise_from(ctx.caller)
                .into_iter()
                .filter(|&s| among == Among::Any || s != ctx.caller)
                .collect();
        }
    }

    fn points_of(&self, p: Points) -> u8 {
        match p {
            Points::Fixed(n) => n,
            Points::Methuselahs => self.state.living() as u8,
        }
    }

    pub(crate) fn referendum_request(&self) -> Option<DecisionRequest> {
        let ctx = self.referendum.as_ref().expect("referendum context");
        match ctx.step {
            RefStep::Terms => Some(self.terms_step_request(ctx)),
            RefStep::Polling => self.polling_request(ctx),
        }
    }

    fn terms_step_request(&self, ctx: &ReferendumCtx) -> DecisionRequest {
        let st = &self.state;
        let options = match ctx.current_term().expect("a term to choose") {
            Term::Allocate {
                points, min_seats, ..
            } => {
                let total = self.points_of(points);
                let given: u8 = ctx.terms.alloc.iter().map(|(_, n)| *n).sum();
                let rem = total - given;
                let nonzero = ctx.terms.alloc.iter().filter(|(_, n)| *n > 0).count() as u8;
                let seat = ctx.terms.pending[0];
                let after = ctx.terms.pending.len() as u8 - 1;
                // n is feasible when the rest can still be allocated with
                // at least `min_seats` Methuselahs receiving a point.
                (0..=rem)
                    .filter(|&n| {
                        let left = rem - n;
                        let nz = nonzero + (n > 0) as u8;
                        if left == 0 {
                            nz >= min_seats
                        } else {
                            after >= 1 && nz + left.min(after) >= min_seats
                        }
                    })
                    .map(|n| Choice::AllocateTo { seat, n })
                    .collect()
            }
            Term::ChooseSeats { min, max } => {
                let mut options = Vec::new();
                if ctx.terms.seats.len() as u8 >= min {
                    options.push(Choice::Pass);
                }
                if (ctx.terms.seats.len() as u8) < max {
                    for s in st.clockwise_from(ctx.caller) {
                        if !ctx.terms.seats.contains(&s) {
                            options.push(Choice::ChooseSeat(s));
                        }
                    }
                }
                options
            }
            Term::ChooseClan => clans().into_iter().map(Choice::ChooseClan).collect(),
        };
        DecisionRequest {
            seat: ctx.caller,
            window: WindowLabel::ReferendumTerms,
            options,
        }
    }

    fn polling_request(&self, ctx: &ReferendumCtx) -> Option<DecisionRequest> {
        let st = &self.state;
        let seat = ctx.window.current()?;
        let mut options = vec![Choice::Pass];
        for id in st.minions_of(seat) {
            // Torpor vampires must abstain; locked ready ones may vote;
            // contested titles carry no votes.
            if ctx.votes_of(st, id) > 0 && !ctx.spent(Source::Vampire(id)) {
                options.push(Choice::CastVotes {
                    minion: id,
                    favor: true,
                });
                options.push(Choice::CastVotes {
                    minion: id,
                    favor: false,
                });
            }
        }
        if st.seats[seat.index()].has_edge && !ctx.spent(Source::Edge(seat)) {
            options.push(Choice::CastEdge { favor: true });
            options.push(Choice::CastEdge { favor: false });
        }
        // Rulebook "Gaining Votes": the calling card's 1 vote for the acting
        // Methuselah, or a political action card burned from hand; at most
        // 1 vote from political action cards per Methuselah.
        if !ctx.card_voted.contains(&seat) {
            if ctx.kind == RefKind::Political && seat == ctx.caller {
                options.push(Choice::CastCardVote { favor: true });
                options.push(Choice::CastCardVote { favor: false });
            }
            for (hi, &card) in st.seats[seat.index()].hand.iter().enumerate() {
                if LIBRARY[card as usize]
                    .types
                    .contains(&CardType::PoliticalAction)
                {
                    options.push(Choice::BurnForVote {
                        card: hi as u8,
                        favor: true,
                    });
                    options.push(Choice::BurnForVote {
                        card: hi as u8,
                        favor: false,
                    });
                }
            }
        }
        let window = match ctx.kind {
            RefKind::BloodHunt { .. } => WindowLabel::BloodHuntPolling,
            RefKind::Political => {
                options.extend(self.play_options(seat, &[Window::Referendum], None));
                WindowLabel::ReferendumPolling
            }
        };
        Some(DecisionRequest {
            seat,
            window,
            options,
        })
    }

    pub(crate) fn referendum_apply(&mut self, choice: &Choice) {
        let mut ctx = self.referendum.take().expect("referendum context");
        match ctx.step {
            RefStep::Terms => self.terms_step_apply(&mut ctx, choice),
            RefStep::Polling => {
                if let Some(play) = self.polling_apply(&mut ctx, choice) {
                    let (card, mode, actor, target, seat) = play;
                    self.referendum = Some(ctx);
                    self.flow = Flow::Referendum;
                    self.announce_play(seat, card, mode, actor, target, Flow::Referendum);
                    return;
                }
            }
        }
        self.referendum = Some(ctx);
        self.flow = Flow::Referendum;
    }

    fn terms_step_apply(&mut self, ctx: &mut ReferendumCtx, choice: &Choice) {
        let term = ctx.current_term().expect("a term to choose");
        let done = match (term, choice) {
            (Term::Allocate { .. }, Choice::AllocateTo { seat, n }) => {
                debug_assert_eq!(ctx.terms.pending.first(), Some(seat));
                ctx.terms.pending.remove(0);
                ctx.terms.alloc.push((*seat, *n));
                ctx.terms.pending.is_empty()
            }
            (Term::ChooseSeats { .. }, Choice::ChooseSeat(s)) => {
                ctx.terms.seats.push(*s);
                false
            }
            (Term::ChooseSeats { .. }, Choice::Pass) => true,
            (Term::ChooseClan, Choice::ChooseClan(c)) => {
                ctx.terms.clan = Some(c);
                true
            }
            (t, c) => unreachable!("illegal terms choice {c:?} for {t:?}"),
        };
        if done {
            ctx.terms.term += 1;
            if ctx.current_term().is_some() {
                self.begin_term(ctx);
            } else {
                ctx.step = RefStep::Polling;
            }
        }
    }

    /// Apply a polling commitment; a card play is returned for
    /// announcement once the context is back on the engine.
    #[allow(clippy::type_complexity)]
    fn polling_apply(
        &mut self,
        ctx: &mut ReferendumCtx,
        choice: &Choice,
    ) -> Option<(u8, u8, Option<MinionId>, cards::Target, SeatId)> {
        let seat = ctx.window.current().expect("commit on closed window");
        let (source, favor, n) = match choice {
            Choice::Pass => {
                ctx.window.pass();
                return None;
            }
            Choice::CastVotes { minion, favor } => (
                Source::Vampire(*minion),
                *favor,
                ctx.votes_of(&self.state, *minion),
            ),
            Choice::CastEdge { favor } => {
                self.state.seats[seat.index()].has_edge = false;
                (Source::Edge(seat), *favor, 1)
            }
            Choice::CastCardVote { favor } => {
                ctx.card_voted.push(seat);
                (Source::Card(seat), *favor, 1)
            }
            Choice::BurnForVote { card, favor } => {
                let s = &mut self.state.seats[seat.index()];
                let c = s.hand.remove(*card as usize);
                s.ash.push(c);
                self.draw_to_hand_size(seat);
                ctx.card_voted.push(seat);
                (Source::Card(seat), *favor, 1)
            }
            Choice::Play {
                card,
                mode,
                actor,
                target,
            } => {
                // A card play is a commitment of the rotation: it reopens a
                // response opportunity for every other Methuselah.
                ctx.window.commit();
                return Some((*card, *mode, *actor, *target, seat));
            }
            other => unreachable!("illegal polling choice {other:?}"),
        };
        ctx.records.push(VoteRecord {
            source,
            seat,
            favor,
            n,
            canceled: false,
        });
        ctx.window.commit();
        None
    }

    /// The polling window closed: tally and resolve.
    pub(crate) fn referendum_advance(&mut self) {
        let ctx = self.referendum.take().expect("referendum context");
        debug_assert!(ctx.step == RefStep::Polling && ctx.window.is_closed());
        let (favor, against) = ctx.tally();
        let passed = !ctx.canceled && favor > against;
        match ctx.kind {
            RefKind::BloodHunt { diablerist } => {
                if passed {
                    self.state.minions[diablerist.index()].in_play = false;
                }
                self.flow = Flow::MinionPhase;
            }
            RefKind::Political => {
                {
                    let a = self.action.as_mut().expect("political action");
                    a.referendum_passed = passed;
                    a.margin = if passed { favor - against } else { 0 };
                }
                if passed {
                    let def = ctx.def.expect("political definition");
                    self.apply_referendum_effect(def.effect, &ctx);
                }
                self.after_referendum();
            }
        }
    }

    /// Effects of a passed referendum; pool losses are simultaneous, ousts
    /// resolved as a batch (spec §6.5.4).
    fn apply_referendum_effect(&mut self, effect: RefEffect, ctx: &ReferendumCtx) {
        let st = &self.state;
        let n_seats = st.seats.len();
        let mut loss = vec![0i32; n_seats];
        match effect {
            RefEffect::BurnAllocated => {
                for &(s, n) in &ctx.terms.alloc {
                    loss[s.index()] += n as i32;
                }
            }
            RefEffect::BurnPerMinion { locked_only } => {
                for m in &st.minions {
                    if m.present() && (!locked_only || m.locked) {
                        loss[m.owner.index()] += 1;
                    }
                }
            }
            RefEffect::BurnChosen {
                base,
                young_bonus,
                young_cap,
            } => {
                for &s in &ctx.terms.seats {
                    loss[s.index()] += base as i32;
                    if st.minions_of(s).any(|m| {
                        st.minions[m.index()].ready()
                            && st.minions[m.index()].capacity() <= young_cap
                    }) {
                        loss[s.index()] += young_bonus as i32;
                    }
                }
            }
        }
        self.burn_pool_batch(&loss);
    }

    /// Simultaneous pool losses: every Methuselah at 0 is ousted at once.
    pub(crate) fn burn_pool_batch(&mut self, loss: &[i32]) {
        let mut ousted = Vec::new();
        for (i, &n) in loss.iter().enumerate() {
            let seat = &mut self.state.seats[i];
            if seat.ousted || n <= 0 {
                continue;
            }
            seat.pool = (seat.pool - n).max(0);
            if seat.pool == 0 {
                ousted.push(SeatId(i as u8));
            }
        }
        if !ousted.is_empty() {
            self.oust_batch(&ousted);
        }
    }

    /// The referendum concluded (passed, failed or canceled): the action
    /// resumes at its after-resolution window — unless the acting
    /// Methuselah was ousted (the turn passes) or the game ended.
    fn after_referendum(&mut self) {
        if self.state.living() <= 1 {
            self.action = None;
            self.flow = Flow::GameOver;
            return;
        }
        let seat = self.action.as_ref().expect("political action").seat;
        if self.state.seats[seat.index()].ousted {
            self.action = None;
            self.next_turn();
            return;
        }
        self.after_resolution();
    }

    /// Effects of a modifier / reaction resolved during the polling step.
    pub(crate) fn apply_vote_effect(&mut self, effect: cards::Effect, play: &cards::PlayCtx) {
        let mut ctx = self.referendum.take().expect("referendum context");
        match effect {
            cards::Effect::AddVotes(n) => {
                let m = play.actor.expect("voting minion");
                // Votes gained after casting join the cast, on the same
                // side (all votes of a source agree).
                match ctx
                    .records
                    .iter_mut()
                    .find(|r| r.source == Source::Vampire(m) && !r.canceled)
                {
                    Some(r) => r.n += n,
                    None => ctx.bonus.push((m, n)),
                }
            }
            cards::Effect::Abstain { lock, burn } => {
                let cards::Target::Minion(t) = play.target else {
                    unreachable!("abstain without a minion target");
                };
                for r in &mut ctx.records {
                    if r.source == Source::Vampire(t) {
                        r.canceled = true;
                    }
                }
                ctx.abstaining.push(t);
                let v = &mut self.state.minions[t.index()];
                if lock {
                    v.locked = true;
                }
                v.blood = v.blood.saturating_sub(burn);
            }
            other => unreachable!("not a referendum effect: {other:?}"),
        }
        self.referendum = Some(ctx);
    }
}

pub(crate) fn is_vote_effect(e: &cards::Effect) -> bool {
    matches!(
        e,
        cards::Effect::AddVotes(_) | cards::Effect::Abstain { .. }
    )
}
