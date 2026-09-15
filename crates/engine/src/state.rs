use crate::cards::Layer;
use crate::crypt::{CryptCard, Title, CRYPT};
use crate::ids::{MinionId, SeatId};
use crate::library::LIBRARY;

pub const START_POOL: i32 = 30;
pub const UNCONTROLLED_START: usize = 4;
pub const HAND_SIZE: usize = 7;

/// CRYPT index of the vampire with this printed name (test helper).
pub fn find_card(name: &str) -> u16 {
    CRYPT
        .iter()
        .position(|c| c.name == name)
        .unwrap_or_else(|| panic!("no V5 crypt card named {name:?}")) as u16
}

/// LIBRARY index of the library card with this printed name (test helper).
pub fn find_library(name: &str) -> u16 {
    LIBRARY
        .iter()
        .position(|c| c.name == name)
        .unwrap_or_else(|| panic!("no V5 library card named {name:?}")) as u16
}

/// Contest identity of a title (rulebook "Contested Titles" / "Vampiric
/// Sects"): prince, baron (and archbishop) of one city contest each other;
/// each clan's justicar is unique; primogen is not unique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleKey {
    City(&'static str),
    Justicar(&'static str),
}

pub fn title_key(t: Title) -> Option<TitleKey> {
    match t {
        Title::Prince(c) | Title::Baron(c) => Some(TitleKey::City(c)),
        Title::Justicar(k) => Some(TitleKey::Justicar(k)),
        Title::None | Title::Primogen => None,
    }
}

pub fn title_votes(t: Title) -> u8 {
    match t {
        Title::None => 0,
        Title::Primogen => 1,
        Title::Prince(_) | Title::Baron(_) => 2,
        Title::Justicar(_) => 3,
    }
}

/// A vampire in play. `contested`: the card is contested — face down and out
/// of play, blood kept (rulebook "Contested Cards"). `title_contested` /
/// `title_lost`: the printed title is under contest / was yielded for the
/// rest of the game. `bled_this_turn` is the NRA special cap (rules-qa 1.4).
#[derive(Clone, Debug)]
pub struct Minion {
    pub owner: SeatId,
    pub card: u16,
    pub blood: u8,
    pub locked: bool,
    pub in_torpor: bool,
    pub in_play: bool,
    pub contested: bool,
    pub title_contested: bool,
    pub title_lost: bool,
    pub bled_this_turn: bool,
    /// One political action per minion per turn (rulebook "Political
    /// Action"; NRA on resolution, rules-qa 1.4).
    pub political_this_turn: bool,
    /// Expulsion: cannot play reaction cards, block or cast votes or
    /// ballots this turn.
    pub expelled: bool,
    /// Gained blood from a hunting ground this turn ("a vampire can gain
    /// blood from only one hunting ground each turn").
    pub hunting_ground_fed: bool,
    /// "Does not unlock as normal during their next unlock phase" (Stolen
    /// Police Cruiser): cleared by that unlock phase.
    pub skip_unlock: bool,
    /// Actions this minion performed this turn that reached resolution
    /// (NRA, rules-qa 1.4): a card from hand by name, a card in play by
    /// copy; bleeds and political actions are capped by type besides.
    pub acted: Vec<ActionId>,
}

/// Identity of an action for the no-repeat-action rule (spec §3.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    /// An action card played from hand: same printed name.
    Card(u16),
    /// An action granted by a card in play: the same copy.
    InPlay(CardId),
}

impl ActionId {
    pub fn same(self, other: ActionId) -> bool {
        match (self, other) {
            (ActionId::Card(a), ActionId::Card(b)) => {
                LIBRARY[a as usize].name == LIBRARY[b as usize].name
            }
            (ActionId::InPlay(a), ActionId::InPlay(b)) => a == b,
            _ => false,
        }
    }
}

impl Minion {
    pub fn card(&self) -> &'static CryptCard {
        &CRYPT[self.card as usize]
    }

    pub fn name(&self) -> &'static str {
        self.card().name
    }

    pub fn capacity(&self) -> u8 {
        self.card().capacity
    }

    /// Controlled and present: in play and not contested (a contested copy
    /// is out of play until the contest ends).
    pub fn present(&self) -> bool {
        self.in_play && !self.contested
    }

    /// Ready: present and not in torpor (locked or not).
    pub fn ready(&self) -> bool {
        self.present() && !self.in_torpor
    }

    /// Effective title: none while contested, yielded, or out of play.
    pub fn title(&self) -> Title {
        if self.title_lost || self.title_contested || !self.present() {
            Title::None
        } else {
            self.card().title
        }
    }

    /// Votes this vampire can cast right now (title votes need a ready,
    /// uncontested holder; rulebook "Gaining Votes").
    pub fn votes(&self) -> u8 {
        if self.ready() {
            title_votes(self.title())
        } else {
            0
        }
    }
}

/// A library card in play (a master on a vampire, a location, …).
#[derive(Clone, Debug)]
pub struct CardInPlay {
    pub card: u16,
    /// The Methuselah who played it: where it goes when it leaves play.
    pub owner: SeatId,
    /// The Methuselah controlling it — its player until a control change
    /// (The Coven); for a minion card on a minion see `card_controller`.
    pub controller: SeatId,
    /// Minion the card is on, if any.
    pub host: Option<MinionId>,
    pub in_play: bool,
    /// Contested (rulebook "Contested Cards"): face down and out of play —
    /// nothing printed on it applies, its layers are suspended (spec
    /// §1.13.3) — until paid for to the end or yielded.
    pub contested: bool,
    pub locked: bool,
    pub counters: u8,
    /// Library cards on this card, out of play (Black Market Cache face
    /// up, Shilmulo Tarot face down), top last: their owner's, burned with
    /// the card (LSJ 20021128).
    pub stored: Vec<u16>,
}

impl CardInPlay {
    /// In play and controlled: not contested.
    pub fn present(&self) -> bool {
        self.in_play && !self.contested
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardId(pub u32);

impl CardId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug)]
pub struct UncontrolledSlot {
    pub card: u16,
    pub counters: u8,
    /// A banished vampire (Banishment): the arena entry it left, with its
    /// cards face down on it; it comes back as that entry when influenced
    /// out again (rules-qa 5.7).
    pub banished: Option<MinionId>,
}

/// An out-of-turn master card played since the seat's last master phase
/// (rulebook "Out-of-turn"; rules-qa 3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfTurn {
    pub trifle: bool,
}

/// Cards played but not replaced yet, by the moment their replacement is
/// drawn (spec §1.9.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held {
    /// "Do not replace until your next unlock phase" (Old Friends).
    pub unlock: u8,
    /// "Do not replace until your next discard phase" (Mirror Walk).
    pub discard: u8,
    /// Replaced at the end of the current action (Consign to Oblivion
    /// [OBL]).
    pub action: u8,
}

impl Held {
    pub fn total(self) -> usize {
        self.unlock as usize + self.discard as usize + self.action as usize
    }
}

#[derive(Clone, Debug)]
pub struct Seat {
    pub pool: i32,
    pub ousted: bool,
    /// Victory points in half-point units (tournament rules 3.7: 0.5 VP for
    /// surviving at the time limit).
    pub half_vps: u32,
    pub has_edge: bool,
    /// Remaining crypt cards (CRYPT indices), top at the end.
    pub crypt: Vec<u16>,
    pub uncontrolled: Vec<UncontrolledSlot>,
    /// Library (LIBRARY indices), top at the end; hand; ash heap.
    pub library: Vec<u16>,
    pub hand: Vec<u16>,
    pub ash: Vec<u16>,
    /// Cards of this Methuselah removed from the game while another
    /// controlled them (spec §1.10: removed cards are public).
    pub removed: Vec<u16>,
    /// Master phase actions left in the current master phase.
    pub mpa: u8,
    /// Hand size above the rulebook's 7 until the end of the turn (Dreams
    /// of the Sphinx).
    pub hand_bonus: u8,
    /// One additional master phase action from trifles per master phase.
    pub trifle_used: bool,
    /// Out-of-turn master card played since the last master phase.
    pub out_of_turn: Option<OutOfTurn>,
    /// Discard phase actions left in the current discard phase.
    pub dpa: u8,
    pub turns_taken: u32,
    /// Cards played but not replaced yet: they count against the hand
    /// size until then (rulebook "Drawing Cards"; spec §6.8.1).
    pub held: Held,
    /// Political actions the seat's minions cannot perform again this
    /// turn (Delaying Tactics): a card by name, a granted action by copy
    /// (LSJ 20020201).
    pub forbidden_political: Vec<ActionId>,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub seats: Vec<Seat>,
    /// Arena: minions are never removed, only marked out of play.
    pub minions: Vec<Minion>,
    /// Arena of library cards in play.
    pub cards: Vec<CardInPlay>,
    /// Abilities (card in play, ability index) used this phase ("during
    /// X": once per phase per copy, rules-qa 6.6); cleared at each phase.
    pub phase_uses: Vec<(CardId, u8)>,
    /// Continuous effects (DESIGN §4.5 scope layers).
    pub layers: Vec<Layer>,
    pub active: SeatId,
    /// Game turn counter, 1-based once the first turn starts.
    pub turn: u32,
}

impl GameState {
    /// The game win (tournament rules 3.7; rules-qa 6.9): the Methuselah
    /// with at least 2 VP and strictly more than every other, ousted or
    /// not; `None` on a draw.
    pub fn game_win(&self) -> Option<SeatId> {
        let best = self.seats.iter().map(|s| s.half_vps).max()?;
        if best < 4 {
            return None;
        }
        let mut leaders = self
            .seats
            .iter()
            .enumerate()
            .filter(|(_, s)| s.half_vps == best);
        let (i, _) = leaders.next()?;
        leaders.next().is_none().then_some(SeatId(i as u8))
    }

    pub fn living(&self) -> usize {
        self.seats.iter().filter(|s| !s.ousted).count()
    }

    /// Living seats clockwise starting at `from` (inclusive).
    pub fn clockwise_from(&self, from: SeatId) -> Vec<SeatId> {
        let n = self.seats.len();
        (0..n)
            .map(|k| SeatId(((from.index() + k) % n) as u8))
            .filter(|s| !self.seats[s.index()].ousted)
            .collect()
    }

    /// Next unousted seat clockwise. Returns `s` itself if it is the sole
    /// survivor.
    pub fn prey(&self, s: SeatId) -> SeatId {
        let n = self.seats.len();
        let mut i = (s.index() + 1) % n;
        while self.seats[i].ousted && i != s.index() {
            i = (i + 1) % n;
        }
        SeatId(i as u8)
    }

    pub fn predator(&self, s: SeatId) -> SeatId {
        let n = self.seats.len();
        let mut i = (s.index() + n - 1) % n;
        while self.seats[i].ousted && i != s.index() {
            i = (i + n - 1) % n;
        }
        SeatId(i as u8)
    }

    /// Present (in play, uncontested) minions of a seat, arena order.
    pub fn minions_of(&self, s: SeatId) -> impl Iterator<Item = MinionId> + '_ {
        self.minions
            .iter()
            .enumerate()
            .filter(move |(_, m)| m.owner == s && m.present())
            .map(|(i, _)| MinionId(i as u32))
    }

    /// Ready and unlocked: eligible to act normally or to attempt a block.
    pub fn ready_unlocked(&self, s: SeatId) -> Vec<MinionId> {
        self.minions_of(s)
            .filter(|m| {
                let m = &self.minions[m.index()];
                !m.locked && !m.in_torpor
            })
            .collect()
    }

    /// Hand indices of the seat's cards — master cards only with
    /// `masters`.
    pub fn hand_picks(&self, seat: SeatId, masters: bool) -> Vec<u8> {
        self.seats[seat.index()]
            .hand
            .iter()
            .enumerate()
            .filter(|(_, &c)| {
                !masters
                    || crate::library::LIBRARY[c as usize]
                        .types
                        .contains(&crate::library::CardType::Master)
            })
            .map(|(i, _)| i as u8)
            .collect()
    }

    /// The Methuselah controlling a card in play: a master card stays its
    /// controller's wherever it sits (Fame on another Methuselah's
    /// vampire; The Coven once taken); a minion card on a minion is its
    /// host's controller's (owner, 2026-09-06; spec §1.12.1).
    pub fn card_controller(&self, id: CardId) -> SeatId {
        let c = &self.cards[id.index()];
        let master = LIBRARY[c.card as usize]
            .types
            .contains(&crate::library::CardType::Master);
        match c.host {
            Some(h) if !master => self.minions[h.index()].owner,
            _ => c.controller,
        }
    }

    /// A copy of this unique card is in play under `seat`, contested or not:
    /// another is not playable, nor equipped by a search ("you cannot
    /// voluntarily contest cards with yourself", rulebook "Contested
    /// Cards"; spec §1.13.1; rules-qa 7.1).
    pub fn controls_unique_copy(&self, seat: SeatId, card: u16) -> bool {
        LIBRARY[card as usize]
            .labels
            .contains(&crate::library::Label::Unique)
            && self.cards.iter().enumerate().any(|(i, c)| {
                c.in_play
                    && self.card_controller(CardId(i as u32)) == seat
                    && LIBRARY[c.card as usize].name == LIBRARY[card as usize].name
            })
    }

    /// Hand size (rulebook "Drawing Cards"): seven, the until-end-of-turn
    /// bonus (Dreams of the Sphinx), and the hand-size layers of the
    /// uncontested cards in play the seat controls (Elder Library; Visit
    /// from the Capuchin, one per counter; spec §1.13.3: a contested
    /// location gives no hand size).
    pub fn hand_size(&self, s: SeatId) -> usize {
        let mut size = HAND_SIZE as i32 + self.seats[s.index()].hand_bonus as i32;
        for l in &self.layers {
            if l.scope != crate::cards::Scope::InPlay
                || !self.layer_active(l)
                || self.card_controller(l.source) != s
            {
                continue;
            }
            match l.modifier {
                crate::cards::Modifier::HandSize(n) => size += n as i32,
                crate::cards::Modifier::HandSizePerCounter => {
                    size += self.cards[l.source.index()].counters as i32
                }
                _ => {}
            }
        }
        size.max(0) as usize
    }

    /// Cards over hand size, the cards not replaced yet counting against
    /// it: discarded down at once (rulebook "Drawing Cards"; spec §6.8.1).
    pub fn hand_excess(&self, s: SeatId) -> usize {
        let seat = &self.seats[s.index()];
        (seat.hand.len() + seat.held.total()).saturating_sub(self.hand_size(s))
    }

    /// A rider's card and host predicates hold (a hostless card has no
    /// host predicate to fail).
    pub fn rider_holds(&self, r: &crate::cards::Rider, id: CardId) -> bool {
        let c = &self.cards[id.index()];
        r.requires.iter().all(|&req| match req {
            crate::cards::Requirement::CountersAtLeast(n) => c.counters >= n,
            _ => match c.host {
                Some(h) => crate::cards::holds_for_host(req, self, h),
                None => true,
            },
        })
    }

    /// Does minion `m` have a card in play of this kind on it?
    pub fn holds_a(&self, m: MinionId, h: crate::cards::Holding) -> bool {
        self.cards
            .iter()
            .any(|c| c.in_play && c.host == Some(m) && holding_matches(c.card, h))
    }

    /// Could minion `m` take equipment `card` as well — none of the card's
    /// "a minion can have only one X" limits reached by what it has or by
    /// the cards `with` it takes in the same action?
    pub fn can_hold(&self, m: MinionId, card: u16, with: &[u16]) -> bool {
        let Some(def) = crate::cards::lookup(card) else {
            return true;
        };
        def.modes
            .iter()
            .flat_map(|mode| mode.requires.iter())
            .all(|r| match *r {
                crate::cards::Requirement::ActorWithout(h) => {
                    !self.holds_a(m, h) && !with.iter().any(|&w| holding_matches(w, h))
                }
                _ => true,
            })
    }

    /// Equipment on minion `m` (uncontested cards in play of the
    /// equipment type).
    pub fn equipment_on(&self, m: MinionId) -> Vec<CardId> {
        self.cards
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.present()
                    && c.host == Some(m)
                    && LIBRARY[c.card as usize]
                        .types
                        .contains(&crate::library::CardType::Equipment)
            })
            .map(|(i, _)| CardId(i as u32))
            .collect()
    }

    /// What the cards in play on minion `m` add to `stat` during its
    /// action of `kind` (`Modifier::Bearer`; contested cards give
    /// nothing).
    pub fn bearer_mod(
        &self,
        m: MinionId,
        stat: crate::cards::Stat,
        kind: crate::decision::ActionKind,
    ) -> i32 {
        self.layers
            .iter()
            .filter(|l| {
                l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && self.cards[l.source.index()].host == Some(m)
            })
            .filter_map(|l| match l.modifier {
                crate::cards::Modifier::Bearer {
                    stat: s,
                    n,
                    during,
                    who,
                } if s == stat
                    && (during.is_empty() || during.contains(&kind))
                    && who.is(self, m) =>
                {
                    Some(n as i32)
                }
                _ => None,
            })
            .sum()
    }

    /// Vampires in torpor, any controller — rescue and diablerie targets.
    pub fn in_torpor(&self) -> Vec<MinionId> {
        self.minions
            .iter()
            .enumerate()
            .filter(|(_, m)| m.present() && m.in_torpor)
            .map(|(i, _)| MinionId(i as u32))
            .collect()
    }

    /// Does `s` already have a copy of this vampire in play, in any state
    /// (rulebook: no voluntary self-contest)?
    pub fn controls_copy(&self, s: SeatId, card: u16) -> bool {
        let name = CRYPT[card as usize].name;
        self.minions
            .iter()
            .any(|m| m.owner == s && m.in_play && m.name() == name)
    }

    /// Other seats' in-play copies of `m`'s vampire (contest rivals).
    pub fn card_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let me = &self.minions[m.index()];
        self.minions
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != m.index()
                    && o.owner != me.owner
                    && o.in_play
                    && !self.seats[o.owner.index()].ousted
                    && o.name() == me.name()
            })
            .map(|(i, _)| MinionId(i as u32))
            .collect()
    }

    /// Other seats' in-play copies (contested included) of the unique
    /// library card `id`: its contest rivals.
    pub fn library_rivals(&self, id: CardId) -> Vec<CardId> {
        let me = &self.cards[id.index()];
        let name = LIBRARY[me.card as usize].name;
        let mine = self.card_controller(id);
        self.cards
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != id.index()
                    && o.in_play
                    && self.card_controller(CardId(*i as u32)) != mine
                    && LIBRARY[o.card as usize].name == name
            })
            .map(|(i, _)| CardId(i as u32))
            .collect()
    }

    /// A layer applies: action-scoped, or sourced by a card in play that
    /// is not contested (a contested card's effects are suspended).
    pub fn layer_active(&self, l: &Layer) -> bool {
        l.scope == crate::cards::Scope::Action || self.cards[l.source.index()].present()
    }

    /// Other present vampires (any seat) claiming a title that contests
    /// `m`'s printed title.
    pub fn title_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let me = &self.minions[m.index()];
        let Some(key) = title_key(me.card().title) else {
            return Vec::new();
        };
        self.minions
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != m.index()
                    && o.present()
                    && !o.title_lost
                    && !self.seats[o.owner.index()].ousted
                    && title_key(o.card().title) == Some(key)
            })
            .map(|(i, _)| MinionId(i as u32))
            .collect()
    }

    /// FNV-1a over a canonical serialization; used for replay equivalence.
    /// Every field that can influence a later decision must be mixed in.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |x: u64| {
            h ^= x;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        };
        mix(self.active.0 as u64);
        mix(self.turn as u64);
        for s in &self.seats {
            mix(s.pool as u64);
            mix(s.ousted as u64);
            mix(s.half_vps as u64);
            mix(s.has_edge as u64);
            mix(s.turns_taken as u64);
            mix(s.mpa as u64);
            mix(s.hand_bonus as u64);
            mix(s.trifle_used as u64);
            mix(s.dpa as u64);
            mix(s.held.unlock as u64);
            mix(s.held.discard as u64);
            mix(s.held.action as u64);
            mix(s.forbidden_political.len() as u64);
            for a in &s.forbidden_political {
                mix(match a {
                    ActionId::Card(c) => *c as u64,
                    ActionId::InPlay(c) => 1 << 32 | c.0 as u64,
                });
            }
            mix(match s.out_of_turn {
                None => 0,
                Some(OutOfTurn { trifle }) => 1 + trifle as u64,
            });
            for zone in [
                &s.crypt,
                &s.uncontrolled_cards(),
                &s.library,
                &s.hand,
                &s.ash,
            ] {
                mix(zone.len() as u64);
                for &c in zone.iter() {
                    mix(c as u64);
                }
            }
            for u in &s.uncontrolled {
                mix(u.counters as u64);
                mix(u.banished.map_or(0, |m| m.0 as u64 + 1));
            }
        }
        mix(self.minions.len() as u64);
        for m in &self.minions {
            mix(m.owner.0 as u64);
            mix(m.card as u64);
            mix(m.blood as u64);
            mix(m.locked as u64);
            mix(m.in_torpor as u64);
            mix(m.in_play as u64);
            mix(m.contested as u64);
            mix(m.title_contested as u64);
            mix(m.title_lost as u64);
            mix(m.bled_this_turn as u64);
            mix(m.political_this_turn as u64);
            mix(m.expelled as u64);
            mix(m.hunting_ground_fed as u64);
            mix(m.skip_unlock as u64);
            mix(m.acted.len() as u64);
            for &c in &m.acted {
                mix(match c {
                    ActionId::Card(c) => c as u64,
                    ActionId::InPlay(id) => (id.0 as u64) << 20,
                });
            }
        }
        mix(self.phase_uses.len() as u64);
        for (c, a) in &self.phase_uses {
            mix((c.0 as u64 + 1) << 8 ^ *a as u64);
        }
        mix(self.cards.len() as u64);
        for c in &self.cards {
            mix(c.card as u64);
            mix(c.owner.0 as u64);
            mix(c.controller.0 as u64);
            mix(c.host.map(|m| m.0 as u64 + 1).unwrap_or(0));
            mix(c.in_play as u64);
            mix(c.contested as u64);
            mix(c.locked as u64);
            mix(c.counters as u64);
        }
        mix(self.layers.len() as u64);
        for l in &self.layers {
            mix(l.hash());
        }
        h
    }
}

fn holding_matches(card: u16, h: crate::cards::Holding) -> bool {
    match h {
        crate::cards::Holding::Label(l) => LIBRARY[card as usize].labels.contains(&l),
        crate::cards::Holding::Named(n) => LIBRARY[card as usize].name == n,
    }
}

impl Seat {
    fn uncontrolled_cards(&self) -> Vec<u16> {
        self.uncontrolled.iter().map(|u| u.card).collect()
    }
}
