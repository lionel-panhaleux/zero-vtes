//! Referendums (rulebook "The Referendum"): terms (rules-qa 3.5), polling
//! (vote sources per rules-qa 1.1; card plays as commitments, DESIGN §4.2),
//! tally. The blood hunt has no cards and no terms
//! (rulebook "The Blood Hunt"). The context sits on the engine so plays
//! resolved from the as-played stack can modify it.

use crate::actions::Outcome;
use crate::cards::{self, Points, RefEffect, ReferendumDef, Term, Window};
use crate::crypt::{Sect, CRYPT};
use crate::decision::{Choice, DecisionRequest, WindowLabel};
use crate::game::{Engine, Flow};
use crate::ids::{MinionId, SeatId};
use crate::library::{CardType, LIBRARY};
use crate::notes::{self, Cause, Rule};
use crate::plays::{HookAt, ModeLayers};
use crate::state::{title_votes, ActionId, Expiry, TimedStat, UncontrolledSlot};
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
    /// Passed: each Methuselah in `terms.pending`, clockwise from the
    /// acting one, can choose a ready vampire they control, in the open
    /// (Ancient Influence, Reins of Power; rules-qa 49.1).
    Choose,
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
    /// Votes a Methuselah got for pool burned (Business Pressure): one
    /// record per pool, each its own side.
    Pool(SeatId),
}

/// Business Pressure resolving: each Methuselah in turn, clockwise from
/// its player, chooses once how much pool to burn for votes and on which
/// side; the card resolves once all have chosen (rules-qa 51.5).
#[derive(Clone, Debug)]
pub struct Pressure {
    /// The card's player: `own` votes a pool; the others `per`.
    pub player: SeatId,
    pub per: u8,
    pub own: u8,
    /// The Methuselahs still to choose, in order.
    pub pending: Vec<SeatId>,
    /// The pool the first of them chose, its side still to choose.
    pub amount: Option<u8>,
    /// The choices so far: (Methuselah, pool burned, in favor).
    pub picks: Vec<(SeatId, u8, bool)>,
}

impl Pressure {
    /// The votes `seat` gets for each pool it burns.
    pub fn rate(&self, seat: SeatId) -> u8 {
        if seat == self.player {
            self.own
        } else {
            self.per
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoteRecord {
    pub source: Source,
    pub seat: SeatId,
    pub favor: bool,
    pub n: u8,
    /// Canceled by an abstention (Scalpel Tongue): not tallied.
    pub canceled: bool,
    /// The source's ballots in the prisci sub-referendum, on the same side
    /// (rulebook "The Prisci Block"): a priscus casting's, and printed ones
    /// (Gratiano's, title or not: LSJ 20040519).
    pub ballots: u8,
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
    /// How many minions the terms before the current one chose: a
    /// distribution's own picks follow them (Public Vilification).
    pub term_start: usize,
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
    /// The vampires the Methuselahs chose as it passed (`RefStep::Choose`).
    pub picks: Vec<(SeatId, MinionId)>,
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
    /// The vampires that forced another's cast this referendum (Mustafa's
    /// Toreador: once each, "during the polling step").
    pub forcers: Vec<MinionId>,
    /// Methuselahs who used their one political-action-card vote.
    pub card_voted: Vec<SeatId>,
    /// Only vampires of this sect vote (Closed Session, Fall of the
    /// Camarilla's ban, Private Audience, Cardinal Benediction).
    pub only_sect: Option<Sect>,
    /// Vampires of this sect do not vote (Bruce de Guy).
    pub barred_sect: Option<Sect>,
    /// Votes each non-Camarilla vampire loses (Camarilla Conclave), but
    /// the vampire a reaction does not affect (Perfect Clarity).
    pub penalties: Vec<(cards::Penalized, u8, Option<MinionId>)>,
    /// Business Pressure's round under way: the polling step waits on it.
    pub pressure: Option<Pressure>,
    /// Passes automatically, the polling step skipped (Día de los Muertos):
    /// nothing "during the polling step" or "during a referendum" is
    /// played, a margin it passed by is none (LSJ 19980107).
    pub auto_pass: bool,
    /// A political referendum's opening rotation, before the polling
    /// proper: the other Methuselahs first, the caller last, may play the
    /// cards usable "before any votes" (Confusion of the Eye [OBF], Fakir
    /// al Sidi) — nothing else, no vote (owner, 2026-10-11: rules-qa 64.12).
    pub pre_vote: bool,
    /// Made to fail (Yoruba Shrine): no tally.
    pub forced_fail: bool,
    /// Malkavian Rider Clause: the players whose next referendum passes
    /// automatically if this one passes.
    pub rider_clauses: Vec<SeatId>,
    /// Scorn of Adonis: pool burned at the tally by each Methuselah who
    /// cast against (one per Scorn).
    pub scorn: u8,
    /// Poison Pill: the Methuselahs whose pool lost to the referendum's
    /// effect the acting vampire's controller loses as well.
    pub poison: Vec<SeatId>,
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
            crate::state::PrintedCtx {
                calling: self.calling,
                ..Default::default()
            },
        );
        // "Each non-priscus titled vampire gets -1 vote" (Eat the Rich).
        let rich = i32::from(
            self.def.is_some_and(|d| d.titled_minus)
                && !matches!(
                    st.title_of(m),
                    crate::crypt::Title::None | crate::crypt::Title::Priscus
                ),
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
        // "+1 vote after a Methuselah burns the Edge for a vote" (Tyler
        // McGill).
        let edge = if (0..st.seats.len()).any(|s| self.spent(Source::Edge(SeatId(s as u8)))) {
            st.printed_mod(
                m,
                crate::cards::Stat::VotesAfterEdgeBurned,
                crate::state::PrintedCtx::default(),
            )
        } else {
            0
        };
        let conclave: i32 = self
            .penalties
            .iter()
            .filter(|&&(p, _, spared)| p.reaches(st, m) && spared != Some(m))
            .map(|&(_, n, _)| n as i32)
            .sum();
        ((title_votes(title) as i32 - penalty).max(0)
            + st.clan_votes(m)
            + st.host_votes(m)
            + bonus as i32
            + granted as i32
            + printed
            + calling
            + edge
            - conclave
            - rich)
            .max(0) as u8
    }

    pub(crate) fn may_vote(&self, st: &crate::state::GameState, m: MinionId) -> bool {
        let v = &st.minions[m.index()];
        v.ready()
            && !v.expelled
            && !self.abstaining.contains(&m)
            && !st.bears(m, crate::cards::Modifier::CastsNoVotes)
            && st.sect_of(m).is_some_and(|s| {
                self.only_sect.is_none_or(|o| s == o) && self.barred_sect != Some(s)
            })
    }

    /// The votes `m` casts on `favor`'s side: its votes, plus printed votes
    /// for that side (Jason "Son" Newberry's +2 against a blood hunt).
    pub fn votes_cast(&self, st: &crate::state::GameState, m: MinionId, favor: bool) -> u8 {
        if !self.may_vote(st, m) || !favor && self.barred_against(st, m) {
            return 0;
        }
        let blood_hunt = matches!(self.kind, RefKind::BloodHunt { .. });
        let read = |stat| st.printed_mod(m, stat, crate::state::PrintedCtx::default());
        let mut side = 0;
        if blood_hunt {
            side += read(crate::cards::Stat::VotesInBloodHunt);
            if !favor {
                side += read(crate::cards::Stat::VotesAgainstBloodHunt);
            }
        }
        (self.votes_of(st, m) as i32 + side).max(0) as u8
    }

    /// "Cannot cast votes or ballots against blood hunt referendums"
    /// (Troius).
    pub(crate) fn barred_against(&self, st: &crate::state::GameState, m: MinionId) -> bool {
        matches!(self.kind, RefKind::BloodHunt { .. })
            && st.printed_mod(
                m,
                crate::cards::Stat::NoVotesAgainstBloodHunt,
                crate::state::PrintedCtx::default(),
            ) > 0
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

    /// (for, against) over the standing records, the prisci block's 3 votes
    /// to the side with more prisci ballots, none on a tie
    /// (rulebook "The Prisci Block"; rules-qa 52.9); a vampire's votes
    /// doubled while a ready vampire of its controller's doubles them, read
    /// as it is tallied (Orlando Oriundus).
    pub fn tally(&self, st: &crate::state::GameState) -> (u32, u32) {
        let doubles = |s: SeatId| {
            st.minions_of(s).any(|m| {
                st.printed_mod(
                    m,
                    crate::cards::Stat::DoublesControllersVotes,
                    crate::state::PrintedCtx::default(),
                ) > 0
            })
        };
        let live = self.records.iter().filter(|r| !r.canceled);
        let (mut f, mut a) = live.clone().fold((0, 0), |(f, a), r| {
            let k = if matches!(r.source, Source::Vampire(_)) && doubles(r.seat) {
                2
            } else {
                1
            };
            let n = r.n as u32 * k;
            if r.favor {
                (f + n, a)
            } else {
                (f, a + n)
            }
        });
        let ballots = live.fold((0u32, 0u32), |(f, a), r| {
            if r.favor {
                (f + r.ballots as u32, a)
            } else {
                (f, a + r.ballots as u32)
            }
        });
        match ballots.0.cmp(&ballots.1) {
            std::cmp::Ordering::Greater => f += 3,
            std::cmp::Ordering::Less => a += 3,
            std::cmp::Ordering::Equal => {}
        }
        (f, a)
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

/// Clans of the V5 crypt, in crypt order (Consanguineous Boon's choice);
/// an imbued's creed is none.
pub fn clans() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in CRYPT.iter().filter(|c| !c.imbued) {
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
            forcers: Vec::new(),
            card_voted: Vec::new(),
            only_sect: def.and_then(|d| d.only_sect),
            barred_sect: match kind {
                RefKind::Political => self.action.as_ref().and_then(|a| a.barred_vote_sect),
                RefKind::BloodHunt { .. } => None,
            },
            penalties: Vec::new(),
            pressure: None,
            auto_pass: false,
            pre_vote: kind == RefKind::Political,
            forced_fail: false,
            rider_clauses: Vec::new(),
            scorn: 0,
            poison: Vec::new(),
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
                    ballots: 0,
                });
            }
        }
        // Día de los Muertos: the first referendum a Sabbat vampire of
        // theirs calls this turn, if the card was played before it.
        if let Some(m) = ctx.calling {
            if self.state.sect_of(m) == Some(Sect::Sabbat) {
                let seat = &mut self.state.seats[caller.index()];
                if seat.auto_pass_pending && !seat.sabbat_referendum_called {
                    ctx.auto_pass = true;
                    ctx.window = PollingWindow::new(Vec::new());
                }
                seat.sabbat_referendum_called = true;
            }
        }
        // Cryptic Rider: the next referendum a vampire of theirs calls.
        if let Some(m) = ctx.calling {
            let seat = &mut self.state.seats[caller.index()];
            if (seat.auto_pass_next || seat.auto_pass_lasting)
                && self.state.minions[m.index()].is_vampire()
            {
                // The turn's (Cryptic Rider) used first: the lasting one
                // keeps for a later referendum.
                if seat.auto_pass_next {
                    seat.auto_pass_next = false;
                } else {
                    seat.auto_pass_lasting = false;
                }
                ctx.auto_pass = true;
                ctx.window = PollingWindow::new(Vec::new());
            }
        }
        // The pre-vote rotation: from the caller's left, the caller last.
        if ctx.auto_pass {
            ctx.pre_vote = false;
        } else if ctx.pre_vote {
            let mut order = self.state.clockwise_from(caller);
            order.rotate_left(1);
            ctx.window = PollingWindow::new(order);
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
            ctx.terms.term_start = ctx.terms.minions.len();
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
            Points::HalfActorCapacity => self
                .action
                .as_ref()
                .and_then(|a| self.state.capacity_of(a.acting))
                .unwrap_or(0)
                .div_ceil(2),
        }
    }

    /// The point allocation under way, whole (for a client; the decisions
    /// still ask one recipient at a time); `None` outside those steps.
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
            RefStep::Polling | RefStep::Choose => return None,
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
            // The returned card may put the hand over size: the engine's
            // discard-down comes first ("discard down afterward").
            return None;
        }
        match ctx.step {
            RefStep::Terms => Some(self.terms_step_request(ctx)),
            RefStep::Polling if ctx.pressure.is_some() => self.pressure_request(ctx),
            RefStep::Polling => self.polling_request(ctx),
            RefStep::Shortfall { points } => Some(self.shortfall_request(ctx, points)),
            RefStep::Choose => Some(self.choose_request(ctx)),
        }
    }

    /// Minions a `ChooseMinions` term may pick, arena order.
    fn minion_choices(&self, ctx: &ReferendumCtx, filter: &[cards::Requirement]) -> Vec<MinionId> {
        let st = &self.state;
        let acting = self.action.as_ref().map(|a| a.acting);
        // The action's card or granted mode, for Vlad Tepes's as if.
        let as_if = self.action.as_ref().is_some_and(|a| {
            let mode = match (a.card, a.source, a.ability) {
                (Some(p), _, _) => cards::lookup(p.card).map(|d| &d.modes[p.mode as usize]),
                (None, Some(src), Some(ab)) => Some(src.mode(st, ab)),
                _ => None,
            };
            mode.is_some_and(|mode| cards::meets_as_if(st, a.acting, mode))
        });
        (0..st.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(|&m| {
                st.minions[m.index()].present()
                    // A distribution picks a vampire again as often as
                    // wished, or up to its cap.
                    && match ctx.current_term() {
                        Some(cards::Term::DistributeBlood { per_max, .. }) => {
                            per_max == 0
                                || (ctx.terms.minions[ctx.terms.term_start..]
                                    .iter()
                                    .filter(|&&k| k == m)
                                    .count() as u8)
                                    < per_max
                        }
                        _ => !ctx.terms.minions.contains(&m),
                    }
                    && filter.iter().all(|&r| {
                        cards::term_filter_holds(
                            r,
                            st,
                            ctx.caller,
                            Some(ctx),
                            acting,
                            m,
                            self.action.as_ref(),
                            as_if,
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
            Term::ChooseMinionsX { filter } => {
                let x = self.action.as_ref().and_then(|a| a.card).map_or(0, |c| c.x);
                let left = self.minion_choices(ctx, filter);
                let mut options = Vec::new();
                if ctx.terms.minions.len() as u8 >= x || left.is_empty() {
                    options.push(Choice::Pass);
                }
                if (ctx.terms.minions.len() as u8) < x {
                    options.extend(left.into_iter().map(Choice::ChooseMinion));
                }
                options
            }
            Term::ChooseClan => clans().into_iter().map(Choice::ChooseClan).collect(),
            Term::ChooseCard(filter) => (0..st.cards.len())
                .filter(|&i| chooseable_card(st, crate::state::CardId(i as u32), filter))
                .map(|i| Choice::ChooseCard(i as u32))
                .collect(),
            Term::SeatOrCard { card, .. } => {
                let (seat, cards) = (ctx.terms.chosen.is_none(), ctx.terms.cards.is_empty());
                let mut options = Vec::new();
                // The second of the two is optional.
                if !(seat && cards) {
                    options.push(Choice::Pass);
                }
                if seat {
                    options.extend(
                        st.clockwise_from(ctx.caller)
                            .into_iter()
                            .map(Choice::ChooseSeat),
                    );
                }
                if cards {
                    options.extend(
                        (0..st.cards.len())
                            .filter(|&i| chooseable_card(st, crate::state::CardId(i as u32), card))
                            .map(|i| Choice::ChooseCard(i as u32)),
                    );
                }
                options
            }
            Term::DistributeBlood { filter, .. } => self
                .minion_choices(ctx, filter)
                .into_iter()
                .map(Choice::ChooseMinion)
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

    /// Business Pressure's round: the first Methuselah still to choose
    /// names the pool they burn (0: none), then the side of its votes.
    fn pressure_request(&self, ctx: &ReferendumCtx) -> Option<DecisionRequest> {
        let p = ctx.pressure.as_ref().expect("pressure round");
        let seat = *p.pending.first()?;
        let options = match p.amount {
            None => {
                let pool = self.state.seats[seat.index()]
                    .pool
                    .max(0)
                    .min(u8::MAX as i32) as u8;
                (0..=pool).map(Choice::SetX).collect()
            }
            Some(_) => vec![
                Choice::CastCardVote { favor: true },
                Choice::CastCardVote { favor: false },
            ],
        };
        Some(DecisionRequest::new(
            seat,
            WindowLabel::ReferendumPolling,
            options,
        ))
    }

    fn pressure_apply(&mut self, ctx: &mut ReferendumCtx, choice: &Choice) {
        let p = ctx.pressure.as_mut().expect("pressure round");
        let seat = p.pending[0];
        match (*choice, p.amount) {
            (Choice::SetX(0), None) => {
                p.pending.remove(0);
            }
            (Choice::SetX(n), None) => p.amount = Some(n),
            (Choice::CastCardVote { favor }, Some(n)) => {
                p.picks.push((seat, n, favor));
                p.amount = None;
                p.pending.remove(0);
            }
            _ => unreachable!("illegal pressure choice {choice:?}"),
        }
        if !p.pending.is_empty() {
            return;
        }
        // All have chosen: the votes and the pool burned at once (the
        // last pool ousts, simultaneous ousts as a batch).
        let p = ctx.pressure.take().expect("pressure round");
        let mut loss = vec![0i32; self.state.seats.len()];
        for &(s, n, favor) in &p.picks {
            ctx.records.push(VoteRecord {
                source: Source::Pool(s),
                seat: s,
                favor,
                n: n.saturating_mul(p.rate(s)),
                canceled: false,
                ballots: 0,
            });
            loss[s.index()] += n as i32;
        }
        let votes = Cause::Cost(notes::Source::Rule(Rule::Vote));
        self.caused(votes, |e| e.burn_pool_batch(&loss));
        for (i, seat) in self.state.seats.iter().enumerate() {
            if seat.ousted {
                ctx.window.remove(SeatId(i as u8));
            }
        }
    }

    fn polling_request(&self, ctx: &ReferendumCtx) -> Option<DecisionRequest> {
        let st = &self.state;
        let seat = ctx.window.current()?;
        if ctx.pre_vote {
            return Some(self.pre_vote_request(ctx, seat));
        }
        let mut options = vec![Choice::Pass];
        for id in st.minions_of(seat) {
            if ctx.spent(Source::Vampire(id)) {
                continue;
            }
            // A priscus casts its prisci ballot even with no vote.
            let ballot = ctx.may_vote(st, id) && st.ballots(id) > 0;
            if (ctx.votes_cast(st, id, true) > 0 || ballot) && !ctx.forced_against.contains(&id) {
                options.push(Choice::CastVotes {
                    minion: id,
                    favor: true,
                });
            }
            if (ctx.votes_cast(st, id, false) > 0 || ballot && !ctx.barred_against(st, id))
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
        // One political-action-card vote per Methuselah (rulebook "Gaining
        // Votes"); the calling card is the acting one's (rules-qa 5.4): they
        // burn no other (owner, 2026-09-30: rules-qa 46.2).
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

    /// The pre-vote rotation's decision: the plays and abilities "only
    /// usable … before any votes and ballots are cast", or pass.
    fn pre_vote_request(&self, ctx: &ReferendumCtx, seat: SeatId) -> DecisionRequest {
        let st = &self.state;
        let before_votes =
            |m: &cards::ModeDef| m.requires.contains(&cards::Requirement::NoVotesCast);
        let windows: &[Window] = if seat == ctx.caller {
            &[Window::Referendum]
        } else {
            &[Window::Referendum, Window::Wake]
        };
        let mut options = vec![Choice::Pass];
        options.extend(
            self.play_options(seat, windows, None)
                .into_iter()
                .filter(|o| match *o {
                    Choice::Play { card, mode, .. } => {
                        cards::lookup(self.virtual_hand(seat)[card as usize].0)
                            .is_some_and(|d| before_votes(&d.modes[mode as usize]))
                    }
                    _ => false,
                }),
        );
        options.extend(
            self.ability_options(seat, Window::Referendum)
                .into_iter()
                .filter(|o| match *o {
                    Choice::UseAbility {
                        source, ability, ..
                    } => before_votes(source.mode(st, ability)),
                    _ => false,
                }),
        );
        DecisionRequest::new(seat, WindowLabel::ReferendumPolling, options)
    }

    pub(crate) fn referendum_apply(&mut self, choice: &Choice) {
        match self.action.as_ref().map(|a| a.cause_source()) {
            Some(source) => self.caused(Cause::Effect(source), |e| e.referendum_apply_now(choice)),
            None => self.referendum_apply_now(choice),
        }
    }

    fn referendum_apply_now(&mut self, choice: &Choice) {
        let mut ctx = self.referendum.take().expect("referendum context");
        match ctx.step {
            RefStep::Terms => self.terms_step_apply(&mut ctx, choice),
            RefStep::Polling if ctx.pressure.is_some() => {
                self.pressure_apply(&mut ctx, choice);
                // The acting Methuselah burned their last pool: the action
                // is over (rules-qa 51.5).
                if self
                    .action
                    .as_ref()
                    .is_some_and(|a| self.state.seats[a.seat.index()].ousted)
                {
                    self.acting_seat_ousted();
                    return;
                }
            }
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
                    let seat = ctx.window.current().expect("commit on closed window");
                    ctx.window.commit();
                    self.referendum = Some(ctx);
                    self.flow = Flow::Referendum;
                    self.use_ability(seat, *source, *ability, *actor, *target, Flow::Referendum);
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
            RefStep::Choose => {
                // Kindred Segregation: allies kept one by one, until a pass.
                let keeps = ctx
                    .def
                    .is_some_and(|d| d.effect == RefEffect::BurnAlliesUnlessKept);
                let seat = ctx.terms.pending[0];
                match *choice {
                    Choice::ChooseMinion(m) => ctx.terms.picks.push((seat, m)),
                    Choice::Pass => {}
                    _ => unreachable!("illegal vampire choice {choice:?}"),
                }
                if !keeps || *choice == Choice::Pass {
                    ctx.terms.pending.remove(0);
                }
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
            (Term::ChooseMinions { .. } | Term::ChooseMinionsX { .. }, Choice::Pass) => true,
            (Term::ChooseMinionsX { .. }, Choice::ChooseMinion(m)) => {
                ctx.terms.minions.push(*m);
                let x = self.action.as_ref().and_then(|a| a.card).map_or(0, |c| c.x);
                ctx.terms.minions.len() as u8 >= x
            }
            (Term::DistributeBlood { points, .. }, Choice::ChooseMinion(m)) => {
                ctx.terms.minions.push(*m);
                (ctx.terms.minions.len() - ctx.terms.term_start) as u8 >= self.points_of(points)
            }
            (Term::ChooseClan, Choice::ChooseClan(c)) => {
                ctx.terms.clan = Some(c);
                true
            }
            (Term::ChooseCard(_), Choice::ChooseCard(c)) => {
                ctx.terms.cards.push(crate::state::CardId(*c));
                true
            }
            (Term::SeatOrCard { both, .. }, c) => {
                match *c {
                    Choice::ChooseSeat(s) => ctx.terms.chosen = Some(s),
                    Choice::ChooseCard(card) => ctx.terms.cards.push(crate::state::CardId(card)),
                    Choice::Pass => {}
                    _ => unreachable!("illegal seat-or-card choice {c:?}"),
                }
                // Both only for the titles named, read as the term is set.
                let titled = ctx
                    .calling
                    .is_some_and(|m| both.iter().any(|t| t.matches(self.state.title_of(m))));
                *c == Choice::Pass
                    || !titled
                    || (ctx.terms.chosen.is_some() && !ctx.terms.cards.is_empty())
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
                if let Some(record) = self.vampire_cast(ctx, *minion, *favor, true) {
                    ctx.records.push(record);
                }
                ctx.window.commit();
                return None;
            }
            // "Burning the Edge is worth 1 additional vote" (Eat the Rich).
            Choice::CastEdge { favor } => {
                self.state.seats[seat.index()].has_edge = false;
                let bonus = ctx.def.map_or(0, |d| d.edge_bonus) as i32
                    + self
                        .state
                        .seat_printed(seat, crate::cards::Stat::EdgeVoteBonus);
                (Source::Edge(seat), *favor, (1 + bonus).max(0) as u8)
            }
            Choice::CastCardVote { favor } => {
                ctx.card_voted.push(seat);
                (Source::Card(seat), *favor, 1)
            }
            Choice::BurnForVote { card, favor } => {
                let s = &mut self.state.seats[seat.index()];
                let (c, copy) = s.hand.take(*card as usize);
                s.ash.put(c, copy);
                self.replace(seat, 1, None);
                ctx.card_voted.push(seat);
                let bonus = self
                    .state
                    .seat_printed(seat, crate::cards::Stat::BurnVoteBonus);
                (Source::Card(seat), *favor, (1 + bonus).max(0) as u8)
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
            ballots: 0,
        });
        ctx.window.commit();
        None
    }

    /// Vampire `m`'s cast on `favor`'s side, its toll against paid: its
    /// votes and, a priscus, its prisci ballot on the same side ("each
    /// priscus gets -1 ballot", Eat the Rich: a cast of nothing still
    /// spends the source). None if it may cast only against, or cannot pay
    /// the toll.
    fn vampire_cast(
        &mut self,
        ctx: &ReferendumCtx,
        m: MinionId,
        favor: bool,
        first: bool,
    ) -> Option<VoteRecord> {
        // A toll on votes against: the first cast only, not a turned one
        // (ANK 20221019).
        let toll = if favor || !first {
            0
        } else {
            self.vote_against_toll(ctx, m)
        };
        let n = ctx.votes_cast(&self.state, m, favor);
        let priscus = self.state.title_of(m) == crate::crypt::Title::Priscus;
        let minus = u8::from(ctx.def.is_some_and(|d| d.titled_minus) && priscus);
        let ballots =
            if ctx.may_vote(&self.state, m) && !(!favor && ctx.barred_against(&self.state, m)) {
                self.state.ballots(m).saturating_sub(minus)
            } else {
                0
            };
        if (favor && ctx.forced_against.contains(&m)) || self.state.minions[m.index()].blood < toll
        {
            return None;
        }
        self.state.minions[m.index()].blood -= toll;
        Some(VoteRecord {
            source: Source::Vampire(m),
            seat: self.state.minions[m.index()].owner,
            favor,
            n,
            canceled: false,
            ballots,
        })
    }

    /// The polling window closed (or the referendum was canceled): tally
    /// and resolve.
    pub(crate) fn referendum_advance(&mut self) {
        let source = match self.action.as_ref() {
            Some(a) => a.cause_source(),
            None => notes::Source::Rule(Rule::Diablerie),
        };
        self.caused(Cause::Effect(source), Self::referendum_advance_now)
    }

    fn referendum_advance_now(&mut self) {
        let mut ctx = self.referendum.take().expect("referendum context");
        // The pre-vote rotation over: the polling proper, from the caller.
        if ctx.pre_vote && !ctx.fizzled && !ctx.canceled {
            ctx.pre_vote = false;
            ctx.window = PollingWindow::new(self.state.clockwise_from(ctx.caller));
            self.referendum = Some(ctx);
            self.flow = Flow::Referendum;
            return;
        }
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
        let (favor, against) = ctx.tally(&self.state);
        let passed = (favor > against || ctx.auto_pass) && !ctx.forced_fail;
        // Malkavian Rider Clause: no "this turn" — until it is used.
        if passed {
            for &s in &ctx.rider_clauses {
                self.state.seats[s.index()].auto_pass_lasting = true;
            }
        }
        // "The attached vampire burns 1 blood after a referendum passes"
        // (Condemnation: Mute [DAI]): any referendum.
        if passed {
            let hook = cards::Hook::AnyReferendumPassed;
            let on = |c: &crate::state::CardInPlay| c.bearer().is_some();
            for (id, r) in self.fired_riders(hook, &HookAt::on(&on)) {
                let host = self.state.cards[id.index()].bearer().expect("its host");
                for &e in r.effects {
                    match e {
                        cards::Effect::HostBurnBlood(n) => {
                            if self.state.burn_blood_or_life(host, n) {
                                self.lifeless(host);
                            }
                        }
                        other => self.apply_rider_effect(id, other, hook),
                    }
                }
            }
        }
        match ctx.kind {
            RefKind::BloodHunt { diablerist } => {
                if passed {
                    // Genina: to torpor instead of burned.
                    let torpor = self.state.printed_mod(
                        diablerist,
                        cards::Stat::TorporInsteadOfBloodHuntBurn,
                        crate::state::PrintedCtx::default(),
                    ) > 0;
                    if torpor {
                        self.minions_to_torpor(&[diablerist]);
                    } else {
                        self.burn_minion(diablerist);
                    }
                }
                // Part of the action (spec §3.7.8.1): the after-resolution
                // window follows it (rules-qa 5.8).
                self.after_referendum(true);
            }
            RefKind::Political => {
                {
                    let a = self.action.as_mut().expect("political action");
                    a.referendum_passed = passed;
                    a.margin = if passed && !ctx.auto_pass {
                        favor - against
                    } else {
                        0
                    };
                }
                // Scorn of Adonis: paid at the tally, before the effects.
                if ctx.scorn > 0 {
                    let mut loss = vec![0i32; self.state.seats.len()];
                    for r in ctx.records.iter().filter(|r| !r.canceled && !r.favor) {
                        loss[r.seat.index()] = ctx.scorn as i32;
                    }
                    self.burn_pool_batch(&loss);
                }
                // Hartmut Stover: not a cost (no blood, none burned), and
                // Methuselah ballots burn nothing (rules-qa 49.2).
                if let Some(c) = ctx.calling {
                    let n = self
                        .state
                        .printed_mod(
                            c,
                            cards::Stat::AgainstCastersBurnBlood,
                            crate::state::PrintedCtx::default(),
                        )
                        .max(0) as u8;
                    if n > 0 {
                        for r in ctx.records.iter().filter(|r| !r.canceled && !r.favor) {
                            if let Source::Vampire(m) = r.source {
                                if self.state.minions[m.index()].present() {
                                    self.state.burn_blood(m, n);
                                }
                            }
                        }
                    }
                }
                // Marcos Belegrad: each Methuselah casting against, by any
                // source of theirs, burns pool once results are tallied —
                // once each (rules-qa 51.1).
                if let Some(c) = ctx.calling {
                    let n = self
                        .state
                        .printed_mod(
                            c,
                            cards::Stat::AgainstMethuselahsBurnPool,
                            crate::state::PrintedCtx::default(),
                        )
                        .max(0);
                    if n > 0 {
                        let mut loss = vec![0i32; self.state.seats.len()];
                        for r in ctx.records.iter().filter(|r| !r.canceled && !r.favor) {
                            loss[r.seat.index()] = n;
                        }
                        self.burn_pool_batch(&loss);
                    }
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
                let keeps = def.effect == RefEffect::BurnAlliesUnlessKept;
                if passed && (def.effect.chooses_vampires() || keeps) {
                    ctx.step = RefStep::Choose;
                    ctx.terms.pending = self
                        .state
                        .clockwise_from(ctx.caller)
                        .into_iter()
                        .filter(|&s| {
                            if keeps {
                                !self.allies_of(s).is_empty()
                            } else {
                                !self.ready_vampires_of(s).is_empty()
                            }
                        })
                        .collect();
                    if !ctx.terms.pending.is_empty() {
                        self.referendum = Some(ctx);
                        self.flow = Flow::Referendum;
                        return;
                    }
                }
                self.resolve_political(ctx);
            }
        }
    }

    /// The ready vampires `s` controls, arena order (a choice of
    /// `RefStep::Choose`).
    fn ready_vampires_of(&self, s: SeatId) -> Vec<MinionId> {
        self.state
            .minions_of(s)
            .filter(|&m| {
                let v = &self.state.minions[m.index()];
                v.is_vampire() && v.ready()
            })
            .collect()
    }

    /// The allies `s` controls, arena order (Kindred Segregation's
    /// choice).
    fn allies_of(&self, s: SeatId) -> Vec<MinionId> {
        self.state
            .minions_of(s)
            .filter(|&m| self.state.minions[m.index()].is_ally())
            .collect()
    }

    /// An ally's pool cost to recruit (Kindred Segregation): its card's
    /// pool cost — 0 for a blood cost, or an ally with no card (RTR
    /// 20010711, LSJ 20060409).
    pub fn ally_pool_cost(&self, m: MinionId) -> i32 {
        match self.state.minions[m.index()]
            .kind
            .ally_card()
            .map(|c| crate::library::LIBRARY[c as usize].pool_cost)
        {
            Some(crate::library::Cost::Fixed(n)) => n as i32,
            _ => 0,
        }
    }

    /// The pool `s` has committed to keep allies so far (Kindred
    /// Segregation), paid as the referendum resolves.
    fn kept_cost(&self, ctx: &ReferendumCtx, s: SeatId) -> i32 {
        ctx.terms
            .picks
            .iter()
            .filter(|(p, _)| *p == s)
            .map(|&(_, m)| self.ally_pool_cost(m))
            .sum()
    }

    fn choose_request(&self, ctx: &ReferendumCtx) -> DecisionRequest {
        let seat = ctx.terms.pending[0];
        let options = if ctx
            .def
            .is_some_and(|d| d.effect == RefEffect::BurnAlliesUnlessKept)
        {
            // Each ally kept in turn, while the pool covers its cost (down
            // to 0: the cost ousts).
            let left = self.state.seats[seat.index()].pool - self.kept_cost(ctx, seat);
            std::iter::once(Choice::Pass)
                .chain(
                    self.allies_of(seat)
                        .into_iter()
                        .filter(|m| !ctx.terms.picks.iter().any(|(_, k)| k == m))
                        .filter(|&m| self.ally_pool_cost(m) <= left)
                        .map(Choice::ChooseMinion),
                )
                .collect()
        } else {
            std::iter::once(Choice::Pass)
                .chain(
                    self.ready_vampires_of(seat)
                        .into_iter()
                        .map(Choice::ChooseMinion),
                )
                .collect()
        };
        DecisionRequest::new(seat, WindowLabel::ReferendumTerms, options)
    }

    fn resolve_political(&mut self, ctx: ReferendumCtx) {
        let passed = self
            .action
            .as_ref()
            .expect("political action")
            .referendum_passed;
        if passed {
            let acting = self.action.as_ref().expect("political action").acting;
            self.state.minions[acting.index()].referendum_passed_turn = true;
            let def = ctx.def.expect("political definition");
            self.apply_referendum_effect(def.effect, &ctx);
        }
        // Alvaro "abstained": no vote or ballot standing, title or not
        // (LSJ 20010526); never worse to take, so gained (rules-qa 53.5).
        for i in 0..self.state.minions.len() {
            let m = MinionId(i as u32);
            let v = &self.state.minions[m.index()];
            if !v.present()
                || ctx
                    .records
                    .iter()
                    .any(|r| !r.canceled && r.source == Source::Vampire(m))
            {
                continue;
            }
            let n = self.state.printed_mod(
                m,
                cards::Stat::BloodIfAbstained,
                crate::state::PrintedCtx::default(),
            );
            if n > 0 {
                self.state.gain_blood(m, n as u8);
            }
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
                // A gain then a loss, the loss in all cases (ANK
                // 20180813); no choice gains (or burns) nothing (rules-qa
                // 49.1). The capacity is read as it resolves.
                RefEffect::GainChosenCapacityBurn(n) => {
                    for s in st.clockwise_from(ctx.caller) {
                        loss[s.index()] += n as i32;
                    }
                    for &(s, m) in &ctx.terms.picks {
                        gain[s.index()] += st.capacity_of(m).unwrap_or(0) as i32;
                    }
                }
                RefEffect::GainBurnPredatorsChosenCapacity(n) => {
                    for s in st.clockwise_from(ctx.caller) {
                        gain[s.index()] += n as i32;
                        let predator = st.predator(s);
                        for &(p, m) in &ctx.terms.picks {
                            if p == predator && p != s {
                                loss[s.index()] += st.capacity_of(m).unwrap_or(0) as i32;
                            }
                        }
                    }
                }
                RefEffect::GainPerVampireCapacityAtLeast { cap, n } => {
                    for (i, m) in st.minions.iter().enumerate() {
                        if m.present()
                            && m.is_vampire()
                            && st.capacity_of(MinionId(i as u32)).is_some_and(|c| c >= cap)
                        {
                            gain[m.owner.index()] += n as i32;
                        }
                    }
                }
                RefEffect::GainPerMinion(n) => {
                    for m in st.minions.iter().filter(|m| m.present()) {
                        gain[m.owner.index()] += n as i32;
                    }
                }
                RefEffect::Banish
                | RefEffect::Expel
                | RefEffect::BurnSource
                | RefEffect::BurnAlliesUnlessKept
                | RefEffect::BurnBloodPerPick
                | RefEffect::PutOnChosen { .. }
                | RefEffect::PutOnCaller { .. }
                | RefEffect::PutInPlay
                | RefEffect::ChosenTakesChosenCard
                | RefEffect::RemoveChosen
                | RefEffect::ChosenVampireTakesChosenCard
                | RefEffect::ChosenShuffleFromAsh(_) => {}
                RefEffect::CallerGainsPool(n) => gain[ctx.caller.index()] += n as i32,
                RefEffect::BloodToChosenEach
                | RefEffect::MoveDistributedFromFirst
                | RefEffect::CallerHandSizeForGame(_)
                | RefEffect::CallerContestsChosenTitle
                | RefEffect::BurnVampiresBleedAtLeast(_)
                | RefEffect::UnlockCallerAndChosen => {}
                RefEffect::BurnEdgeToUntitle(n) => {
                    if st.seats[ctx.caller.index()].has_edge {
                        if let Some(&m) = ctx.terms.minions.first() {
                            if st.minions[m.index()].in_play {
                                loss[st.minions[m.index()].owner.index()] += n as i32;
                            }
                        }
                    }
                }
                RefEffect::ChosenBurnsPoolCardBurned(n) => {
                    if let Some(s) = ctx.terms.chosen {
                        loss[s.index()] += n as i32;
                    }
                }
                RefEffect::BurnChosenPlusIfTitled { base, bonus } => {
                    for &s in &ctx.terms.seats {
                        loss[s.index()] += base as i32;
                        if st.minions_of(s).any(|m| {
                            st.minions[m.index()].is_vampire()
                                && st.minions[m.index()].ready()
                                && st.title_of(m) != crate::crypt::Title::None
                        }) {
                            loss[s.index()] += bonus as i32;
                        }
                    }
                }
                RefEffect::BurnChosenPlusIfBig { base, bonus, cap } => {
                    for &s in &ctx.terms.seats {
                        loss[s.index()] += base as i32;
                        if st.minions_of(s).any(|m| {
                            st.minions[m.index()].is_vampire()
                                && st.minions[m.index()].ready()
                                && st.capacity_of(m).is_some_and(|c| c >= cap)
                        }) {
                            loss[s.index()] += bonus as i32;
                        }
                    }
                }
            }
        }
        for (i, g) in gain.iter().enumerate() {
            if !self.state.seats[i].ousted {
                self.state.seats[i].pool += g;
            }
        }
        match effect {
            RefEffect::UnlockCallerAndChosen => {
                let caller = self.action.as_ref().map(|a| a.acting);
                for m in caller.into_iter().chain(ctx.terms.minions.iter().copied()) {
                    if self.state.minions[m.index()].present() {
                        self.effect_unlock(m);
                    }
                }
            }
            RefEffect::BurnVampiresBleedAtLeast(n) => {
                let burned: Vec<MinionId> = (0..self.state.minions.len())
                    .map(|i| MinionId(i as u32))
                    .filter(|&m| {
                        // In torpor too (owner, 2026-10-09: rules-qa 59.9).
                        let v = &self.state.minions[m.index()];
                        v.present() && v.is_vampire() && self.state.current_bleed(m) >= n
                    })
                    .collect();
                for m in burned {
                    self.burn_minion(m);
                }
            }
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
            // Free States Rant: in torpor too, as it passes.
            RefEffect::BurnBloodPerPick => {
                for &m in &ctx.terms.minions {
                    if self.state.minions[m.index()].present() {
                        self.state.burn_blood(m, 1);
                    }
                }
            }
            RefEffect::MoveDistributedFromFirst => {
                if let [from, picks @ ..] = &ctx.terms.minions[..] {
                    let v = &self.state.minions[from.index()];
                    let to: Vec<MinionId> = picks
                        .iter()
                        .copied()
                        .filter(|t| self.state.minions[t.index()].present())
                        .collect();
                    if v.ready() {
                        if (v.blood as usize) >= to.len() {
                            for t in to {
                                self.state.burn_blood(*from, 1);
                                self.state.gain_blood(t, 1);
                            }
                        } else {
                            let mut slots = [None; 8];
                            debug_assert!(
                                to.len() <= slots.len(),
                                "more blood allotted than slots"
                            );
                            for (s, t) in slots.iter_mut().zip(to) {
                                *s = Some(t);
                            }
                            self.interrupts
                                .push(crate::draws::Interrupt::ShortfallPicks {
                                    seat: ctx.caller,
                                    from: *from,
                                    to: slots,
                                });
                        }
                    }
                }
            }
            RefEffect::BloodToChosenEach => {
                for &m in &ctx.terms.minions {
                    if self.state.minions[m.index()].ready() {
                        self.state.gain_blood(m, 1);
                    }
                }
            }
            RefEffect::CallerContestsChosenTitle => {
                let caller = self.action.as_ref().map(|a| a.acting);
                if let (Some(c), Some(&m)) = (caller, ctx.terms.minions.first()) {
                    let title = self.state.title_of(m);
                    if self.state.minions[c.index()].present()
                        && cards::TitleClass::Prince.matches(title)
                    {
                        self.gain_title(c, title, None);
                    }
                }
            }
            RefEffect::CallerHandSizeForGame(n) => {
                self.state.seats[ctx.caller.index()].timed.add(
                    TimedStat::HandSize,
                    Expiry::Never,
                    n as i8,
                );
            }
            // The pool was burned with the batch, the Edge read the same.
            RefEffect::BurnEdgeToUntitle(_) => {
                if self.state.seats[ctx.caller.index()].has_edge {
                    self.state.seats[ctx.caller.index()].has_edge = false;
                    if let Some(&m) = ctx.terms.minions.first() {
                        if self.state.minions[m.index()].in_play {
                            self.lose_title(m);
                        }
                    }
                }
            }
            RefEffect::Expel => {
                for &m in &ctx.terms.minions {
                    self.state.minions[m.index()].expelled = true;
                }
            }
            // The equipment moves onto the chosen vampire, if both are
            // still there and it can hold it (rules-qa 51.3).
            RefEffect::ChosenVampireTakesChosenCard => {
                let (c, m) = (ctx.terms.cards[0], ctx.terms.minions[0]);
                let card = self.state.cards[c.index()].card;
                if self.state.cards[c.index()].present()
                    && self.state.minions[m.index()].present()
                    && self.state.cards[c.index()].host != Some(m)
                    && !self.state.pinned(c)
                    && self.state.can_hold(m, card, &[])
                {
                    let owner = self.state.minions[m.index()].owner;
                    self.state.set_host(c, m);
                    self.state.cards[c.index()].controller = owner;
                }
            }
            RefEffect::ChosenBurnsPoolCardBurned(_) => {
                if let Some(&c) = ctx.terms.cards.first() {
                    if self.state.cards[c.index()].present() {
                        self.burn_card_in_play(c);
                    }
                }
            }
            RefEffect::ChosenShuffleFromAsh(n) => {
                for &s in &ctx.terms.seats {
                    if !self.state.seats[s.index()].ousted {
                        self.interrupts.push(crate::draws::Interrupt::AshurPicks {
                            seat: s,
                            left: n,
                            to_library: true,
                        });
                    }
                }
            }
            RefEffect::RemoveChosen => {
                let m = ctx.terms.minions[0];
                if self.state.minions[m.index()].present() {
                    self.remove_ally_from_game(m);
                }
            }
            RefEffect::PutOnChosen { .. } | RefEffect::PutOnCaller { .. } => {
                let a = self.action.as_ref().expect("political action");
                let m = match effect {
                    RefEffect::PutOnCaller { .. } => a.acting,
                    _ => ctx.terms.minions[0],
                };
                let (seat, play) = (a.seat, a.card.expect("a political action card"));
                if self.state.minions[m.index()].present() {
                    self.action
                        .as_mut()
                        .expect("political action")
                        .card_entered_play = true;
                    self.place_now(
                        crate::state::CardInPlay::entering(
                            play.card,
                            *play.copy,
                            seat,
                            Some(m),
                            play.mode,
                        ),
                        ModeLayers::Push,
                        false,
                    );
                    // Alastor: the equipment next; "finding equipment is
                    // optional" (LSJ 20050331-2).
                    if effect == (RefEffect::PutOnChosen { equipment: true }) {
                        self.interrupts
                            .push(crate::draws::Interrupt::SearchEquipFor { seat, minion: m });
                    }
                    if let RefEffect::PutOnCaller {
                        unlocks_if: Some(clan),
                    } = effect
                    {
                        self.state.minions[m.index()].unlock_at_discard_if = Some(clan);
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
                // "A Methuselah can have only one" (Raising the
                // Portcullis): not a second.
                let one = cards::lookup(play.card).is_some_and(|d| {
                    d.modes[play.mode as usize]
                        .requires
                        .contains(&cards::Requirement::OnePerMethuselah)
                });
                if !(one && self.state.controls_card_copy(seat, play.card)) {
                    // The Methuselahs its terms chose, remembered (Rumors of
                    // Gehenna, The Meddling of Semsith).
                    let mut against = crate::ids::SeatSet::EMPTY;
                    for &s in ctx.terms.seats.iter().chain(ctx.terms.chosen.iter()) {
                        against.0 |= crate::ids::SeatSet::single(s).0;
                    }
                    self.place_now(
                        crate::state::CardInPlay {
                            against,
                            ..crate::state::CardInPlay::entering(
                                play.card, *play.copy, seat, None, play.mode,
                            )
                        },
                        ModeLayers::Push,
                        false,
                    );
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
            // Kept allies paid for as a batch (ousts with the referendum's),
            // then every other ally burned, in torpor too.
            RefEffect::BurnAlliesUnlessKept => {
                let mut cost = vec![0i32; self.state.seats.len()];
                for s in self.state.clockwise_from(ctx.caller) {
                    cost[s.index()] = self.kept_cost(ctx, s);
                }
                self.burn_pool_batch(&cost);
                for i in 0..self.state.minions.len() {
                    let m = MinionId(i as u32);
                    let v = &self.state.minions[i];
                    if v.present() && v.is_ally() && !ctx.terms.picks.iter().any(|&(_, k)| k == m) {
                        self.burn_minion(m);
                    }
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
        // Poison Pill: the losses only, never offset by a gain (ANK
        // 20180813, LSJ 20041025); any decrease counts (ANK 20180719-2).
        for &p in &ctx.poison {
            loss[ctx.caller.index()] += loss[p.index()];
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
        let mut burned = Vec::new();
        for (i, &n) in loss.iter().enumerate() {
            let seat = &mut self.state.seats[i];
            if seat.ousted || n <= 0 {
                continue;
            }
            // Below 0 is kept until the oust: a Life Boon must make it good
            // (RTR 19970630).
            seat.pool -= n;
            burned.push(SeatId(i as u8));
            if seat.pool <= 0 {
                ousted.push(SeatId(i as u8));
            }
        }
        for s in burned {
            self.note_pool_burned(s);
        }
        if !ousted.is_empty() {
            self.oust_batch(&ousted);
        }
    }

    /// Move a vampire to `to`'s uncontrolled region (Banishment, Pit of
    /// Contemplation), keeping its blood and cards (rules-qa 25.1).
    pub(crate) fn banish(&mut self, m: MinionId, to: SeatId) {
        self.about_to_leave_ready(&[m]);
        let v = &mut self.state.minions[m.index()];
        let was_ready = v.ready();
        v.in_play = false;
        let card = v.crypt_card().unwrap_or(crate::state::CREATED);
        let copy = *v.copy;
        let blood = v.blood;
        v.blood = 0;
        // The cards on it go out of play until it returns, a created
        // vampire's own card too.
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
                copy: crate::copies::Untracked(copy),
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

    /// The acting Methuselah was ousted during their action: on their turn
    /// the turn passes, else (Enkil Cog) the active one's phase goes on.
    pub(crate) fn acting_seat_ousted(&mut self) {
        let seat = self.drop_action().expect("action").seat;
        self.release_action_scope();
        self.referendum = None;
        if self.state.living() <= 1 {
            self.flow = Flow::GameOver;
        } else if seat == self.state.active {
            self.next_turn();
        } else {
            self.flow = Flow::MinionPhase;
        }
    }

    /// The referendum concluded: on to the action's after-resolution window
    /// (`riders`: the failed-or-canceled riders apply, not if fizzled).
    fn after_referendum(&mut self, riders: bool) {
        if self.state.living() <= 1 {
            self.drop_action();
            self.flow = Flow::GameOver;
            return;
        }
        self.political_passed_riders();
        if self.state.living() <= 1 {
            self.drop_action();
            self.flow = Flow::GameOver;
            return;
        }
        let a = self.action.as_ref().expect("action");
        let (seat, blocked) = (a.seat, a.outcome == Some(Outcome::Blocked));
        if self.state.seats[seat.index()].ousted {
            self.acting_seat_ousted();
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

    /// Lutz: his controller's prey burns 1 pool after any political
    /// referendum passes, after all its effects, not for a controller they
    /// ousted (ANK 20220808); the prey read then.
    fn political_passed_riders(&mut self) {
        let a = self.action.as_ref().expect("action");
        if a.kind != crate::decision::ActionKind::Political || !a.referendum_passed {
            return;
        }
        let mut loss = vec![0i32; self.state.seats.len()];
        let mut burns = Vec::new();
        for m in 0..self.state.minions.len() {
            let v = &self.state.minions[m];
            if !v.present() {
                continue;
            }
            let s = v.owner;
            let seat = &self.state.seats[s.index()];
            if seat.ousted || seat.oust_pending {
                continue;
            }
            let hook = cards::Hook::PoliticalReferendumPassed;
            for r in self.printed_riders(MinionId(m as u32), hook, None) {
                for e in r.effects {
                    match *e {
                        cards::Effect::PreyBurnsPool(n) => {
                            loss[self.state.prey(s).index()] += n as i32;
                        }
                        cards::Effect::HostBurnBlood(n) => burns.push((MinionId(m as u32), n)),
                        other => unreachable!("not a referendum rider effect: {other:?}"),
                    }
                }
            }
        }
        if loss.iter().any(|&n| n > 0) {
            self.burn_pool_batch(&loss);
        }
        for (m, n) in burns {
            self.state.burn_blood(m, n);
        }
    }

    /// "If the referendum of a political action called by this vampire is
    /// canceled or fails" riders (Cedrick Calhoun), applied as the action
    /// resolves, before the after-resolution window.
    fn own_referendum_failed_riders(&mut self) {
        let a = self.action.as_ref().expect("action");
        if a.kind != crate::decision::ActionKind::Political {
            return;
        }
        let (m, seat) = (a.acting, a.seat);
        // "After a referendum called by Juliet passes" (Juliet Parr).
        let hook = if a.referendum_passed {
            cards::Hook::OwnReferendumPassed
        } else {
            cards::Hook::OwnReferendumFailed
        };
        if !self.state.minions[m.index()].ready() {
            return;
        }
        for r in self.printed_riders(m, hook, None) {
            for e in r.effects {
                match *e {
                    cards::Effect::GoToTorpor => self.minions_to_torpor(&[m]),
                    cards::Effect::HandSizeUntilUnlock(n) => {
                        self.state.seats[seat.index()].timed.add(
                            TimedStat::HandSize,
                            Expiry::UnlockPhase,
                            n,
                        );
                    }
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
            ballots: 0,
        });
    }

    /// Effects of a modifier / reaction resolved during the polling step.
    pub(crate) fn apply_vote_effect(&mut self, effect: cards::Effect, play: &cards::PlayCtx) {
        let mut ctx = self.referendum.take().expect("referendum context");
        match effect {
            cards::Effect::AddVotes(_)
            | cards::Effect::AddVotesPerReadySect(_)
            | cards::Effect::AddVotesXLinear { .. } => {
                let n = match effect {
                    cards::Effect::AddVotesXLinear { mul, plus } => mul * play.x + plus,
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
            cards::Effect::PoisonPill => ctx.poison.push(play.seat),
            cards::Effect::CastTargetVotes(favor) => {
                let m = play.target.minion();
                ctx.forcers.extend(play.actor);
                // A cast already made is turned: cast again on the side
                // chosen (its printed side votes; no toll again, ANK
                // 20221019), the votes gained since it was cast kept.
                let old = ctx
                    .records
                    .iter()
                    .position(|r| r.source == Source::Vampire(m) && !r.canceled);
                let extra = old.map_or(0, |i| {
                    let r = ctx.records[i];
                    r.n.saturating_sub(ctx.votes_cast(&self.state, m, r.favor))
                });
                let previous = old.map(|i| ctx.records.remove(i));
                match self.vampire_cast(&ctx, m, favor, previous.is_none()) {
                    Some(mut record) => {
                        record.n += extra;
                        // Forced, a vampire with nothing to cast casts nothing.
                        if record.n > 0 || record.ballots > 0 {
                            ctx.records.push(record);
                        }
                    }
                    // The new side unpayable: its cast stands as it was.
                    None => ctx.records.extend(previous),
                }
            }
            cards::Effect::GrantVotesAll(g) => {
                for r in ctx.records.iter_mut().filter(|r| !r.canceled) {
                    if let Source::Vampire(m) = r.source {
                        if g.voters.reach(&self.state, m) {
                            r.n += g.n;
                        }
                    }
                }
                ctx.grants.push((None, g));
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
            cards::Effect::BarNonCamarillaVotes => ctx.only_sect = Some(Sect::Camarilla),
            cards::Effect::NextReferendumPassesIfPasses => ctx.rider_clauses.push(play.seat),
            cards::Effect::BarNonSectVotes(sect) => ctx.only_sect = Some(sect),
            cards::Effect::PoolForVotesRound { per, own } => {
                let order = self.state.clockwise_from(play.seat);
                ctx.pressure = Some(Pressure {
                    player: play.seat,
                    per,
                    own,
                    pending: order,
                    amount: None,
                    picks: Vec::new(),
                });
            }
            // Votes already cast lose it too, down to 0 (ANK 20251125).
            cards::Effect::NonCamarillaVotesMinus(_) | cards::Effect::NotPathVotesMinus(..) => {
                let (who, n) = match effect {
                    cards::Effect::NonCamarillaVotesMinus(n) => {
                        (cards::Penalized::NotSect(Sect::Camarilla), n)
                    }
                    cards::Effect::NotPathVotesMinus(path, n) => {
                        (cards::Penalized::NotPath(path), n)
                    }
                    _ => unreachable!(),
                };
                // Not on the vampire under Perfect Clarity, from a
                // reaction (owner, 2026-10-14: rules-qa 71.25).
                let spared = self.clarity_shielded(play.card, play.mode as usize, play.actor);
                ctx.penalties.push((who, n, spared));
                // Votes, not ballots (advanced rules §3.7.6.1).
                for r in ctx.records.iter_mut().filter(|r| !r.canceled) {
                    if let Source::Vampire(m) = r.source {
                        if who.reaches(&self.state, m) && spared != Some(m) {
                            r.n = r.n.saturating_sub(n);
                        }
                    }
                }
            }
            // "Have the referendum fail" (Yoruba Shrine): the polling is
            // over.
            cards::Effect::FailReferendum => {
                ctx.forced_fail = true;
                ctx.window = PollingWindow::new(Vec::new());
            }
            // Confusion of the Eye [OBF]: the acting vampire casts no votes
            // or ballots.
            cards::Effect::ActingAbstains => {
                if let Some(t) = self.action.as_ref().map(|a| a.acting) {
                    for r in &mut ctx.records {
                        if r.source == Source::Vampire(t) {
                            r.canceled = true;
                        }
                    }
                    ctx.abstaining.push(t);
                }
            }
            // "If the referendum requires a titled vampire, the referendum
            // fails": a title requirement on the calling card or granted
            // action (rules-qa 63.3).
            cards::Effect::FailIfTitleRequired => {
                let titled = self.action.as_ref().is_some_and(|a| {
                    let mode = match (a.card, a.source, a.ability) {
                        (Some(p), _, _) => {
                            cards::lookup(p.card).and_then(|d| d.modes.get(p.mode as usize))
                        }
                        (None, Some(src), Some(ab)) => Some(src.mode(&self.state, ab)),
                        _ => None,
                    };
                    mode.is_some_and(|m| {
                        m.requires.iter().any(|r| {
                            matches!(
                                r,
                                cards::Requirement::ActorTitleIn(_)
                                    | cards::Requirement::ActorClaimsTitleIn(_)
                            )
                        })
                    })
                });
                if titled {
                    ctx.forced_fail = true;
                    ctx.window = PollingWindow::new(Vec::new());
                }
            }
            cards::Effect::CancelReferendum {
                unlock_acting,
                forbid_repeat,
            } => {
                ctx.canceled = true;
                let a = self.action.as_mut().expect("political action");
                // The political action card returns to its owner's hand
                // (before it is burned: LSJ 20050324) — not one removed
                // from the game whatever happens (Reinforcements: rules-qa
                // 51.6).
                if let Some(card) = a
                    .card
                    .filter(|c| !cards::removed_once_played(c.card, c.mode))
                {
                    // Its owner's: the prey's card played from The Erciyes
                    // Fragments goes back to them (rules-qa 60.19).
                    let owner = card
                        .from
                        .and_then(|f| self.state.cards[f.index()].stored_owner)
                        .unwrap_or(a.seat);
                    self.state.seats[owner.index()]
                        .hand
                        .put(card.card, *card.copy);
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
                    self.state.unlock(a.acting);
                }
            }
            other => unreachable!("not a referendum effect: {other:?}"),
        }
        self.referendum = Some(ctx);
    }
}
