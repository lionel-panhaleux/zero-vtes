use crate::cards::Layer;
use crate::crypt::{CryptCard, Sect, Title, CRYPT};
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
    /// A unique title a card names (Saulot's Guiding Wisdom).
    Named(&'static str),
}

pub fn title_key(t: Title) -> Option<TitleKey> {
    match t {
        Title::Prince(c) | Title::Baron(c) => Some(TitleKey::City(c)),
        Title::Justicar(k) => Some(TitleKey::Justicar(k)),
        Title::Independent { name, .. } => Some(TitleKey::Named(name)),
        Title::None | Title::Primogen => None,
    }
}

pub fn title_votes(t: Title) -> u8 {
    match t {
        Title::None => 0,
        Title::Primogen => 1,
        Title::Prince(_) | Title::Baron(_) => 2,
        Title::Justicar(_) => 3,
        Title::Independent { votes, .. } => votes,
    }
}

/// What a vampire's printed modifier reads besides the game state
/// (`GameState::printed_mod`): the action performed, if any, and the
/// minion opposing the modified one — the acting minion for a block
/// attempt, the opposing combatant in combat.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrintedCtx {
    pub kind: Option<crate::decision::ActionKind>,
    pub directed: Option<bool>,
    pub opponent: Option<MinionId>,
}

/// What a minion is: a vampire (its crypt card) or an ally — a library card
/// in play (`id`, whose `CardInPlay::body` points back here) that is a
/// minion in its own right (rulebook "Ally cards"), with the type line of
/// the version that entered play (`body`: its starting life, strength and
/// bleed; advanced rules 5.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinionKind {
    Vampire(u16),
    Ally {
        card: u16,
        id: CardId,
        body: crate::library::Body,
    },
}

impl MinionKind {
    /// The ally's LIBRARY index; `None` for a vampire.
    pub fn ally_card(self) -> Option<u16> {
        match self {
            MinionKind::Ally { card, .. } => Some(card),
            MinionKind::Vampire(_) => None,
        }
    }
}

/// A minion in play: a vampire, or an ally. `contested`: the card is
/// contested — face down and out of play, blood kept (rulebook "Contested
/// Cards"). `title_contested`: the title it claims is under contest;
/// `title_lost`: the printed title is gone for the rest of the game
/// (yielded, stolen, or replaced by a title gained since: rulebook
/// "Vampiric Sects", one title per vampire). `granted_title`: a title
/// gained with no card to represent it (Valerius's steal); a title card
/// on the vampire represents its own (`GameState::claimed_title`). `bled_this_turn` is the
/// NRA special cap (rules-qa 1.4).
#[derive(Clone, Debug)]
pub struct Minion {
    pub owner: SeatId,
    pub kind: MinionKind,
    /// A vampire's blood; an ally's life (advanced rules 5.5.1: an ally
    /// has life, not blood, and never pays a blood cost).
    pub blood: u8,
    pub locked: bool,
    pub in_torpor: bool,
    pub in_play: bool,
    pub contested: bool,
    pub title_contested: bool,
    pub title_lost: bool,
    pub granted_title: Option<Title>,
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
    /// Recruited this turn: an ally cannot act the turn it is recruited
    /// (rulebook "Recruit Ally"; one put in play otherwise can, advanced
    /// rules 5.5.4). Cleared as each turn begins.
    pub recruited: bool,
    /// Successful recruit ally actions this turn (Spectral Servitor
    /// unlocks its recruiter on the first).
    pub recruits_this_turn: u8,
    /// Card names (LIBRARY indices) played under
    /// `Limit::BetweenUnlockPhases`, cleared in its controller's unlock
    /// phase.
    pub played_since_unlock: Vec<u16>,
    /// "This ally cannot gain life (instead, any life it would gain goes
    /// to the blood bank)" (Aggressive Corpse, Rotting Behemoth).
    pub no_life_gain: bool,
    /// "Unlock this vampire during the next discard phase" (Feral Hound
    /// [ani]): cleared by that phase.
    pub unlock_at_discard: bool,
}

/// Identity of an action for the no-repeat-action rule (spec §3.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    /// An action card played from hand: same printed name.
    Card(u16),
    /// An action granted by a card in play: the same copy.
    InPlay(CardId),
    /// A vampire's printed action, by vampire and ability index: never the
    /// same as a card's or a rulebook action (spec §3.8.1). The vampire is
    /// kept for records held by the seat (a forbidden political action).
    Printed(MinionId, u8),
    /// A rulebook action and its target minion, recorded only where a card
    /// forbids repeating it ("this minion cannot perform the same action
    /// again this turn", Change of Target): rulebook actions are otherwise
    /// repeatable (LSJ 20060522).
    Rulebook(crate::decision::ActionKind, Option<MinionId>),
}

impl ActionId {
    pub fn same(self, other: ActionId) -> bool {
        match (self, other) {
            (ActionId::Card(a), ActionId::Card(b)) => {
                LIBRARY[a as usize].name == LIBRARY[b as usize].name
            }
            (ActionId::InPlay(a), ActionId::InPlay(b)) => a == b,
            (ActionId::Printed(m, a), ActionId::Printed(n, b)) => m == n && a == b,
            (ActionId::Rulebook(k, a), ActionId::Rulebook(l, b)) => k == l && a == b,
            _ => false,
        }
    }
}

impl Minion {
    /// The vampire's crypt card; `None` for an ally (no clan, sect,
    /// Disciplines, capacity or title).
    pub fn vampire(&self) -> Option<&'static CryptCard> {
        match self.kind {
            MinionKind::Vampire(c) => Some(&CRYPT[c as usize]),
            MinionKind::Ally { .. } => None,
        }
    }

    /// The vampire's CRYPT index (printed text lookup); `None` for an ally.
    pub fn crypt_card(&self) -> Option<u16> {
        match self.kind {
            MinionKind::Vampire(c) => Some(c),
            MinionKind::Ally { .. } => None,
        }
    }

    pub fn is_ally(&self) -> bool {
        matches!(self.kind, MinionKind::Ally { .. })
    }

    /// The ally's card in play; `None` for a vampire.
    pub fn ally_id(&self) -> Option<CardId> {
        match self.kind {
            MinionKind::Ally { id, .. } => Some(id),
            MinionKind::Vampire(_) => None,
        }
    }

    /// Printed name: the contest identity of a vampire or unique ally.
    pub fn name(&self) -> &'static str {
        match self.kind {
            MinionKind::Vampire(c) => CRYPT[c as usize].name,
            MinionKind::Ally { card, .. } => LIBRARY[card as usize].name,
        }
    }

    /// A vampire's capacity; `None` for an ally, which has none (its life
    /// is not capped, advanced rules 5.5.2).
    pub fn capacity(&self) -> Option<u8> {
        self.vampire().map(|v| v.capacity)
    }

    /// A vampire younger than `than` (lower capacity); an ally is no
    /// vampire, so neither younger nor older.
    pub fn younger_than(&self, than: &Minion) -> bool {
        matches!((self.capacity(), than.capacity()), (Some(a), Some(b)) if a < b)
    }

    /// "Allies and younger vampires" (Stolen Police Cruiser, Perfect
    /// Paragon): an ally, or a vampire of capacity below `than`.
    pub fn ally_or_younger(&self, than: u8) -> bool {
        self.capacity().is_none_or(|c| c < than)
    }

    /// Blood or life gained "not to exceed starting life" (Seraphina): a
    /// vampire's capacity, an ally's starting life — never below what it
    /// already has.
    pub fn cap_starting(&self, blood: u8) -> u8 {
        let start = match self.kind {
            MinionKind::Vampire(c) => CRYPT[c as usize].capacity,
            MinionKind::Ally { body, .. } => body.life,
        };
        if self.no_life_gain {
            return blood.min(self.blood);
        }
        blood.min(start.max(self.blood))
    }

    /// Room left under capacity for blood gained: unlimited for an ally —
    /// none for one that cannot gain life.
    pub fn cap_blood(&self, blood: u8) -> u8 {
        if self.no_life_gain {
            return blood.min(self.blood);
        }
        match self.capacity() {
            Some(c) => blood.min(c),
            None => blood,
        }
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
    /// An ally: the minion this card is (not a card on that minion — its
    /// controller is the minion's).
    pub body: Option<MinionId>,
    /// The mode it entered play with: its version, for a card whose
    /// superior version differs (the Discipline level used to employ or
    /// recruit it decides, RTR 19941109).
    pub mode: u8,
    /// The minion that played it, when its text names it ("this acting
    /// vampire", Slaughtering the Herd [DOM]).
    pub by: Option<MinionId>,
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
    /// A lingering effect of a card that went to the ash heap as it resolved
    /// (Touch of Oblivion's counters, Sculpt the Flesh [PRO]): represented
    /// here, while the card itself stays in the ash heap; burning it ends
    /// the effect, and moves no card (owner, 2026-09-26: rules-qa 21.1).
    pub effect_only: bool,
}

impl CardInPlay {
    /// In play and controlled: not contested.
    pub fn present(&self) -> bool {
        self.in_play && !self.contested
    }

    /// The minion whose card text this is, for its `Actor::Host`
    /// abilities: the minion it is on, or the ally it is.
    pub fn bearer(&self) -> Option<MinionId> {
        self.host.or(self.body)
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
    pub banished: Option<Banished>,
}

/// A banished vampire's arena entry and the cards that were in play on it
/// as it left — face down on it, out of play, until it returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Banished {
    pub minion: MinionId,
    pub cards: Vec<CardId>,
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
    /// Cards of this Methuselah removed from the game (while another
    /// controlled them, or from its ash heap: Rotting Behemoth; spec §1.10:
    /// removed cards are public).
    pub removed: Vec<u16>,
    /// Burned vampires (CRYPT indices): the ash heap's crypt cards.
    pub ash_crypt: Vec<u16>,
    /// Vampires removed from the game (CRYPT indices).
    pub removed_crypt: Vec<u16>,
    /// Master phase actions left in the current master phase.
    pub mpa: u8,
    /// Hand size above the rulebook's 7 until the end of the turn (Dreams
    /// of the Sphinx).
    pub hand_bonus: u8,
    /// One additional master phase action from trifles per master phase.
    pub trifle_used: bool,
    /// Out-of-turn master card played since the last master phase.
    pub out_of_turn: Option<OutOfTurn>,
    /// Master phase actions added at the start of the next master phase
    /// (Wash canceling an out-of-turn master).
    pub mpa_next: u8,
    /// Veil the Legions [OBF]: the next actions of this Methuselah's
    /// minions this turn that get +1 stealth.
    pub veil_left: u8,
    /// Discard phase actions left in the current discard phase.
    pub dpa: u8,
    /// A master card was played in the current master phase (Nonu Dis).
    pub master_played: bool,
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
    pub phase_uses: Vec<(crate::cards::AbilitySource, u8)>,
    /// Abilities used this turn ("once each turn", `Limit::PerTurn`);
    /// cleared as each Methuselah's turn begins.
    pub turn_uses: Vec<(crate::cards::AbilitySource, u8)>,
    /// A superior Veil the Legions was played this turn, by anyone ("only
    /// one … at superior each turn"; owner, 2026-09-26: rules-qa 23.6).
    pub veil_superior: bool,
    /// Continuous effects (DESIGN §4.5 scope layers).
    pub layers: Vec<Layer>,
    pub active: SeatId,
    /// Game turn counter, 1-based once the first turn starts.
    pub turn: u32,
    /// The minions in the combat under way, if any (the acting side
    /// first): "while the employer is in combat" (Raptor [ANI]).
    pub combatants: Option<[MinionId; 2]>,
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

    /// An ally's type line, of the version that entered play (`None` for
    /// a vampire).
    pub fn ally_body(&self, m: MinionId) -> Option<crate::library::Body> {
        match self.minions[m.index()].kind {
            MinionKind::Ally { body, .. } => Some(body),
            MinionKind::Vampire(_) => None,
        }
    }

    /// Bleed amount before modifiers (rulebook "Bleed": all vampires bleed
    /// for 1 by default; an ally for its printed bleed).
    pub fn base_bleed(&self, m: MinionId) -> u8 {
        self.ally_body(m).map_or(1, |b| b.bleed)
    }

    /// Strength before modifiers (rulebook "Hand Strike": vampires have a
    /// default strength of 1; an ally has its printed strength).
    pub fn base_strength(&self, m: MinionId) -> u8 {
        self.ally_body(m).map_or(1, |b| b.strength)
    }

    /// Hand indices of the seat's cards — master cards only with
    /// `masters`.
    pub fn hand_picks(&self, seat: SeatId, of: crate::cards::CardFilter) -> Vec<u8> {
        self.seats[seat.index()]
            .hand
            .iter()
            .enumerate()
            .filter(|(_, &c)| of.matches(c))
            .map(|(i, _)| i as u8)
            .collect()
    }

    /// The Methuselah controlling a card in play: a master card stays its
    /// controller's wherever it sits (Fame on another Methuselah's
    /// vampire; The Coven once taken); a minion card on a minion is its
    /// host's controller's (owner, 2026-09-06; spec §1.12.1).
    pub fn card_controller(&self, id: CardId) -> SeatId {
        let c = &self.cards[id.index()];
        if let Some(m) = c.body {
            return self.minions[m.index()].owner;
        }
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

    /// Printed modifiers to minion `m`'s `stat` from the vampires in play
    /// (`cards::vampires`): its own text and other vampires' ("Brujah get
    /// +1 strength in combat with him"), read live — in torpor included
    /// (LSJ 20011022), nothing from a contested vampire.
    pub fn printed_mod(&self, m: MinionId, stat: crate::cards::Stat, ctx: PrintedCtx) -> i32 {
        use crate::cards::PrintedTo;
        let owner = self.minions[m.index()].owner;
        let mut total = 0;
        for (i, v) in self.minions.iter().enumerate() {
            let source = MinionId(i as u32);
            let Some(def) = v
                .crypt_card()
                .and_then(crate::cards::vampire)
                .filter(|_| v.present())
            else {
                continue;
            };
            for p in def.printed.iter().filter(|p| p.stat == stat) {
                let applies = match p.to {
                    PrintedTo::Itself => source == m,
                    PrintedTo::Opponent => ctx.opponent == Some(source),
                    PrintedTo::OthersControlled => source != m && v.owner == owner,
                };
                if applies
                    && p.requires
                        .iter()
                        .all(|&r| self.printed_holds(r, source, m, ctx))
                {
                    total += p.n as i32 * self.printed_count(p.per, source);
                }
            }
        }
        total
    }

    /// A printed-modifier predicate, read on the vampire whose text it is
    /// (`source`), the modified minion (`actor`) and the opposing minion.
    fn printed_holds(
        &self,
        r: crate::cards::Requirement,
        source: MinionId,
        actor: MinionId,
        ctx: PrintedCtx,
    ) -> bool {
        use crate::cards::Requirement as R;
        use crate::decision::ActionKind;
        let src = &self.minions[source.index()];
        let me = &self.minions[actor.index()];
        let opp = ctx.opponent.map(|o| &self.minions[o.index()]);
        match r {
            R::ThisVampireReady => src.ready(),
            R::IsDirected => ctx.directed == Some(true),
            R::IsUndirected => ctx.directed == Some(false),
            R::IsBleed => ctx.kind == Some(ActionKind::Bleed),
            R::IsPolitical => ctx.kind == Some(ActionKind::Political),
            R::OpponentClan(c) => opp.and_then(|o| o.vampire()).is_some_and(|o| o.clan == c),
            // Younger and older compare vampires: an ally has no capacity.
            R::OpponentYounger => {
                matches!((opp.and_then(|o| o.capacity()), me.capacity()), (Some(o), Some(m)) if o < m)
            }
            R::OpponentAllyOrYounger => opp.is_some_and(|o| o.is_ally() || o.younger_than(me)),
            R::OpponentOlder => {
                matches!((opp.and_then(|o| o.capacity()), me.capacity()), (Some(o), Some(m)) if o > m)
            }
            R::OpponentTitled => ctx
                .opponent
                .is_some_and(|o| self.title_of(o) != Title::None),
            R::ActorClan(c) => me.vampire().is_some_and(|v| v.clan == c),
            R::ActorTitleIn(classes) => classes.iter().any(|c| c.matches(self.title_of(actor))),
            R::PoolAtMost(n) => self.seats[src.owner.index()].pool <= n as i32,
            R::ControlsLocation | R::ControlsNoLocation => {
                let any = self.cards.iter().enumerate().any(|(i, c)| {
                    c.present()
                        && self.card_controller(CardId(i as u32)) == src.owner
                        && LIBRARY[c.card as usize]
                            .labels
                            .contains(&crate::library::Label::Location)
                });
                any == (r == R::ControlsLocation)
            }
            R::ControlsVampireNamed(name) => self
                .minions
                .iter()
                .any(|v| v.owner == src.owner && v.present() && v.name() == name),
            _ => unreachable!("{r:?} is not a printed-modifier predicate"),
        }
    }

    fn printed_count(&self, per: crate::cards::Per, source: MinionId) -> i32 {
        use crate::library::{CardType, Label};
        match per {
            crate::cards::Per::One => 1,
            crate::cards::Per::UniqueEquipmentOn => self
                .cards
                .iter()
                .filter(|c| {
                    let l = &LIBRARY[c.card as usize];
                    c.present()
                        && c.host == Some(source)
                        && l.types.contains(&CardType::Equipment)
                        && l.labels.contains(&Label::Unique)
                })
                .count() as i32,
        }
    }

    /// Hand size (rulebook "Drawing Cards"): seven, the until-end-of-turn
    /// bonus (Dreams of the Sphinx), and the hand-size layers of the
    /// uncontested cards in play the seat controls (Elder Library; Visit
    /// from the Capuchin, one per counter; spec §1.13.3: a contested
    /// location gives no hand size).
    /// Discard phase actions printed on the seat's vampires ("if Sreelekha
    /// is ready at the start of your discard phase, you get +1 discard
    /// phase action"), read as the phase begins.
    pub fn printed_discard_actions(&self, s: SeatId) -> i32 {
        self.minions_of(s)
            .map(|m| {
                self.printed_mod(
                    m,
                    crate::cards::Stat::DiscardPhaseActions,
                    PrintedCtx::default(),
                )
            })
            .sum()
    }

    pub fn hand_size(&self, s: SeatId) -> usize {
        // Printed hand size of the seat's vampires ("while André is ready,
        // you get +2 hand size").
        // Read on every request (discard-down): one pass over the seat's
        // vampires rather than `printed_mod` per minion. Hand size is only
        // printed as "you get" on the vampire itself (`PrintedTo::Itself`).
        let printed: i32 = self
            .minions
            .iter()
            .enumerate()
            .filter(|(_, v)| v.owner == s && v.present())
            .filter_map(|(i, v)| {
                v.crypt_card()
                    .and_then(crate::cards::vampire)
                    .map(|d| (MinionId(i as u32), d))
            })
            .flat_map(|(m, d)| {
                d.printed
                    .iter()
                    .filter(|p| p.stat == crate::cards::Stat::HandSize)
                    .map(move |p| (m, p))
            })
            .filter(|(m, p)| {
                p.requires
                    .iter()
                    .all(|&r| self.printed_holds(r, *m, *m, PrintedCtx::default()))
            })
            .map(|(_, p)| p.n as i32)
            .sum();
        let mut size = HAND_SIZE as i32 + self.seats[s.index()].hand_bonus as i32 + printed;
        // "While the employer is in combat, the opposing minion's
        // controller gets -1 hand size" (Raptor [ANI]).
        if let Some(pair) = self.combatants {
            for (i, &m) in pair.iter().enumerate() {
                if self.minions[pair[1 - i].index()].owner == s {
                    size -= self.combat_hand_penalty(m) as i32;
                }
            }
        }
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

    /// Transfers seat `s` gets from cards in play (Information Highway).
    pub fn transfer_bonus(&self, s: SeatId) -> u8 {
        self.layers
            .iter()
            .filter(|l| {
                l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && self.card_controller(l.source) == s
            })
            .map(|l| match l.modifier {
                crate::cards::Modifier::Transfers(n) => n,
                _ => 0,
            })
            .sum()
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
            crate::cards::Requirement::SuperiorVersion => c.mode > 0,
            crate::cards::Requirement::BasicVersion => c.mode == 0,
            _ => match c.bearer() {
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
        if self.no_equipment_or_retainers(m) {
            return false;
        }
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
    /// equipment type; a locquipment is only a location once in play, spec
    /// §1.3.1).
    pub fn equipment_on(&self, m: MinionId) -> Vec<CardId> {
        self.cards
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.present()
                    && c.host == Some(m)
                    && (crate::cards::CardFilter::EquipmentWithout(&[]).matches(c.card)
                        || crate::cards::becomes_weapon(c.card))
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
        self.bearer_mods(m, stat, kind).map(|(_, n)| n).sum()
    }

    /// Can `actor` perform an action directed at `m`? Not without one of
    /// the Disciplines a card on `m` names ("minions without Auspex cannot
    /// perform actions directed at this vampire", Shadow Cloak [OBL]).
    pub fn may_direct_at(&self, actor: MinionId, m: MinionId) -> bool {
        let disc: &[&str] = self.minions[actor.index()]
            .vampire()
            .map_or(&[], |v| v.disciplines);
        self.layers.iter().all(|l| {
            !(l.scope == crate::cards::Scope::InPlay
                && self.layer_active(l)
                && self.cards[l.source.index()].bearer() == Some(m))
                || !matches!(l.modifier, crate::cards::Modifier::DirectedOnlyBy(ds)
                    if !ds.iter().any(|d| crate::cards::has_discipline(disc, d)))
        })
    }

    /// Cards on `m` whose counter burns instead of its unlocking as normal
    /// (Touch of Oblivion).
    pub fn counter_instead_of_unlock(&self, m: MinionId) -> Vec<CardId> {
        self.layers
            .iter()
            .filter(|l| {
                l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && l.modifier == crate::cards::Modifier::CounterInsteadOfUnlock
                    && self.cards[l.source.index()].bearer() == Some(m)
            })
            .map(|l| l.source)
            .collect()
    }

    /// The discount on `rescuer`'s rescue of `target` from its cards in play,
    /// and the blood the rescued vampire gains on success (Saulot's Healing
    /// Touch).
    pub fn rescue_discount(&self, rescuer: MinionId, target: MinionId) -> (u8, u8) {
        let clan = self.minions[target.index()].vampire().map(|v| v.clan);
        self.layers
            .iter()
            .filter(|l| {
                l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && self.cards[l.source.index()].bearer() == Some(rescuer)
            })
            .fold((0, 0), |(d, g), l| match l.modifier {
                crate::cards::Modifier::RescueDiscount { not_clan, n, gain }
                    if clan != Some(not_clan) =>
                {
                    (d + n, g + gain)
                }
                _ => (d, g),
            })
    }

    /// As `bearer_mod`, with no equipment increasing it — reductions still
    /// apply ("no other equipment can increase the bleed amount of this
    /// action", Camera Phone, which grants its own +1 with the action).
    pub fn bearer_mod_without_equipment_increase(
        &self,
        m: MinionId,
        stat: crate::cards::Stat,
        kind: crate::decision::ActionKind,
    ) -> i32 {
        self.bearer_mods(m, stat, kind)
            .map(|(card, n)| {
                let equipment = LIBRARY[card as usize]
                    .types
                    .contains(&crate::library::CardType::Equipment);
                if equipment {
                    n.min(0)
                } else {
                    n
                }
            })
            .sum()
    }

    /// The bearer modifiers of the cards in play on minion `m` for `stat`
    /// during its action of `kind`, with the card each comes from.
    fn bearer_mods(
        &self,
        m: MinionId,
        stat: crate::cards::Stat,
        kind: crate::decision::ActionKind,
    ) -> impl Iterator<Item = (u16, i32)> + '_ {
        self.layers
            .iter()
            .filter(move |l| {
                l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && self.cards[l.source.index()].bearer() == Some(m)
            })
            .filter_map(move |l| match l.modifier {
                crate::cards::Modifier::Bearer {
                    stat: s,
                    n,
                    during,
                    who,
                } if s == stat
                    && (during.is_empty() || during.contains(&kind))
                    && who.is(self, m) =>
                {
                    Some((self.cards[l.source.index()].card, n as i32))
                }
                _ => None,
            })
    }

    /// Damage the retainers on minion `m` inflict on its opponent each
    /// round during normal strike resolution (`Modifier::RetainerDamage`).
    pub fn retainer_damage(&self, m: MinionId) -> u8 {
        self.layers
            .iter()
            .filter(|l| {
                l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && self.cards[l.source.index()].host == Some(m)
            })
            .map(|l| match l.modifier {
                crate::cards::Modifier::RetainerDamage(n) => n,
                _ => 0,
            })
            .sum()
    }

    /// Whether `viewer` sees `seat`'s hand: their own, or an open hand —
    /// `seat`'s minion in combat with the employer of an Owl Companion.
    /// The legal view reads it.
    pub fn hand_visible_to(&self, seat: SeatId, viewer: SeatId) -> bool {
        viewer == seat
            || self.combatants.is_some_and(|pair| {
                (0..2).any(|i| {
                    self.minions[pair[1 - i].index()].owner == seat
                        && self.layers.iter().any(|l| {
                            l.modifier == crate::cards::Modifier::OpponentOpenHandInCombat
                                && l.scope == crate::cards::Scope::InPlay
                                && self.layer_active(l)
                                && self.cards[l.source.index()].host == Some(pair[i])
                        })
                })
            })
    }

    /// Hand size the retainers on combatant `m` take from its opponent's
    /// controller (`Modifier::OpponentHandSizeInCombat`).
    pub fn combat_hand_penalty(&self, m: MinionId) -> u8 {
        self.layers
            .iter()
            .filter(|l| {
                l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && self.cards[l.source.index()].host == Some(m)
            })
            .map(|l| match l.modifier {
                crate::cards::Modifier::OpponentHandSizeInCombat(n) => n,
                _ => 0,
            })
            .sum()
    }

    /// `m` cannot have or use equipment or retainers (Bone Shambler's own
    /// text): it takes none, by any means.
    pub fn no_equipment_or_retainers(&self, m: MinionId) -> bool {
        self.layers.iter().any(|l| {
            l.modifier == crate::cards::Modifier::NoEquipmentOrRetainers
                && l.scope == crate::cards::Scope::InPlay
                && self.layer_active(l)
                && self.cards[l.source.index()].bearer() == Some(m)
        })
    }

    /// `m` cannot be the target of a directed action requiring one of the
    /// Disciplines of `tags` (a mode's tags, any level).
    pub fn immune_to_directed(&self, m: MinionId, tags: &[&str]) -> bool {
        self.layers.iter().any(|l| {
            l.scope == crate::cards::Scope::InPlay
                && self.layer_active(l)
                && self.cards[l.source.index()].bearer() == Some(m)
                && matches!(l.modifier, crate::cards::Modifier::ImmuneToDirectedActions(ds)
                    if tags.iter().any(|t| ds.contains(&t.to_ascii_lowercase().as_str())))
        })
    }

    /// What `m` — an ally — may play "as a vampire", if anything.
    pub fn plays_as_vampire(&self, m: MinionId) -> Option<crate::cards::AsVampire> {
        self.layers.iter().find_map(|l| match l.modifier {
            crate::cards::Modifier::PlaysAsVampire(g)
                if l.scope == crate::cards::Scope::InPlay
                    && self.layer_active(l)
                    && self.cards[l.source.index()].bearer() == Some(m) =>
            {
                Some(g)
            }
            _ => None,
        })
    }

    /// Allies cannot block `m` (Libertas on it).
    pub fn allies_cannot_block(&self, m: MinionId) -> bool {
        self.layers.iter().any(|l| {
            l.modifier == crate::cards::Modifier::AlliesCannotBlockBearer
                && l.scope == crate::cards::Scope::InPlay
                && self.layer_active(l)
                && self.cards[l.source.index()].host == Some(m)
        })
    }

    /// The strikes of `m` cannot be dodged (Aggressive Corpse's own text).
    pub fn undodgeable(&self, m: MinionId) -> bool {
        self.layers.iter().any(|l| {
            l.modifier == crate::cards::Modifier::UndodgeableStrikes
                && l.scope == crate::cards::Scope::InPlay
                && self.layer_active(l)
                && self.cards[l.source.index()].bearer() == Some(m)
        })
    }

    /// `m` cannot block vampires (Vagabond Mystic's own text).
    pub fn cannot_block_vampires(&self, m: MinionId) -> bool {
        self.layers.iter().any(|l| {
            l.modifier == crate::cards::Modifier::CannotBlockVampires
                && l.scope == crate::cards::Scope::InPlay
                && self.layer_active(l)
                && self.cards[l.source.index()].bearer() == Some(m)
        })
    }

    /// Minions in combat with `m` cannot strike: combat ends (a retainer on
    /// it: Dog Pack).
    pub fn combat_ends_barred(&self, m: MinionId) -> bool {
        self.layers.iter().any(|l| {
            l.modifier == crate::cards::Modifier::NoCombatEndsAgainstHost
                && l.scope == crate::cards::Scope::InPlay
                && self.layer_active(l)
                && self.cards[l.source.index()].host == Some(m)
        })
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

    /// Other seats' in-play copies of `m`'s vampire or unique ally (contest
    /// rivals: uniqueness runs on the name, across minion types, advanced
    /// rules 5.1.3).
    pub fn card_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let me = &self.minions[m.index()];
        if me.kind.ally_card().is_some_and(|c| {
            !LIBRARY[c as usize]
                .labels
                .contains(&crate::library::Label::Unique)
        }) {
            return Vec::new();
        }
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

    /// The title card on vampire `m`, with the title it represents: face
    /// down under a title contest included, not under a card contest (a
    /// unique title card contests as a card first, its title out of play
    /// with it: rules-qa 19.1).
    pub fn title_card_on(&self, m: MinionId) -> Option<(CardId, Title)> {
        let title_contested = self.minions[m.index()].title_contested;
        self.cards.iter().enumerate().find_map(|(i, c)| {
            (c.in_play && c.host == Some(m) && (!c.contested || title_contested))
                .then(|| crate::cards::card_title(c.card))
                .flatten()
                .map(|t| (CardId(i as u32), t))
        })
    }

    /// The title vampire `m` claims, contested or not: its title card's,
    /// a title gained without a card, or its printed one unless lost.
    pub fn claimed_title(&self, m: MinionId) -> Title {
        let v = &self.minions[m.index()];
        let Some(printed) = v.vampire() else {
            return Title::None;
        };
        if let Some((_, t)) = self.title_card_on(m) {
            return t;
        }
        match v.granted_title {
            Some(t) => t,
            None if !v.title_lost => printed.title,
            None => Title::None,
        }
    }

    /// Effective title (read live): the claimed title, none while it is
    /// contested, while the vampire is out of play, or while its sect is
    /// inappropriate for it (rulebook "Vampiric Sects": the benefit returns
    /// with the sect). A justicar's clan is checked as the title is given:
    /// no pool card changes a clan, and printed justicars keep their
    /// clan's old name (Kasim Bayar, Justicar "Assamite").
    pub fn title_of(&self, m: MinionId) -> Title {
        let v = &self.minions[m.index()];
        if v.title_contested || !v.present() {
            return Title::None;
        }
        let t = self.claimed_title(m);
        let Some(c) = v.vampire() else {
            return Title::None;
        };
        let fits = match t {
            Title::None => true,
            Title::Primogen | Title::Prince(_) | Title::Justicar(_) => c.sect == Sect::Camarilla,
            Title::Baron(_) => c.sect == Sect::Anarch,
            Title::Independent { .. } => c.sect == Sect::Independent,
        };
        if fits {
            t
        } else {
            Title::None
        }
    }

    /// Votes vampire `m` can cast right now from its title (a ready,
    /// uncontested holder; rulebook "Gaining Votes").
    pub fn title_votes_of(&self, m: MinionId) -> u8 {
        if self.minions[m.index()].ready() {
            title_votes(self.title_of(m))
        } else {
            0
        }
    }

    /// Other present vampires (any seat) claiming a title that contests
    /// the one `m` claims.
    pub fn title_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let Some(key) = title_key(self.claimed_title(m)) else {
            return Vec::new();
        };
        self.minions
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != m.index()
                    && o.present()
                    && !self.seats[o.owner.index()].ousted
                    && title_key(self.claimed_title(MinionId(*i as u32))) == Some(key)
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
        mix(self
            .combatants
            .map_or(0, |[a, b]| 1 + ((a.0 as u64) << 20 | b.0 as u64)));
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
            mix(s.master_played as u64);
            mix(s.held.unlock as u64);
            mix(s.held.discard as u64);
            mix(s.held.action as u64);
            mix(s.forbidden_political.len() as u64);
            for a in &s.forbidden_political {
                mix(match a {
                    ActionId::Card(c) => *c as u64,
                    ActionId::InPlay(c) => 1 << 32 | c.0 as u64,
                    ActionId::Printed(m, i) => 2 << 40 | (m.0 as u64) << 8 | *i as u64,
                    ActionId::Rulebook(k, t) => {
                        3 << 40 | (*k as u64) << 32 | t.map_or(u32::MAX, |t| t.0) as u64
                    }
                });
            }
            mix(match s.out_of_turn {
                None => 0,
                Some(OutOfTurn { trifle }) => 1 + trifle as u64,
            });
            mix(s.mpa_next as u64);
            mix(s.veil_left as u64);
            for zone in [
                &s.crypt,
                &s.uncontrolled_cards(),
                &s.library,
                &s.hand,
                &s.ash,
                &s.ash_crypt,
                &s.removed_crypt,
                &s.removed,
            ] {
                mix(zone.len() as u64);
                for &c in zone.iter() {
                    mix(c as u64);
                }
            }
            for u in &s.uncontrolled {
                mix(u.counters as u64);
                mix(u.banished.as_ref().map_or(0, |b| b.minion.0 as u64 + 1));
                for c in u.banished.iter().flat_map(|b| &b.cards) {
                    mix(c.0 as u64);
                }
            }
        }
        mix(self.minions.len() as u64);
        for m in &self.minions {
            mix(m.owner.0 as u64);
            mix(match m.kind {
                MinionKind::Vampire(c) => c as u64,
                MinionKind::Ally { card, id, body } => {
                    1 << 40
                        | (body.life as u64) << 48
                        | (body.strength as u64) << 44
                        | (id.0 as u64) << 16
                        | card as u64
                }
            });
            mix(m.recruited as u64);
            mix(m.recruits_this_turn as u64);
            for &c in &m.played_since_unlock {
                mix(c as u64);
            }
            mix(m.unlock_at_discard as u64);
            mix(m.no_life_gain as u64);
            mix(m.blood as u64);
            mix(m.locked as u64);
            mix(m.in_torpor as u64);
            mix(m.in_play as u64);
            mix(m.contested as u64);
            mix(m.title_contested as u64);
            mix(m.title_lost as u64);
            mix(match m.granted_title {
                None => 0,
                Some(t) => format!("{t:?}")
                    .bytes()
                    .fold(1u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64)),
            });
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
                    ActionId::Printed(m, i) => 1 << 52 | (m.0 as u64) << 8 | i as u64,
                    ActionId::Rulebook(k, t) => {
                        1 << 53 | (k as u64) << 32 | t.map_or(u32::MAX, |t| t.0) as u64
                    }
                });
            }
        }
        mix(self.phase_uses.len() as u64);
        mix(self.turn_uses.len() as u64);
        mix(self.veil_superior as u64);
        for (c, a) in self.phase_uses.iter().chain(&self.turn_uses) {
            let source = match c {
                crate::cards::AbilitySource::Card(id) => id.0 as u64,
                crate::cards::AbilitySource::Vampire(m) => (m.0 as u64) << 32,
            };
            mix((source + 1) << 8 ^ *a as u64);
        }
        mix(self.cards.len() as u64);
        for c in &self.cards {
            mix(c.card as u64);
            mix(c.owner.0 as u64);
            mix(c.controller.0 as u64);
            mix(c.host.map(|m| m.0 as u64 + 1).unwrap_or(0));
            mix(c.body.map(|m| m.0 as u64 + 1).unwrap_or(0));
            mix(c.mode as u64);
            mix(c.by.map_or(0, |m| m.0 as u64 + 1));
            mix(c.in_play as u64);
            mix(c.contested as u64);
            mix(c.locked as u64);
            mix(c.counters as u64);
            mix(c.effect_only as u64);
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
