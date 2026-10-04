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
//! from the as-played stack can modify it (vote bonuses, abstentions,
//! cancellation).

use crate::actions::Outcome;
use crate::cards::{self, Points, RefEffect, ReferendumDef, Term, Window};
use crate::crypt::{Sect, CRYPT};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, LIBRARY};
use crate::state::{title_votes, ActionId, UncontrolledSlot};
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
    /// Passed, but the chosen Methuselah's pool falls short of the points
    /// allocated from it (Parity Shift): the caller re-allocates what there
    /// is within the terms (LSJ 20010606).
    Shortfall {
        points: u8,
    },
}

/// A vote source (rules-qa 1.1): atomic, commits once, all its votes one
/// way. Methuselah ballots (the Edge, a political action card) are
/// independent of each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Vampire(MinionId),
    /// Votes printed on a vampire that it does not cast (Styles Margs).
    Printed(MinionId),
    Edge(SeatId),
    Card(SeatId),
    /// Votes a card in play gives its Methuselah (Ventrue Headquarters).
    InPlay(crate::state::CardId),
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

/// A recipient of referendum points: a Methuselah, or a card in play
/// (Revolutionary Council's locations and equipment).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recipient {
    Seat(SeatId),
    Card(crate::state::CardId),
}

/// An allocation of points under way (`Engine::allocation`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allocation {
    /// Points still to allocate: all of them, in the end.
    pub points: u8,
    /// Recipients still to allocate to, in the order the decisions ask.
    pub recipients: Vec<Recipient>,
    /// At most this many to each (a shortfall: what the terms gave), in
    /// `recipients` order; `None`: no cap.
    pub caps: Vec<Option<u8>>,
    /// How many of `recipients` must get at least 1 point (the terms'
    /// minimum of Methuselahs, less those given points already).
    pub min_nonzero: u8,
}

/// Terms under construction (rulebook "The Referendum" step 1).
#[derive(Clone, Debug, Default)]
pub struct TermsState {
    /// Index of the term being chosen.
    pub term: usize,
    /// Points allocated so far, in allocation order.
    pub alloc: Vec<(SeatId, u8)>,
    /// Seats still to allocate for the current `Allocate` (or shortfall)
    /// step.
    pub pending: Vec<SeatId>,
    /// Chosen Methuselahs (`ChooseSeats`).
    pub seats: Vec<SeatId>,
    /// The chosen Methuselah (`ChooseSeat`).
    pub chosen: Option<SeatId>,
    pub minions: Vec<MinionId>,
    pub clan: Option<&'static str>,
    /// Chosen cards in play (`ChooseCard`).
    pub cards: Vec<crate::state::CardId>,
    /// Cards still to allocate points to, after the seats
    /// (`AllocatePerChosenWithCards`), and the points they got.
    pub card_pending: Vec<crate::state::CardId>,
    pub card_alloc: Vec<(crate::state::CardId, u8)>,
    /// The shortfall re-allocation, in allocation order.
    pub realloc: Vec<(SeatId, u8)>,
}

#[derive(Clone, Debug)]
pub struct ReferendumCtx {
    pub kind: RefKind,
    /// The acting Methuselah (political) or the diablerist's controller.
    pub caller: SeatId,
    /// The vampire calling it (a political action's acting minion).
    pub calling: Option<MinionId>,
    pub def: Option<&'static ReferendumDef>,
    pub step: RefStep,
    pub window: PollingWindow,
    pub records: Vec<VoteRecord>,
    /// Vote bonuses granted to vampires that have not cast yet.
    pub bonus: Vec<(MinionId, u8)>,
    /// Vote grants in force this referendum (the calling card's, all
    /// Methuselahs'; a card in play's, its Methuselah's), read live.
    pub grants: Vec<(Option<SeatId>, cards::VoteGrant)>,
    /// Vampires forced to abstain: they cast nothing more.
    pub abstaining: Vec<MinionId>,
    /// Vampires whose bonus votes are against the referendum (Protected
    /// District): they can only cast against.
    pub forced_against: Vec<MinionId>,
    /// Methuselahs who used their one political-action-card vote.
    pub card_voted: Vec<SeatId>,
    /// Closed Session: non-Camarilla vampires cast nothing.
    pub camarilla_only: bool,
    /// Made to fail (Yoruba Shrine): no tally.
    pub forced_fail: bool,
    /// Scorn of Adonis: pool burned at the tally by each Methuselah who
    /// cast against (one per Scorn).
    pub scorn: u8,
    pub terms: TermsState,
    pub canceled: bool,
    /// A term had no legal choice when its turn came (Parity Shift with no
    /// Methuselah left with more pool): the action fizzles, no referendum
    /// is held (rules-qa 43.1).
    pub fizzled: bool,
    /// Abilities used this referendum (`Limit::PerReferendum`).
    pub abilities_used: Vec<(cards::AbilitySource, u8)>,
}

impl ReferendumCtx {
    /// Votes a vampire can cast right now: its title (ready holder,
    /// uncontested) plus bonuses granted this referendum; none while
    /// abstaining, expelled, or barred by sect.
    pub fn votes_of(&self, st: &crate::state::GameState, m: MinionId) -> u8 {
        if !self.may_vote(st, m) {
            return 0;
        }
        let bonus: u8 = self
            .bonus
            .iter()
            .filter(|(b, _)| *b == m)
            .map(|(_, n)| *n)
            .sum();
        let granted: u8 = self
            .grants
            .iter()
            .filter(|(s, g)| {
                s.is_none_or(|s| st.minions[m.index()].owner == s) && g.voters.reach(st, m)
            })
            .map(|(_, g)| g.n)
            .sum();
        // Printed votes (Neserian) count only while ready, as title votes.
        let printed = st.printed_mod(
            m,
            crate::cards::Stat::Votes,
            crate::state::PrintedCtx::default(),
        );
        // "Non-Anarch titles are worth -1 vote" (Fee Stake: Boston); a
        // title card's "+1 vote during referendums they call".
        let title = st.title_of(m);
        let penalty = i32::from(
            self.def.is_some_and(|d| d.non_anarch_titles_minus)
                && !matches!(
                    title,
                    crate::crypt::Title::None | crate::crypt::Title::Baron(_)
                ),
        );
        let calling = if self.calling == Some(m) {
            st.calling_votes(m)
        } else {
            0
        };
        ((title_votes(title) as i32 - penalty).max(0)
            + st.clan_votes(m)
            + st.host_votes(m)
            + bonus as i32
            + granted as i32
            + printed
            + calling)
            .max(0) as u8
    }

    fn may_vote(&self, st: &crate::state::GameState, m: MinionId) -> bool {
        let v = &st.minions[m.index()];
        v.ready()
            && !v.expelled
            && !self.abstaining.contains(&m)
            && st
                .sect_of(m)
                .is_some_and(|s| !(self.camarilla_only && s != Sect::Camarilla))
    }

    /// The votes `m` casts on `favor`'s side: its votes, plus printed votes
    /// for that side (Jason "Son" Newberry's +2 against a blood hunt).
    pub fn votes_cast(&self, st: &crate::state::GameState, m: MinionId, favor: bool) -> u8 {
        if !self.may_vote(st, m) {
            return 0;
        }
        let side = if !favor && matches!(self.kind, RefKind::BloodHunt { .. }) {
            st.printed_mod(
                m,
                crate::cards::Stat::VotesAgainstBloodHunt,
                crate::state::PrintedCtx::default(),
            )
        } else {
            0
        };
        (self.votes_of(st, m) as i32 + side).max(0) as u8
    }

    /// Has this vampire cast votes or ballots that still stand?
    pub fn has_cast(&self, m: MinionId) -> bool {
        self.records
            .iter()
            .any(|r| r.source == Source::Vampire(m) && !r.canceled)
    }

    /// Is this vampire's standing cast in favor?
    pub fn voting_for(&self, m: MinionId) -> bool {
        self.records
            .iter()
            .any(|r| r.source == Source::Vampire(m) && !r.canceled && r.favor)
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

    pub fn current_term(&self) -> Option<Term> {
        self.def.and_then(|d| d.terms.get(self.terms.term).copied())
    }

    fn next_term(&self) -> Option<Term> {
        self.def
            .and_then(|d| d.terms.get(self.terms.term + 1).copied())
    }
}

/// A location or equipment in play (Revolutionary Council's recipients).
fn allocatable_card(st: &crate::state::GameState, c: crate::state::CardId) -> bool {
    chooseable_card(st, c, cards::CardFilter::Location)
        || chooseable_card(st, c, cards::CardFilter::Type(CardType::Equipment))
}

/// A card in play a `ChooseCard` term may pick.
pub(crate) fn chooseable_card(
    st: &crate::state::GameState,
    c: crate::state::CardId,
    filter: cards::CardFilter,
) -> bool {
    let card = &st.cards[c.index()];
    card.present()
        && !card.effect_only
        && !cards::became(card.card, card.mode)
        && filter.matches(card.card)
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

/// Feasible point values for the next seat of a stepwise allocation: `rem`
/// points left, `nonzero` recipients so far, `after` seats still to come
/// with their caps (`None`: unlimited), at least `min_seats` recipients in
/// the end.
fn feasible_points(
    rem: u8,
    cap: Option<u8>,
    nonzero: u8,
    after: &[Option<u8>],
    min_seats: u8,
) -> Vec<u8> {
    let after_cap: u32 = after
        .iter()
        .map(|c| c.map_or(u32::from(u8::MAX), u32::from))
        .sum();
    (0..=cap.map_or(rem, |c| c.min(rem)))
        .filter(|&n| {
            let left = rem - n;
            let nz = nonzero + (n > 0) as u8;
            if left == 0 {
                nz >= min_seats
            } else {
                u32::from(left) <= after_cap && nz + left.min(after.len() as u8) >= min_seats
            }
        })
        .collect()
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
            calling: match kind {
                RefKind::Political => self.action.as_ref().map(|a| a.acting),
                RefKind::BloodHunt { .. } => None,
            },
            kind,
            caller,
            def,
            step: RefStep::Polling,
            window: PollingWindow::new(order),
            records: Vec::new(),
            bonus: Vec::new(),
            grants: def
                .map(|d| d.grants.iter().map(|&g| (None, g)).collect())
                .unwrap_or_default(),
            abstaining: Vec::new(),
            forced_against: Vec::new(),
            card_voted: Vec::new(),
            camarilla_only: false,
            forced_fail: false,
            scorn: 0,
            terms: TermsState::default(),
            canceled: false,
            fizzled: false,
            abilities_used: Vec::new(),
        };
        // Printed votes in favor of a blood hunt called on the diablerist
        // (Styles Margs), counted as it is called.
        if let RefKind::BloodHunt { diablerist } = ctx.kind {
            let n = self.state.printed_mod(
                diablerist,
                cards::Stat::BloodHuntVotesInFavor,
                crate::state::PrintedCtx::default(),
            );
            if n > 0 {
                ctx.records.push(VoteRecord {
                    source: Source::Printed(diablerist),
                    seat: caller,
                    favor: true,
                    n: n as u8,
                    canceled: false,
                });
            }
        }
        if def.is_some_and(|d| !d.terms.is_empty()) {
            ctx.step = RefStep::Terms;
            self.begin_term(&mut ctx);
        }
        self.referendum = Some(ctx);
        self.flow = Flow::Referendum;
    }

    /// Move on to the next term with something to choose, or the polling.
    fn next_term(&self, ctx: &mut ReferendumCtx) {
        loop {
            ctx.terms.term += 1;
            match ctx.current_term() {
                None => {
                    ctx.step = RefStep::Polling;
                    return;
                }
                // No point to allocate: X = 0 (rules-qa 39.1).
                Some(Term::AllocatePerChosenWithCards { .. }) if ctx.terms.minions.is_empty() => {}
                Some(_) => {
                    self.begin_term(ctx);
                    return;
                }
            }
        }
    }

    /// Set up the current term's state (the seats an allocation runs over,
    /// clockwise from the caller); a term with no legal choice by then
    /// fizzles the action (rules-qa 43.1).
    fn begin_term(&self, ctx: &mut ReferendumCtx) {
        self.set_up_term(ctx);
        if self.terms_step_request(ctx).options.is_empty() {
            ctx.fizzled = true;
        }
    }

    fn set_up_term(&self, ctx: &mut ReferendumCtx) {
        ctx.terms.pending.clear();
        ctx.terms.card_pending.clear();
        match ctx.current_term() {
            Some(Term::Allocate { among, .. }) => {
                ctx.terms.pending =
                    cards::recipients(&self.state, ctx.caller, among, ctx.terms.chosen);
            }
            // Every Methuselah, clockwise from the caller, then every
            // location and equipment in play, arena order.
            Some(Term::AllocatePerChosenWithCards { .. }) => {
                let st = &self.state;
                ctx.terms.pending = st.clockwise_from(ctx.caller);
                ctx.terms.card_pending = (0..st.cards.len())
                    .map(|i| crate::state::CardId(i as u32))
                    .filter(|&c| allocatable_card(st, c))
                    .collect();
            }
            _ => {}
        }
    }

    fn points_of(&self, p: Points) -> u8 {
        match p {
            Points::Fixed(n) | Points::PoolOfChosen(n) => n,
            Points::Methuselahs => self.state.living() as u8,
        }
    }

    /// The allocation of points under way, whole (read-only: a client shows
    /// every recipient at once; the decisions still ask one at a time):
    /// the terms' `Allocate` or `AllocatePerChosenWithCards`, or a
    /// shortfall's re-allocation. `None` outside those steps.
    pub fn allocation(&self) -> Option<Allocation> {
        let ctx = self.referendum.as_ref()?;
        if ctx.fizzled || ctx.canceled {
            return None;
        }
        let t = &ctx.terms;
        let nonzero = |a: &[(SeatId, u8)]| a.iter().filter(|(_, n)| *n > 0).count() as u8;
        let seats = t.pending.iter().map(|&s| Recipient::Seat(s));
        let (points, recipients, caps, min_nonzero) = match ctx.step {
            RefStep::Terms => match ctx.current_term()? {
                Term::Allocate {
                    points, min_seats, ..
                } => {
                    let given: u8 = t.alloc.iter().map(|(_, n)| *n).sum();
                    (
                        self.points_of(points) - given,
                        seats.collect::<Vec<_>>(),
                        vec![None; t.pending.len()],
                        min_seats.saturating_sub(nonzero(&t.alloc)),
                    )
                }
                Term::AllocatePerChosenWithCards { per } => {
                    let given: u8 = t.alloc.iter().map(|(_, n)| *n).sum::<u8>()
                        + t.card_alloc.iter().map(|(_, n)| *n).sum::<u8>();
                    let any = nonzero(&t.alloc) > 0 || t.card_alloc.iter().any(|(_, n)| *n > 0);
                    let recipients: Vec<Recipient> = seats
                        .chain(t.card_pending.iter().map(|&c| Recipient::Card(c)))
                        .collect();
                    let caps = vec![None; recipients.len()];
                    (
                        per * t.minions.len() as u8 - given,
                        recipients,
                        caps,
                        u8::from(!any),
                    )
                }
                _ => return None,
            },
            RefStep::Shortfall { points } => {
                let cap = |s: SeatId| t.alloc.iter().find(|(r, _)| *r == s).map(|(_, n)| *n);
                let given: u8 = t.realloc.iter().map(|(_, n)| *n).sum();
                (
                    points - given,
                    seats.collect(),
                    t.pending
                        .iter()
                        .map(|&s| Some(cap(s).unwrap_or(0)))
                        .collect(),
                    0,
                )
            }
            RefStep::Polling => return None,
        };
        if recipients.is_empty() {
            return None;
        }
        Some(Allocation {
            points,
            recipients,
            caps,
            min_nonzero,
        })
    }

    pub(crate) fn referendum_request(&self) -> Option<DecisionRequest> {
        let ctx = self.referendum.as_ref().expect("referendum context");
        if ctx.fizzled {
            return None;
        }
        if ctx.canceled {
            // The returned political action card may leave the acting
            // Methuselah over hand size: "discard down afterward" — the
            // engine's discard-down, asked before any other decision.
            return None;
        }
        match ctx.step {
            RefStep::Terms => Some(self.terms_step_request(ctx)),
            RefStep::Polling => self.polling_request(ctx),
            RefStep::Shortfall { points } => Some(self.shortfall_request(ctx, points)),
        }
    }

    /// Minions a `ChooseMinions` term may pick, arena order.
    fn minion_choices(&self, ctx: &ReferendumCtx, filter: &[cards::Requirement]) -> Vec<MinionId> {
        let st = &self.state;
        let acting = self.action.as_ref().map(|a| a.acting);
        (0..st.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(|&m| {
                st.minions[m.index()].present()
                    && !ctx.terms.minions.contains(&m)
                    && filter.iter().all(|&r| {
                        cards::holds_for_minion(
                            r,
                            st,
                            ctx.caller,
                            Some(ctx),
                            acting,
                            m,
                            self.action.as_ref(),
                        )
                    })
            })
            .collect()
    }

    fn terms_step_request(&self, ctx: &ReferendumCtx) -> DecisionRequest {
        let st = &self.state;
        let options = match ctx.current_term().expect("a term to choose") {
            Term::Allocate {
                points, min_seats, ..
            } => {
                let total = self.points_of(points);
                let given: u8 = ctx.terms.alloc.iter().map(|(_, n)| *n).sum();
                let nonzero = ctx.terms.alloc.iter().filter(|(_, n)| *n > 0).count() as u8;
                let seat = ctx.terms.pending[0];
                let after = vec![None; ctx.terms.pending.len() - 1];
                feasible_points(total - given, None, nonzero, &after, min_seats)
                    .into_iter()
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
            // A Methuselah may be chosen only if the allocation that
            // follows stays feasible (Camarilla's Iron Fist with two
            // opponents: the beneficiary cannot be one of them).
            Term::ChooseSeat(filter) => st
                .clockwise_from(ctx.caller)
                .into_iter()
                .filter(|&c| {
                    cards::seat_filter_holds(filter, st, ctx.caller, c)
                        && match ctx.next_term() {
                            Some(Term::Allocate {
                                among, min_seats, ..
                            }) => {
                                cards::recipients(st, ctx.caller, among, Some(c)).len() as u8
                                    >= min_seats
                            }
                            _ => true,
                        }
                })
                .map(Choice::ChooseSeat)
                .collect(),
            Term::ChooseMinions { filter, min, max } => {
                let mut options = Vec::new();
                if ctx.terms.minions.len() as u8 >= min {
                    options.push(Choice::Pass);
                }
                if (ctx.terms.minions.len() as u8) < max {
                    options.extend(
                        self.minion_choices(ctx, filter)
                            .into_iter()
                            .map(Choice::ChooseMinion),
                    );
                }
                options
            }
            Term::ChooseClan => clans().into_iter().map(Choice::ChooseClan).collect(),
            Term::ChooseCard(filter) => (0..st.cards.len())
                .filter(|&i| chooseable_card(st, crate::state::CardId(i as u32), filter))
                .map(|i| Choice::ChooseCard(i as u32))
                .collect(),
            Term::AllocatePerChosenWithCards { per } => {
                let total = per * ctx.terms.minions.len() as u8;
                let given: u8 = ctx.terms.alloc.iter().map(|(_, n)| *n).sum::<u8>()
                    + ctx.terms.card_alloc.iter().map(|(_, n)| *n).sum::<u8>();
                let nonzero = (ctx.terms.alloc.iter().filter(|(_, n)| *n > 0).count()
                    + ctx.terms.card_alloc.iter().filter(|(_, n)| *n > 0).count())
                    as u8;
                let left = ctx.terms.pending.len() + ctx.terms.card_pending.len();
                let after = vec![None; left - 1];
                let ns = feasible_points(total - given, None, nonzero, &after, 1);
                match ctx.terms.pending.first() {
                    Some(&seat) => ns
                        .into_iter()
                        .map(|n| Choice::AllocateTo { seat, n })
                        .collect(),
                    None => {
                        let card = ctx.terms.card_pending[0].0;
                        ns.into_iter()
                            .map(|n| Choice::AllocateToCard { card, n })
                            .collect()
                    }
                }
            }
        };
        DecisionRequest::new(ctx.caller, WindowLabel::ReferendumTerms, options)
    }

    /// Re-allocation of a shortfall: each recipient may get at most what
    /// the terms gave it, and all `points` are distributed.
    fn shortfall_request(&self, ctx: &ReferendumCtx, points: u8) -> DecisionRequest {
        let cap_of = |s: SeatId| {
            ctx.terms
                .alloc
                .iter()
                .find(|(r, _)| *r == s)
                .map_or(0, |(_, n)| *n)
        };
        let given: u8 = ctx.terms.realloc.iter().map(|(_, n)| *n).sum();
        let seat = ctx.terms.pending[0];
        let after: Vec<Option<u8>> = ctx.terms.pending[1..]
            .iter()
            .map(|&s| Some(cap_of(s)))
            .collect();
        let options = feasible_points(points - given, Some(cap_of(seat)), 0, &after, 0)
            .into_iter()
            .map(|n| Choice::AllocateTo { seat, n })
            .collect();
        DecisionRequest::new(ctx.caller, WindowLabel::ReferendumTerms, options)
    }

    /// The blood `m` burns to cast votes against the referendum: printed on
    /// the vampire calling it (Alexander Silverson) — a political action's,
    /// not a blood hunt's, which has no calling vampire.
    fn vote_against_toll(&self, ctx: &ReferendumCtx, m: MinionId) -> u8 {
        self.state
            .printed_mod(
                m,
                cards::Stat::VoteAgainstToll,
                crate::state::PrintedCtx {
                    opponent: ctx.calling,
                    ..Default::default()
                },
            )
            .max(0) as u8
    }

    fn polling_request(&self, ctx: &ReferendumCtx) -> Option<DecisionRequest> {
        let st = &self.state;
        let seat = ctx.window.current()?;
        let mut options = vec![Choice::Pass];
        for id in st.minions_of(seat) {
            // Torpor vampires must abstain; locked ready ones may vote;
            // contested titles carry no votes.
            if ctx.spent(Source::Vampire(id)) {
                continue;
            }
            if ctx.votes_cast(st, id, true) > 0 && !ctx.forced_against.contains(&id) {
                options.push(Choice::CastVotes {
                    minion: id,
                    favor: true,
                });
            }
            if ctx.votes_cast(st, id, false) > 0
                && st.minions[id.index()].blood >= self.vote_against_toll(ctx, id)
            {
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
        // 1 vote from political action cards per Methuselah — the calling
        // card is the acting Methuselah's (rules-qa 5.4): they burn no
        // other (owner, 2026-09-30: rules-qa 46.2).
        if !ctx.card_voted.contains(&seat) {
            // A referendum granted by a card in play has no calling card.
            let card_called = self.action.as_ref().is_some_and(|a| a.card.is_some());
            let calling = ctx.kind == RefKind::Political && seat == ctx.caller && card_called;
            if calling {
                options.push(Choice::CastCardVote { favor: true });
                options.push(Choice::CastCardVote { favor: false });
            }
            let hand = &st.seats[seat.index()].hand;
            for (hi, &card) in hand.iter().enumerate() {
                // Copies alike: one choice (owner, 2026-10-04).
                if !calling
                    && crate::state::first_copy(hand, hi)
                    && LIBRARY[card as usize]
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
        // Referendum abilities ("during the polling step of any referendum").
        options.extend(self.ability_options(seat, Window::Referendum));
        let window = match ctx.kind {
            RefKind::BloodHunt { .. } => WindowLabel::BloodHuntPolling,
            RefKind::Political => {
                // Other Methuselahs' locked minions may wake to react.
                let windows: &[Window] = if seat == ctx.caller {
                    &[Window::Referendum]
                } else {
                    &[Window::Referendum, Window::Wake]
                };
                options.extend(self.play_options(seat, windows, None));
                options.extend(self.ability_options(seat, Window::AnyImpulse));
                WindowLabel::ReferendumPolling
            }
        };
        Some(DecisionRequest::new(seat, window, options))
    }

    pub(crate) fn referendum_apply(&mut self, choice: &Choice) {
        let mut ctx = self.referendum.take().expect("referendum context");
        match ctx.step {
            RefStep::Terms => self.terms_step_apply(&mut ctx, choice),
            RefStep::Polling => {
                if let Choice::UseAbility {
                    source,
                    ability,
                    actor,
                    target,
                } = choice
                {
                    // An untimed ability is a commitment of the rotation
                    // like a card play.
                    ctx.window.commit();
                    self.referendum = Some(ctx);
                    self.flow = Flow::Referendum;
                    self.use_ability(*source, *ability, *actor, *target, Flow::Referendum);
                    return;
                }
                if let Some(play) = self.polling_apply(&mut ctx, choice) {
                    let (card, mode, actor, target, seat) = play;
                    self.referendum = Some(ctx);
                    self.flow = Flow::Referendum;
                    self.announce_play(seat, card, mode, actor, target, Flow::Referendum);
                    return;
                }
            }
            RefStep::Shortfall { .. } => {
                let Choice::AllocateTo { seat, n } = choice else {
                    unreachable!("illegal shortfall choice {choice:?}");
                };
                debug_assert_eq!(ctx.terms.pending.first(), Some(seat));
                ctx.terms.pending.remove(0);
                ctx.terms.realloc.push((*seat, *n));
                if ctx.terms.pending.is_empty() {
                    self.resolve_political(ctx);
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
            (Term::ChooseSeat(_), Choice::ChooseSeat(s)) => {
                ctx.terms.chosen = Some(*s);
                true
            }
            (Term::ChooseMinions { max, .. }, Choice::ChooseMinion(m)) => {
                ctx.terms.minions.push(*m);
                ctx.terms.minions.len() as u8 >= max
            }
            (Term::ChooseMinions { .. }, Choice::Pass) => true,
            (Term::ChooseClan, Choice::ChooseClan(c)) => {
                ctx.terms.clan = Some(c);
                true
            }
            (Term::ChooseCard(_), Choice::ChooseCard(c)) => {
                ctx.terms.cards.push(crate::state::CardId(*c));
                true
            }
            (Term::AllocatePerChosenWithCards { .. }, Choice::AllocateTo { seat, n }) => {
                debug_assert_eq!(ctx.terms.pending.first(), Some(seat));
                ctx.terms.pending.remove(0);
                ctx.terms.alloc.push((*seat, *n));
                ctx.terms.pending.is_empty() && ctx.terms.card_pending.is_empty()
            }
            (Term::AllocatePerChosenWithCards { .. }, Choice::AllocateToCard { card, n }) => {
                let c = ctx.terms.card_pending.remove(0);
                debug_assert_eq!(c.0, *card);
                ctx.terms.card_alloc.push((c, *n));
                ctx.terms.card_pending.is_empty()
            }
            (t, c) => unreachable!("illegal terms choice {c:?} for {t:?}"),
        };
        if done {
            self.next_term(ctx);
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
            Choice::CastVotes { minion, favor } => {
                if !*favor {
                    let toll = self.vote_against_toll(ctx, *minion);
                    self.state.minions[minion.index()].blood -= toll;
                }
                (
                    Source::Vampire(*minion),
                    *favor,
                    ctx.votes_cast(&self.state, *minion, *favor),
                )
            }
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
                self.replace(seat, 1, None);
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

    /// The polling window closed (or the referendum was canceled): tally
    /// and resolve.
    pub(crate) fn referendum_advance(&mut self) {
        let mut ctx = self.referendum.take().expect("referendum context");
        if ctx.fizzled {
            // Held neither to pass, fail nor be canceled: no rider keyed
            // to its result (rules-qa 43.1).
            self.after_referendum(false);
            return;
        }
        if ctx.canceled {
            // No tally: nothing keyed to the results happens (LSJ
            // 20010209).
            debug_assert!(ctx.kind == RefKind::Political);
            self.after_referendum(true);
            return;
        }
        debug_assert!(ctx.step == RefStep::Polling && ctx.window.is_closed());
        let (favor, against) = ctx.tally();
        let passed = favor > against && !ctx.forced_fail;
        match ctx.kind {
            RefKind::BloodHunt { diablerist } => {
                if passed {
                    self.burn_minion(diablerist);
                }
                // Part of the action (spec §3.7.8.1): the after-resolution
                // window follows it (rules-qa 5.8).
                self.after_referendum(true);
            }
            RefKind::Political => {
                {
                    let a = self.action.as_mut().expect("political action");
                    a.referendum_passed = passed;
                    a.margin = if passed { favor - against } else { 0 };
                }
                // Scorn of Adonis: paid at the tally, before the effects.
                if ctx.scorn > 0 {
                    let mut loss = vec![0i32; self.state.seats.len()];
                    for r in ctx.records.iter().filter(|r| !r.canceled && !r.favor) {
                        loss[r.seat.index()] = ctx.scorn as i32;
                    }
                    self.burn_pool_batch(&loss);
                }
                let def = ctx.def.expect("political definition");
                if passed && def.effect == RefEffect::MoveAllocatedFromChosen {
                    let chosen = ctx.terms.chosen.expect("chosen Methuselah");
                    let total: u8 = ctx.terms.alloc.iter().map(|(_, n)| *n).sum();
                    let available = self.state.seats[chosen.index()].pool.max(0) as u8;
                    if available < total {
                        ctx.step = RefStep::Shortfall { points: available };
                        ctx.terms.pending = ctx
                            .terms
                            .alloc
                            .iter()
                            .filter(|(_, n)| *n > 0)
                            .map(|(s, _)| *s)
                            .collect();
                        self.referendum = Some(ctx);
                        self.flow = Flow::Referendum;
                        return;
                    }
                }
                self.resolve_political(ctx);
            }
        }
    }

    fn resolve_political(&mut self, ctx: ReferendumCtx) {
        let passed = self
            .action
            .as_ref()
            .expect("political action")
            .referendum_passed;
        if passed {
            let def = ctx.def.expect("political definition");
            self.apply_referendum_effect(def.effect, &ctx);
        }
        self.after_referendum(true);
    }

    /// Effects of a passed referendum; pool losses are simultaneous, ousts
    /// resolved as a batch (spec §6.5.4).
    fn apply_referendum_effect(&mut self, effect: RefEffect, ctx: &ReferendumCtx) {
        let n_seats = self.state.seats.len();
        let mut loss = vec![0i32; n_seats];
        let mut gain = vec![0i32; n_seats];
        {
            let st = &self.state;
            match effect {
                RefEffect::BurnAllocated => {
                    for &(s, n) in &ctx.terms.alloc {
                        loss[s.index()] += n as i32;
                    }
                }
                RefEffect::BurnAllocatedChosenGains(n) => {
                    for &(s, k) in &ctx.terms.alloc {
                        loss[s.index()] += k as i32;
                    }
                    gain[ctx.terms.chosen.expect("chosen").index()] += n as i32;
                }
                RefEffect::MoveAllocatedFromChosen => {
                    let alloc = if ctx.terms.realloc.is_empty() {
                        &ctx.terms.alloc
                    } else {
                        &ctx.terms.realloc
                    };
                    let chosen = ctx.terms.chosen.expect("chosen");
                    for &(s, n) in alloc {
                        gain[s.index()] += n as i32;
                        loss[chosen.index()] += n as i32;
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
                                && st.capacity_of(m).is_some_and(|c| c <= young_cap)
                        }) {
                            loss[s.index()] += young_bonus as i32;
                        }
                    }
                }
                RefEffect::GainPerClanVampire => {
                    let clan = ctx.terms.clan.expect("chosen clan");
                    for m in &st.minions {
                        if m.present() && m.clan() == Some(clan) {
                            gain[m.owner.index()] += 1;
                        }
                    }
                }
                RefEffect::AnarchSalon => {
                    for (i, m) in st.minions.iter().enumerate() {
                        if m.present() && st.sect_of(MinionId(i as u32)) == Some(Sect::Anarch) {
                            gain[m.owner.index()] = 1;
                        }
                    }
                }
                RefEffect::LockChosenBurnAllocated => {
                    for &(s, n) in &ctx.terms.alloc {
                        loss[s.index()] += n as i32;
                    }
                }
                RefEffect::Banish
                | RefEffect::Expel
                | RefEffect::BurnSource
                | RefEffect::PutOnChosen
                | RefEffect::PutInPlay
                | RefEffect::ChosenTakesChosenCard => {}
            }
        }
        for (i, g) in gain.iter().enumerate() {
            if !self.state.seats[i].ousted {
                self.state.seats[i].pool += g;
            }
        }
        match effect {
            RefEffect::AnarchSalon => {
                for i in 0..self.state.minions.len() {
                    let m = &self.state.minions[i];
                    if m.ready()
                        && self.state.sect_of(crate::ids::MinionId(i as u32)) == Some(Sect::Anarch)
                    {
                        self.state.gain_blood(crate::ids::MinionId(i as u32), 1);
                    }
                }
            }
            RefEffect::Banish => {
                let m = ctx.terms.minions[0];
                let to = self.state.minions[m.index()].owner;
                self.banish(m, to);
            }
            RefEffect::Expel => {
                for &m in &ctx.terms.minions {
                    self.state.minions[m.index()].expelled = true;
                }
            }
            RefEffect::PutOnChosen => {
                let m = ctx.terms.minions[0];
                let a = self.action.as_ref().expect("political action");
                let (seat, play) = (a.seat, a.card.expect("a political action card"));
                if self.state.minions[m.index()].present() {
                    let id = self.enter_play(crate::state::CardInPlay::new(
                        play.card,
                        seat,
                        Some(m),
                        play.mode,
                    ));
                    self.action
                        .as_mut()
                        .expect("political action")
                        .card_entered_play = true;
                    self.push_mode_layers(id);
                    if let Some(t) = cards::card_title(play.card) {
                        self.gain_title(m, t, Some(id));
                    }
                }
            }
            // Locked, burned and pool lost at once; the chosen Anarchs
            // count though locked since (LSJ 20081203).
            RefEffect::LockChosenBurnAllocated => {
                for &m in &ctx.terms.minions {
                    if self.state.minions[m.index()].present() {
                        self.state.minions[m.index()].locked = true;
                    }
                }
                for &(c, n) in &ctx.terms.card_alloc {
                    if n > 0 && self.state.cards[c.index()].present() {
                        self.burn_card_in_play(c);
                    }
                }
            }
            RefEffect::PutInPlay => {
                let a = self.action.as_ref().expect("political action");
                let (seat, play) = (a.seat, a.card.expect("a political action card"));
                // "A Methuselah can have only one": not a second.
                if !self.state.controls_card_copy(seat, play.card) {
                    let id = self.enter_play(crate::state::CardInPlay::new(
                        play.card, seat, None, play.mode,
                    ));
                    self.push_mode_layers(id);
                    self.action
                        .as_mut()
                        .expect("political action")
                        .card_entered_play = true;
                }
            }
            RefEffect::ChosenTakesChosenCard => {
                let c = ctx.terms.cards[0];
                let to = ctx.terms.chosen.expect("chosen Methuselah");
                if self.state.cards[c.index()].present()
                    && !self.state.seats[to.index()].ousted
                    && self.state.card_controller(c) != to
                {
                    self.give_location(c, to);
                }
            }
            RefEffect::BurnSource => {
                let src = self
                    .action
                    .as_ref()
                    .and_then(|a| a.source)
                    .expect("granted action's source")
                    .card();
                self.burn_card_in_play(src);
            }
            _ => {}
        }
        self.burn_pool_batch(&loss);
    }

    /// The layers of the mode card `id` is in play with (a political action
    /// card: Archon, Raising the Portcullis; a card placed or become in
    /// combat).
    pub(crate) fn push_mode_layers(&mut self, id: crate::state::CardId) {
        let c = &self.state.cards[id.index()];
        let mode = &cards::lookup(c.card).expect("definition").modes[c.mode as usize];
        for e in mode.effects {
            if let cards::Effect::Layer(modifier) = e {
                self.state.layers.push(cards::Layer {
                    scope: cards::Scope::InPlay,
                    modifier: *modifier,
                    source: id,
                });
            }
        }
    }

    /// Simultaneous pool losses: every Methuselah at 0 is ousted at once.
    pub(crate) fn burn_pool_batch(&mut self, loss: &[i32]) {
        let mut ousted = Vec::new();
        for (i, &n) in loss.iter().enumerate() {
            let seat = &mut self.state.seats[i];
            if seat.ousted || n <= 0 {
                continue;
            }
            // Below 0 is kept until the oust: a Life Boon must make it good
            // (RTR 19970630).
            seat.pool -= n;
            if seat.pool <= 0 {
                ousted.push(SeatId(i as u8));
            }
        }
        if !ousted.is_empty() {
            self.oust_batch(&ousted);
        }
    }

    /// A vampire moved to `to`'s uncontrolled region (Banishment, Pit of
    /// Contemplation): its blood kept as counters, the cards on it leaving
    /// play with it, face down, until it returns — everything it had
    /// stays with it (rules-qa 25.1).
    pub(crate) fn banish(&mut self, m: MinionId, to: SeatId) {
        self.about_to_leave_ready(&[m]);
        let v = &mut self.state.minions[m.index()];
        let was_ready = v.ready();
        v.in_play = false;
        let card = v.crypt_card().unwrap_or(crate::state::CREATED);
        let blood = v.blood;
        v.blood = 0;
        // Face down on the vampire: out of play until it returns (the cards
        // in play on it, not those burned from it) — a created vampire's
        // own card too.
        let mut on = Vec::new();
        for (ci, c) in self.state.cards.iter_mut().enumerate() {
            if c.in_play && (c.host == Some(m) || c.body == Some(m)) {
                c.in_play = false;
                on.push(crate::state::CardId(ci as u32));
            }
        }
        self.state.seats[to.index()]
            .uncontrolled
            .push(UncontrolledSlot {
                card,
                counters: blood,
                banished: Some(crate::state::Banished {
                    minion: m,
                    cards: on,
                }),
            });
        // It left the ready region (Slaughtering the Herd [DOM]).
        if was_ready {
            self.by_left_ready(m);
        }
    }

    /// The referendum concluded (passed, failed or canceled; `riders`: the
    /// failed-or-canceled riders apply — not to a fizzled one): the action
    /// resumes at its after-resolution window — unless the acting
    /// Methuselah was ousted (the turn passes) or the game ended.
    fn after_referendum(&mut self, riders: bool) {
        if self.state.living() <= 1 {
            self.action = None;
            self.flow = Flow::GameOver;
            return;
        }
        let a = self.action.as_ref().expect("action");
        let (seat, blocked) = (a.seat, a.outcome == Some(Outcome::Blocked));
        if self.state.seats[seat.index()].ousted {
            self.action = None;
            self.next_turn();
            return;
        }
        // A blocked leave-torpor whose blocker diablerised (its blood hunt)
        // opens the after-resolution window too, for what the diablerie
        // triggers (rules-qa 16.2).
        if riders && !(blocked && a.diablerist.is_none()) {
            self.own_referendum_failed_riders();
        }
        self.after_resolution();
    }

    /// "If the referendum of a political action called by this vampire is
    /// canceled or fails" riders (Cedrick Calhoun), applied as the action
    /// resolves, before the after-resolution window.
    fn own_referendum_failed_riders(&mut self) {
        let a = self.action.as_ref().expect("action");
        if a.kind != crate::decision::ActionKind::Political || a.referendum_passed {
            return;
        }
        let m = a.acting;
        let v = &self.state.minions[m.index()];
        let Some(def) = cards::vampire_of(v).filter(|_| v.ready()) else {
            return;
        };
        for r in def
            .riders
            .iter()
            .filter(|r| r.hook == cards::Hook::OwnReferendumFailed)
        {
            for e in r.effects {
                match *e {
                    cards::Effect::GoToTorpor => self.minions_to_torpor(&[m]),
                    other => unreachable!("not a referendum rider effect: {other:?}"),
                }
            }
        }
    }

    /// Votes card `id` in play gives its Methuselah, cast at once on one
    /// side (Ventrue Headquarters).
    pub(crate) fn methuselah_votes(
        &mut self,
        id: crate::state::CardId,
        seat: SeatId,
        n: u8,
        favor: bool,
    ) {
        let ctx = self.referendum.as_mut().expect("referendum context");
        ctx.records.push(VoteRecord {
            source: Source::InPlay(id),
            seat,
            favor,
            n,
            canceled: false,
        });
    }

    /// Effects of a modifier / reaction resolved during the polling step.
    pub(crate) fn apply_vote_effect(&mut self, effect: cards::Effect, play: &cards::PlayCtx) {
        let mut ctx = self.referendum.take().expect("referendum context");
        match effect {
            cards::Effect::AddVotes(_) | cards::Effect::AddVotesPerReadySect(_) => {
                let n = match effect {
                    cards::Effect::AddVotesPerReadySect(sect) => {
                        self.state
                            .minions_of(play.seat)
                            .filter(|&m| {
                                self.state.minions[m.index()].ready()
                                    && self.state.sect_of(m) == Some(sect)
                            })
                            .count() as u8
                    }
                    cards::Effect::AddVotes(n) => n,
                    _ => unreachable!(),
                };
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
            cards::Effect::GrantVotes(g) => {
                // Votes gained after casting join the cast.
                for r in ctx.records.iter_mut().filter(|r| !r.canceled) {
                    if let Source::Vampire(m) = r.source {
                        if self.state.minions[m.index()].owner == play.seat
                            && g.voters.reach(&self.state, m)
                        {
                            r.n += g.n;
                        }
                    }
                }
                ctx.grants.push((Some(play.seat), g));
            }
            cards::Effect::AddVotesAgainst(n) => {
                // Votes against only: they join a cast against, or force
                // the vampire's cast against (LSJ 20031119).
                let m = play.actor.expect("voting minion");
                match ctx
                    .records
                    .iter_mut()
                    .find(|r| r.source == Source::Vampire(m) && !r.canceled)
                {
                    Some(r) => {
                        debug_assert!(!r.favor);
                        r.n += n;
                    }
                    None => {
                        ctx.bonus.push((m, n));
                        ctx.forced_against.push(m);
                    }
                }
            }
            cards::Effect::Abstain { lock, burn } => {
                let t = play.target.minion();
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
            cards::Effect::TallyPenaltyAgainst(n) => ctx.scorn += n,
            cards::Effect::BarNonCamarillaVotes => ctx.camarilla_only = true,
            // "Have the referendum fail" (Yoruba Shrine): the polling is
            // over.
            cards::Effect::FailReferendum => {
                ctx.forced_fail = true;
                ctx.window = PollingWindow::new(Vec::new());
            }
            cards::Effect::CancelReferendum {
                unlock_acting,
                forbid_repeat,
            } => {
                ctx.canceled = true;
                let a = self.action.as_mut().expect("political action");
                // The political action card returns to its owner's hand
                // (before it is burned: LSJ 20050324).
                if let Some(card) = a.card {
                    self.state.seats[a.seat.index()].hand.push(card.card);
                    a.card_returned = true;
                }
                // The forbidden action is the card's from hand (by name),
                // or this copy's in play: another copy's referendum stays
                // available (LSJ 20020201).
                let id = match (a.card, a.source) {
                    (Some(card), _) => Some(ActionId::Card(card.card)),
                    (None, Some(src)) => a.ability.map(|ab| src.action_id(&self.state, ab)),
                    (None, None) => None,
                };
                if let (true, Some(id)) = (forbid_repeat, id) {
                    self.state.seats[a.seat.index()]
                        .forbidden_political
                        .push(id);
                }
                if unlock_acting && !a.no_unlock {
                    self.state.minions[a.acting.index()].locked = false;
                }
            }
            other => unreachable!("not a referendum effect: {other:?}"),
        }
        self.referendum = Some(ctx);
    }
}

pub(crate) fn is_vote_effect(e: &cards::Effect) -> bool {
    matches!(
        e,
        cards::Effect::AddVotes(_)
            | cards::Effect::GrantVotes(_)
            | cards::Effect::AddVotesPerReadySect(_)
            | cards::Effect::AddVotesAgainst(_)
            | cards::Effect::Abstain { .. }
            | cards::Effect::TallyPenaltyAgainst(_)
            | cards::Effect::BarNonCamarillaVotes
            | cards::Effect::FailReferendum
            | cards::Effect::CancelReferendum { .. }
    )
}
