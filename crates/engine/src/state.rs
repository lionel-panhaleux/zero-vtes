use crate::cards::{self, AbilitySource, Holding, Layer, Modifier, Requirement, Scope, Stat};
use crate::crypt::{CryptCard, Sect, Title, CRYPT};
use crate::decision::ActionKind;
use crate::ids::{MinionId, SeatId};
use crate::library::{Body, CardType, Label, LIBRARY};

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

/// The cards of `cards` that `keep` keeps, one of each (copies are alike),
/// in card order.
pub fn distinct(cards: &[u16], keep: impl Fn(u16) -> bool) -> Vec<u16> {
    let mut out: Vec<u16> = cards.iter().copied().filter(|&c| keep(c)).collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Whether `i` is the first copy of its card among `cards` (copies in
/// hand are alike: one choice for them all, owner 2026-10-04).
pub fn first_copy(cards: &[u16], i: usize) -> bool {
    !cards[..i].contains(&cards[i])
}

/// Contest identity of a title (rulebook "Contested Titles" / "Vampiric
/// Sects"): prince, baron and archbishop of one city contest each other;
/// each clan's justicar and Inner Circle member are unique (one of each:
/// they do not contest each other); primogen is not unique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TitleKey {
    City(&'static str),
    Justicar(&'static str),
    InnerCircle(&'static str),
    /// A unique title a card names (Saulot's Guiding Wisdom).
    Named(&'static str),
}

pub fn title_key(t: Title) -> Option<TitleKey> {
    match t {
        Title::Prince(c) | Title::Baron(c) | Title::Archbishop(c) => Some(TitleKey::City(c)),
        Title::Justicar(k) => Some(TitleKey::Justicar(k)),
        Title::InnerCircle(k) => Some(TitleKey::InnerCircle(k)),
        // The regent is unique; bishops, cardinals, prisci are not.
        Title::Regent => Some(TitleKey::Named("Regent")),
        Title::Bishop | Title::Cardinal | Title::Priscus => None,
        Title::Independent { name, .. } => Some(TitleKey::Named(name)),
        Title::None | Title::Primogen => None,
    }
}

pub fn title_votes(t: Title) -> u8 {
    match t {
        Title::None => 0,
        Title::Primogen => 1,
        Title::Prince(_) | Title::Baron(_) | Title::Archbishop(_) => 2,
        Title::Justicar(_) => 3,
        Title::InnerCircle(_) => 4,
        Title::Bishop => 1,
        Title::Cardinal => 3,
        Title::Regent => 4,
        // A priscus votes in the prisci block (its ballot), not with votes.
        Title::Priscus => 0,
        Title::Independent { votes, .. } => votes,
    }
}

/// What a vampire's printed modifier reads besides the game state
/// (`GameState::printed_mod`): the action performed, if any, and the
/// minion opposing the modified one — the acting minion for a block
/// attempt, the opposing combatant in combat.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrintedCtx {
    pub kind: Option<ActionKind>,
    pub directed: Option<bool>,
    pub opponent: Option<MinionId>,
    /// The card the modified minion plays or puts in play, and the mode
    /// it is played with (a printed cost reduction).
    pub card: Option<(u16, u8)>,
    /// The Methuselah the action is directed at ("+1 bleed against a
    /// Methuselah controlling …", Osvaldo Kühnemann).
    pub target: Option<SeatId>,
}

/// What a minion's printed text takes off a card's cost
/// (`GameState::printed_cost_off`): pool only, blood only, and "blood or
/// pool" — off whichever the card costs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CostOff {
    pub pool: u8,
    pub blood: u8,
    pub either: u8,
}

impl CostOff {
    /// Take the reductions off a `(pool, blood)` cost.
    pub fn apply(self, pool: &mut i32, blood: &mut u8) {
        *pool = (*pool - self.pool as i32).max(0);
        *blood = blood.saturating_sub(self.blood);
        crate::plays::reduce(pool, blood, self.either);
    }
}

/// What a minion is: a vampire (its crypt card) or an ally — a library card
/// in play (`id`, whose `CardInPlay::body` points back here) that is a
/// minion in its own right (rulebook "Ally cards"), with the type line of
/// the version that entered play (`body`: its starting life, strength and
/// bleed; advanced rules 5.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MinionKind {
    Vampire(u16),
    Ally {
        card: u16,
        id: CardId,
        body: Body,
    },
    /// A library card in play as a vampire (Childe of the Revolution,
    /// Waters of Duat): only what the effect describes — a non-unique
    /// 1-capacity vampire of the clan and sect given, with no Discipline
    /// and no title; the card's own name, type and text are inert
    /// (advanced rules 1.4, 5.1.3).
    Created {
        card: u16,
        id: CardId,
        clan: &'static str,
        sect: Sect,
    },
    /// The top card of a library put in play as "a nameless, textless
    /// ghoul ally" (Absimiliard's Army, `source`): an ally with the stats
    /// given, no card in play, no name, label or text — no contest (LSJ
    /// 20040623); its card removed from the game as it leaves.
    Token {
        card: u16,
        body: Body,
        source: CardId,
    },
}

impl MinionKind {
    /// The ally's LIBRARY index; `None` for a vampire.
    pub fn ally_card(self) -> Option<u16> {
        match self {
            MinionKind::Ally { card, .. } => Some(card),
            // Textless: no card to read.
            MinionKind::Vampire(_) | MinionKind::Created { .. } | MinionKind::Token { .. } => None,
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// Corruption counters on this minion, by the Methuselah whose they are
    /// ("your corruption counters"): kept through torpor, control changes
    /// and banishment (a banished vampire keeps everything; owner,
    /// 2026-09-26: rules-qa 25.1), gone when it is burned or removed with
    /// its ousted Methuselah.
    pub corruption: Vec<(SeatId, u8)>,
    /// Committed diablerie: ignores Blood Weakens and The Slow Withering
    /// until a Gehenna event is played.
    pub diablerie_shield: bool,
    /// The game turn of its last diablerie (Carlton Van Wyk).
    pub diablerie_turn: Option<u32>,
    /// Took a hunt action this turn (Thirst: "who did not hunt during that
    /// minion phase").
    pub hunted: bool,
    /// Held on temporary control: the Methuselah it returns to at the end
    /// of its controller's turn (Puppet Master [DOM]).
    pub temp_from: Option<SeatId>,
    /// The vampire's owner, when its control changed for good
    /// (Graverobbing): its ash heap is the owner's.
    pub origin: Option<SeatId>,
    /// A sect a card made it (Twilight Camp: "make that vampire Anarch").
    pub sect_override: Option<Sect>,
    /// The Sabbat path it follows when not the one printed on it (Black
    /// Chantry, 2025-06-12: one path at a time); none in the pool sets it.
    pub path_override: Option<&'static str>,
    /// "Must hunt this turn" (Childe of the Revolution): as a vampire with
    /// no blood, until it hunts or the turn ends.
    pub must_hunt: bool,
    /// Hostage counters (Carver's Meat Packing and Storage): while it has
    /// any, the vampire cannot be moved to the ready region or be
    /// diablerized.
    pub hostage: u8,
    /// Nightmare counters (Week of Nightmares).
    pub nightmare: u8,
}

/// Identity of an action for the no-repeat-action rule (spec §3.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionId {
    /// An action card played from hand: same printed name.
    Card(u16),
    /// An action granted by a card in play: the same copy's same ability
    /// (two actions a card grants are not cross-restrictive, Open War: LSJ
    /// 20100909).
    InPlay(CardId, u8),
    /// A vampire's printed action, by vampire and ability index: never the
    /// same as a card's or a rulebook action (spec §3.8.1). The vampire is
    /// kept for records held by the seat (a forbidden political action).
    Printed(MinionId, u8),
    /// A rulebook action and its target minion, recorded only where a card
    /// forbids repeating it ("this minion cannot perform the same action
    /// again this turn", Change of Target): rulebook actions are otherwise
    /// repeatable (LSJ 20060522).
    Rulebook(ActionKind, Option<MinionId>),
}

impl ActionId {
    pub fn same(self, other: ActionId) -> bool {
        match (self, other) {
            (ActionId::Card(a), ActionId::Card(b)) => {
                LIBRARY[a as usize].name == LIBRARY[b as usize].name
            }
            (ActionId::InPlay(a, i), ActionId::InPlay(b, j)) => a == b && i == j,
            (ActionId::Printed(m, a), ActionId::Printed(n, b)) => m == n && a == b,
            (ActionId::Rulebook(k, a), ActionId::Rulebook(l, b)) => k == l && a == b,
            _ => false,
        }
    }
}

impl Minion {
    /// `seat`'s corruption counters on this minion.
    pub fn corruption_of(&self, seat: SeatId) -> u8 {
        self.corruption
            .iter()
            .find(|(s, _)| *s == seat)
            .map_or(0, |&(_, n)| n)
    }

    pub fn add_corruption(&mut self, seat: SeatId, n: u8) {
        match self.corruption.iter_mut().find(|(s, _)| *s == seat) {
            Some((_, k)) => *k += n,
            None => self.corruption.push((seat, n)),
        }
    }

    /// Burn up to n of `seat`'s corruption counters.
    pub fn burn_corruption(&mut self, seat: SeatId, n: u8) {
        if let Some((_, k)) = self.corruption.iter_mut().find(|(s, _)| *s == seat) {
            *k = k.saturating_sub(n);
        }
        self.corruption.retain(|&(_, k)| k > 0);
    }

    /// A minion entering play: ready, unlocked and uncontested, with no
    /// history.
    pub fn new(owner: SeatId, kind: MinionKind, blood: u8) -> Minion {
        Minion {
            owner,
            kind,
            blood,
            locked: false,
            in_torpor: false,
            in_play: true,
            contested: false,
            title_contested: false,
            title_lost: false,
            granted_title: None,
            bled_this_turn: false,
            political_this_turn: false,
            expelled: false,
            hunting_ground_fed: false,
            skip_unlock: false,
            acted: Vec::new(),
            recruited: false,
            recruits_this_turn: 0,
            played_since_unlock: Vec::new(),
            unlock_at_discard: false,
            corruption: Vec::new(),
            diablerie_shield: false,
            diablerie_turn: None,
            hunted: false,
            temp_from: None,
            origin: None,
            sect_override: None,
            path_override: None,
            no_life_gain: false,
            must_hunt: false,
            hostage: 0,
            nightmare: 0,
        }
    }

    /// The vampire's crypt card; `None` for an ally (no clan, sect,
    /// Disciplines, capacity or title).
    pub fn vampire(&self) -> Option<&'static CryptCard> {
        match self.kind {
            MinionKind::Vampire(c) => Some(&CRYPT[c as usize]),
            MinionKind::Ally { .. } | MinionKind::Created { .. } | MinionKind::Token { .. } => None,
        }
    }

    /// The vampire's CRYPT index (printed text lookup); `None` for an ally
    /// or a created vampire.
    pub fn crypt_card(&self) -> Option<u16> {
        match self.kind {
            MinionKind::Vampire(c) => Some(c),
            MinionKind::Ally { .. } | MinionKind::Created { .. } | MinionKind::Token { .. } => None,
        }
    }

    pub fn is_ally(&self) -> bool {
        matches!(
            self.kind,
            MinionKind::Ally { .. } | MinionKind::Token { .. }
        )
    }

    /// A vampire: from the crypt or created from a library card.
    pub fn is_vampire(&self) -> bool {
        !self.is_ally()
    }

    /// The ally's card in play; `None` for a vampire.
    pub fn ally_id(&self) -> Option<CardId> {
        match self.kind {
            MinionKind::Ally { id, .. } => Some(id),
            MinionKind::Vampire(_) | MinionKind::Created { .. } | MinionKind::Token { .. } => None,
        }
    }

    /// Printed name: the contest identity of a vampire or unique ally. A
    /// created vampire has none (non-unique, advanced rules 5.1.3): its
    /// card's name, for display only.
    pub fn name(&self) -> &'static str {
        match self.kind {
            MinionKind::Vampire(c) => CRYPT[c as usize].name,
            MinionKind::Ally { card, .. } | MinionKind::Created { card, .. } => {
                LIBRARY[card as usize].name
            }
            // "Nameless": the name on its card is ignored (LSJ 20040623).
            MinionKind::Token { .. } => "",
        }
    }

    /// A vampire's clan; `None` for an ally.
    pub fn clan(&self) -> Option<&'static str> {
        match self.kind {
            MinionKind::Vampire(c) => Some(CRYPT[c as usize].clan),
            MinionKind::Created { clan, .. } => Some(clan),
            MinionKind::Ally { .. } | MinionKind::Token { .. } => None,
        }
    }

    /// The path the vampire follows, its benefits aside (`GameState::follows`
    /// adds the sect): the one printed on it unless another replaced it.
    pub fn path(&self) -> Option<&'static str> {
        match self.kind {
            MinionKind::Vampire(c) => self.path_override.or(CRYPT[c as usize].path),
            _ => None,
        }
    }

    /// A vampire's own sect, before any global effect (`GameState::sect_of`
    /// reads the Falls); `None` for an ally.
    pub fn underlying_sect(&self) -> Option<Sect> {
        if self.sect_override.is_some() && !self.is_ally() {
            return self.sect_override;
        }
        match self.kind {
            MinionKind::Vampire(c) => Some(CRYPT[c as usize].sect),
            MinionKind::Created { sect, .. } => Some(sect),
            MinionKind::Ally { .. } | MinionKind::Token { .. } => None,
        }
    }

    /// A vampire's printed Disciplines (lower case inferior, upper case
    /// superior); none for an ally or a created vampire.
    pub fn printed_disciplines(&self) -> &'static [&'static str] {
        self.vampire().map_or(&[], |v| v.disciplines)
    }

    /// A vampire's printed title; none for a created vampire.
    pub fn printed_title(&self) -> Title {
        self.vampire().map_or(Title::None, |v| v.title)
    }

    /// A vampire's capacity before cards in play (`GameState::capacity_of`
    /// reads it live); `None` for an ally, which has none (its life is not
    /// capped, advanced rules 5.5.2).
    pub fn printed_capacity(&self) -> Option<u8> {
        match self.kind {
            MinionKind::Vampire(c) => Some(CRYPT[c as usize].capacity),
            MinionKind::Created { .. } => Some(1),
            MinionKind::Ally { .. } | MinionKind::Token { .. } => None,
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
    /// The Methuselahs this card remembers, set as it enters play: those
    /// the action it was played in was directed at ("the same Methuselah",
    /// Spying Mission [OBF]; "the same Methuselah or same set of
    /// Methuselahs", Shadow Cast [OBL]). They stay set when one is ousted:
    /// an ousted Methuselah is never directed at again.
    pub against: crate::ids::SeatSet,
    /// The game turn it entered play ("during your next minion phase",
    /// Puppet Master [DOM]).
    pub entered: u32,
    /// The vampires it chose, kept through their leaving play and coming
    /// back (advanced rules 6.4.3): The Rack's, Priority Contract's
    /// Assamite, Toreador Grand Ball's locked Toreador then the other.
    pub chosen: Vec<MinionId>,
}

impl CardInPlay {
    /// A library card entering play under `owner`'s control, on `host` if
    /// any, in the version of `mode`.
    pub fn new(card: u16, owner: SeatId, host: Option<MinionId>, mode: u8) -> Self {
        CardInPlay {
            card,
            owner,
            controller: owner,
            host,
            body: None,
            mode,
            by: None,
            in_play: true,
            contested: false,
            locked: false,
            counters: 0,
            stored: Vec::new(),
            effect_only: false,
            against: Default::default(),
            entered: 0,
            chosen: Vec::new(),
        }
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CardId(pub u32);

impl CardId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// `UncontrolledSlot::card` of a created vampire moved there: it has no
/// crypt card (its identity is its arena entry, `banished`).
pub const CREATED: u16 = u16::MAX;

/// A card whose identity the viewer does not know (`GameState::legal_view`):
/// neither a LIBRARY nor a CRYPT index, nor `CREATED`.
pub const HIDDEN: u16 = u16::MAX - 1;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Banished {
    pub minion: MinionId,
    pub cards: Vec<CardId>,
}

/// An out-of-turn master card played since the seat's last master phase
/// (rulebook "Out-of-turn"; rules-qa 3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OutOfTurn {
    pub trifle: bool,
}

/// Cards played but not replaced yet, by the moment their replacement is
/// drawn (spec §1.9.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Held {
    /// "Do not replace until your next unlock phase" (Old Friends).
    pub unlock: u8,
    /// "Do not replace until your next discard phase" (Mirror Walk).
    pub discard: u8,
    /// Replaced at the end of the current action (Consign to Oblivion
    /// [OBL]).
    pub action: u8,
    /// Replaced at the end of the action or as the hand empties (Blood
    /// Weakens).
    pub weakens: u8,
    /// "Until a Methuselah is ousted" (Absimiliard's Army).
    pub ousted: u8,
    /// "Until a vampire commits diablerie" (Blood Weakens).
    pub diablerie: u8,
    /// "Until a titled vampire goes to torpor" (The New Inquisition).
    pub titled_torpor: u8,
    /// "Until a vampire successfully hunts" (Restricted Vitae).
    pub hunt: u8,
    /// "Until a vampire moves from torpor to the ready region" (Torpid
    /// Blood).
    pub leaves_torpor: u8,
    /// "Until a vampire becomes a liaison" (Rise of the Nephtali).
    pub liaison: u8,
    /// Discarded to Nightmares upon Nightmares: replaced as the
    /// Methuselah's master phase begins.
    pub master: u8,
}

impl Held {
    pub fn total(self) -> usize {
        [
            self.unlock,
            self.discard,
            self.action,
            self.weakens,
            self.ousted,
            self.diablerie,
            self.titled_torpor,
            self.hunt,
            self.leaves_torpor,
            self.liaison,
            self.master,
        ]
        .iter()
        .map(|&n| n as usize)
        .sum()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
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
    /// Hand size above 7 until their next discard phase begins (Fotini
    /// Katsikaris; rules-qa 53.4).
    pub hand_bonus_discard: u8,
    /// One additional master phase action from trifles per master phase.
    pub trifle_used: bool,
    /// Día de los Muertos this turn: the first referendum a Sabbat vampire
    /// of theirs calls passes automatically.
    pub auto_pass_pending: bool,
    /// A Sabbat vampire of theirs called a referendum this turn.
    pub sabbat_referendum_called: bool,
    /// A burn-option card discarded this unlock phase (rulebook, advanced:
    /// one each unlock phase).
    pub burn_option_used: bool,
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
    /// Of `dpa`, those granted by Powerbase: Los Angeles and not used yet:
    /// one used to discard a card requiring an Anarch or making a vampire
    /// Anarch lets its user unlock a ready Anarch.
    pub granted_dpa: u8,
    /// A master card was played in the current master phase (Nonu Dis).
    pub master_played: bool,
    pub turns_taken: u32,
    /// About to be ousted, waiting on a Life Boon (`Engine::ousting`).
    pub oust_pending: bool,
    /// The game turn of this Methuselah's current (or last) turn, and of
    /// the one before it: "since your last turn" (Carlton Van Wyk).
    pub this_turn: u32,
    pub previous_turn: u32,
    /// Cards played but not replaced yet: they count against the hand
    /// size until then (rulebook "Drawing Cards"; spec §6.8.1).
    pub held: Held,
    /// "Do not replace as long as this card is in play" (Fall of the
    /// Camarilla, The Uncoiling): replaced as the card leaves play (even
    /// burned, LSJ 20080805).
    pub held_in_play: Vec<CardId>,
    /// An event card was put in play this discard phase (rulebook: no more
    /// than one per phase).
    pub event_played: bool,
    /// Burned a vampire of capacity 6 or more to Recalled to the Founder:
    /// ignores it until the end of the game.
    pub recalled_ignored: bool,
    /// The Uncoiling's burn, due in this discard phase: mandatory, ordered
    /// among the phase's other effects, resolved before the impulse passes
    /// (rules-qa 40.5).
    pub uncoiling_due: Option<CardId>,
    /// Political actions the seat's minions cannot perform again this
    /// turn (Delaying Tactics): a card by name, a granted action by copy
    /// (LSJ 20020201).
    pub forbidden_political: Vec<ActionId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GameState {
    pub seats: Vec<Seat>,
    /// Arena: minions are never removed, only marked out of play.
    pub minions: Vec<Minion>,
    /// Arena of library cards in play.
    pub cards: Vec<CardInPlay>,
    /// Abilities (card in play, ability index) used this phase ("during
    /// X": once per phase per copy, rules-qa 6.6); cleared at each phase.
    pub phase_uses: Vec<(AbilitySource, u8)>,
    /// Abilities used this turn ("once each turn", `Limit::PerTurn`);
    /// cleared as each Methuselah's turn begins.
    pub turn_uses: Vec<(AbilitySource, u8)>,
    /// A superior Veil the Legions was played this turn, by anyone ("only
    /// one … at superior each turn"; owner, 2026-09-26: rules-qa 23.6).
    pub veil_superior: bool,
    /// Actions announced in the current minion phase (Channel 10: "not
    /// usable during the first action in a minion phase"); one canceled as
    /// its card was played never was (LSJ 20100505).
    pub phase_actions: u8,
    /// A non-mandatory action was performed this turn (Khabar: Glory).
    pub optional_action: bool,
    /// Cards played this game that "can be played only once in a game"
    /// (Week of Nightmares), by LIBRARY index.
    pub once_played: Vec<u16>,
    /// Cards played this turn that "can be played only once each turn"
    /// (Ashur Tablets), by LIBRARY index; cleared as each turn begins.
    pub turn_played: Vec<u16>,
    /// Cards each minion played this turn under `Limit::PerTurn` ("a
    /// vampire can play only one Forced March each turn"), by LIBRARY
    /// index; cleared as each turn begins.
    pub turn_plays: Vec<(MinionId, u16)>,
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
    pub fn ally_body(&self, m: MinionId) -> Option<Body> {
        match self.minions[m.index()].kind {
            MinionKind::Ally { body, .. } | MinionKind::Token { body, .. } => Some(body),
            MinionKind::Vampire(_) | MinionKind::Created { .. } => None,
        }
    }

    /// Discipline master cards on `m` (Oblivion, "+1 level of Oblivion and
    /// +1 capacity"): the Discipline each gives. Out of play with an
    /// uncontrolled vampire, they give nothing (advanced rules 5.1.1).
    fn discipline_grants(&self, m: MinionId) -> impl Iterator<Item = &'static str> + '_ {
        self.layers.iter().filter_map(move |l| match l.modifier {
            Modifier::DisciplineLevel(d)
                if self.layer_active(l) && self.cards[l.source.index()].host == Some(m) =>
            {
                Some(d)
            }
            _ => None,
        })
    }

    /// A vampire's capacity, read live: printed, +1 for each Discipline
    /// master card on it, never below one (advanced rules 5.1.1); `None`
    /// for an ally.
    pub fn capacity_of(&self, m: MinionId) -> Option<u8> {
        let printed = self.minions[m.index()].printed_capacity()?;
        let cap = (printed + self.discipline_grants(m).count() as u8).max(1);
        // Wormwood: "considered vampires with capacity X (minimum of 1)",
        // the capacity as changed by other effects ceiled (LSJ 20051205).
        Some(match self.capacity_ceiling() {
            Some(x) => cap.min(x),
            None => cap,
        })
    }

    /// The capacity every vampire is ceiled to (Wormwood's counters, at
    /// least 1), if any.
    pub fn capacity_ceiling(&self) -> Option<u8> {
        self.in_play_layers()
            .filter(|l| l.modifier == Modifier::CapacityCapByCounters)
            .map(|l| self.cards[l.source.index()].counters.max(1))
            .min()
    }

    /// Torpid Blood's +1: on an action by a vampire in torpor (leave
    /// torpor), and on rescuing an older vampire (`target` older than
    /// `rescuer`).
    pub fn torpid_surcharge(&self, rescuer: Option<MinionId>, target: MinionId) -> u8 {
        if self.global(Modifier::TorpidBlood).is_none() {
            return 0;
        }
        match rescuer {
            None => 1,
            Some(r) => u8::from(self.younger_than(r, target)),
        }
    }

    /// Is a card in play giving this modifier (an event's global effect)?
    pub fn global(&self, modifier: Modifier) -> Option<CardId> {
        self.in_play_layers()
            .find(|l| l.modifier == modifier)
            .map(|l| l.source)
    }

    /// The card in play burning allies in combat with an acting vampire
    /// (FBI Special Affairs Division), with its damage and burn threshold.
    pub fn ally_burned_in_combat(&self) -> Option<(CardId, u8, u8)> {
        self.in_play_layers().find_map(|l| match l.modifier {
            Modifier::AllyBurnedInCombat { damage, burn_at } => Some((l.source, damage, burn_at)),
            _ => None,
        })
    }

    /// `m`'s level in Discipline `d` (lower case): 0 none, 1 inferior, 2
    /// superior — printed, +1 level for each Discipline master card on it.
    pub fn discipline_level(&self, m: MinionId, d: &str) -> u8 {
        let printed = self.minions[m.index()]
            .printed_disciplines()
            .iter()
            .find(|p| p.eq_ignore_ascii_case(d))
            .map_or(0, |p| {
                if p.chars().all(|c| c.is_ascii_uppercase()) {
                    2
                } else {
                    1
                }
            });
        if printed == 2 {
            return 2;
        }
        let granted = self
            .discipline_grants(m)
            .filter(|g| g.eq_ignore_ascii_case(d))
            .count() as u8;
        // An equipment's level ("this vampire gets +1 level", Ritual
        // Goblet): no capacity with it, and nothing on an ally.
        let equipped = self
            .layers
            .iter()
            .filter(|l| match l.modifier {
                Modifier::BearerDisciplineLevel(g) => {
                    self.layer_active(l)
                        && g.eq_ignore_ascii_case(d)
                        && self.cards[l.source.index()].host == Some(m)
                        && self.minions[m.index()].is_vampire()
                }
                _ => false,
            })
            .count() as u8;
        let granted = granted + equipped;
        (printed + granted).min(2)
    }

    /// Does `m` have Discipline `tag`? Lower case at any level, upper case
    /// superior only.
    pub fn has_discipline(&self, m: MinionId, tag: &str) -> bool {
        let level = self.discipline_level(m, tag);
        if tag.chars().all(|c| c.is_ascii_uppercase()) {
            level == 2
        } else {
            level >= 1
        }
    }

    /// The capacity and clan of the vampire in `seat`'s uncontrolled slot
    /// `i`: a banished or created one's own, cards on it out of play and
    /// giving nothing (advanced rules 5.1.1); otherwise its crypt card's.
    pub fn slot_vampire(&self, seat: SeatId, i: usize) -> (u8, &'static str) {
        let slot = &self.seats[seat.index()].uncontrolled[i];
        match slot.banished {
            Some(ref b) => {
                let v = &self.minions[b.minion.index()];
                (v.printed_capacity().unwrap_or(1), v.clan().unwrap_or(""))
            }
            None => {
                let c = &CRYPT[slot.card as usize];
                (c.capacity, c.clan)
            }
        }
    }

    /// The printed sect of `seat`'s uncontrolled vampire `i`.
    pub fn slot_sect(&self, seat: SeatId, i: usize) -> Sect {
        let slot = &self.seats[seat.index()].uncontrolled[i];
        match slot.banished {
            Some(ref b) => self.minions[b.minion.index()]
                .underlying_sect()
                .unwrap_or(Sect::Independent),
            None => CRYPT[slot.card as usize].sect,
        }
    }

    /// The printed title of `seat`'s uncontrolled vampire `i`.
    pub fn slot_title(&self, seat: SeatId, i: usize) -> Title {
        let slot = &self.seats[seat.index()].uncontrolled[i];
        match slot.banished {
            Some(ref b) => self.minions[b.minion.index()].printed_title(),
            None => CRYPT[slot.card as usize].title,
        }
    }

    /// Add `n` blood to `seat`'s uncontrolled vampire `i`. Any amount, over
    /// its capacity too: the excess drains off only as it is influenced
    /// out, a choice (owner, 2026-09-26: rules-qa 28.3).
    pub fn add_uncontrolled_blood(&mut self, seat: SeatId, i: usize, n: u8) {
        let slot = &mut self.seats[seat.index()].uncontrolled[i];
        slot.counters = slot.counters.saturating_add(n);
    }

    /// `a` younger than `b`: both vampires, `a` of lower capacity.
    pub fn younger_than(&self, a: MinionId, b: MinionId) -> bool {
        matches!((self.capacity_of(a), self.capacity_of(b)), (Some(x), Some(y)) if x < y)
    }

    /// "Allies and younger vampires" (Stolen Police Cruiser, Perfect
    /// Paragon): an ally, or a vampire of capacity below `than`.
    pub fn ally_or_younger(&self, m: MinionId, than: u8) -> bool {
        self.capacity_of(m).is_none_or(|c| c < than)
    }

    /// Blood `m` may hold of `blood` gained: capped at capacity, unlimited
    /// for an ally — none for one that cannot gain life.
    pub fn cap_blood(&self, m: MinionId, blood: u8) -> u8 {
        let v = &self.minions[m.index()];
        if v.no_life_gain {
            return blood.min(v.blood);
        }
        match self.capacity_of(m) {
            Some(c) => blood.min(c),
            None => blood,
        }
    }

    /// Blood or life gained "not to exceed starting life" (Seraphina): a
    /// vampire's capacity, an ally's starting life — never below what it
    /// already has.
    pub fn cap_starting(&self, m: MinionId, blood: u8) -> u8 {
        let v = &self.minions[m.index()];
        if v.no_life_gain {
            return blood.min(v.blood);
        }
        let start = match v.kind {
            MinionKind::Ally { body, .. } => body.life,
            _ => self.capacity_of(m).unwrap_or(0),
        };
        blood.min(start.max(v.blood))
    }

    /// Add `n` blood (life) to `m`, capped (`cap_blood`); the rest goes to
    /// the blood bank.
    pub fn gain_blood(&mut self, m: MinionId, n: u8) {
        let b = self.cap_blood(m, self.minions[m.index()].blood.saturating_add(n));
        self.minions[m.index()].blood = b;
    }

    /// Burn up to `n` blood (life) from `m`.
    pub fn burn_blood(&mut self, m: MinionId, n: u8) {
        let v = &mut self.minions[m.index()];
        v.blood -= n.min(v.blood);
    }

    /// Blood above capacity drains off from every vampire in play,
    /// however the capacity went (a Discipline master card burned, or out
    /// of play with its ousted player; advanced rules 5.1.1). Out of play,
    /// a vampire keeps it.
    /// "After this card leaves play, burn all the hostage counters"
    /// (Carver's Meat Packing and Storage): none while no copy is present.
    pub fn release_hostages(&mut self) {
        if !self.minions.iter().any(|m| m.hostage > 0) {
            return;
        }
        let held = self.cards.iter().any(|c| {
            c.present()
                && cards::lookup(c.card).is_some_and(|d| {
                    d.riders.iter().any(|r| {
                        r.effects
                            .iter()
                            .any(|e| matches!(e, cards::Effect::HostageOnTorpor(_)))
                    })
                })
        });
        if !held {
            for m in &mut self.minions {
                m.hostage = 0;
            }
        }
    }

    pub fn drain_all_excess(&mut self) {
        for i in 0..self.minions.len() {
            let m = MinionId(i as u32);
            let v = &self.minions[i];
            // Only cards on it raise its capacity: at or below the printed
            // one, nothing to read — unless Wormwood ceils it.
            let ceiled = self.capacity_ceiling().is_some_and(|x| v.blood > x);
            if !v.in_play || v.printed_capacity().is_none_or(|p| v.blood <= p) && !ceiled {
                continue;
            }
            if let Some(c) = self.capacity_of(m) {
                let v = &mut self.minions[i];
                v.blood = v.blood.min(c);
            }
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
    pub fn hand_picks(&self, seat: SeatId, of: cards::CardFilter) -> Vec<u8> {
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
        let master = LIBRARY[c.card as usize].types.contains(&CardType::Master);
        // "You still control this card" (Puppet Master [DOM]).
        let still_yours = cards::lookup(c.card).is_some_and(|d| {
            d.modes
                .get(c.mode as usize)
                .is_some_and(|m| m.effects.contains(&cards::Effect::StillYours))
        });
        match c.host {
            Some(h) if !master && !still_yours => self.minions[h.index()].owner,
            _ => c.controller,
        }
    }

    /// A copy of this unique card is in play under `seat`, contested or not:
    /// another is not playable, nor equipped by a search ("you cannot
    /// voluntarily contest cards with yourself", rulebook "Contested
    /// Cards"; spec §1.13.1; rules-qa 7.1).
    pub fn controls_unique_copy(&self, seat: SeatId, card: u16) -> bool {
        LIBRARY[card as usize].labels.contains(&Label::Unique)
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
    pub fn printed_mod(&self, m: MinionId, stat: Stat, ctx: PrintedCtx) -> i32 {
        use crate::cards::PrintedTo;
        let owner = self.minions[m.index()].owner;
        let mut total = 0;
        for (i, v) in self.minions.iter().enumerate() {
            let source = MinionId(i as u32);
            let Some(def) = v
                .crypt_card()
                .and_then(cards::vampire)
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

    /// Does `m` meet a card's requirement of one of `clans` for playing it
    /// — `m` playing it, or with `by_methuselah` its Methuselah? Its own
    /// clan, or a printed "can play cards requiring a Tremere as if he were
    /// a Tremere" (Abraham Mellon; "you and Lady Scarlett", its Methuselah
    /// too). Only for playing the card normally, not for a card another
    /// puts in play (LSJ 20050119); the effects the card has once in play
    /// read the clan itself (`of_card_clan`; LSJ 20101013).
    pub fn meets_clan(&self, m: MinionId, clans: &[&str], by_methuselah: bool) -> bool {
        let v = &self.minions[m.index()];
        v.clan().is_some_and(|c| clans.contains(&c))
            || v.present()
                && cards::vampire_of(v)
                    .and_then(|d| d.plays_as)
                    .is_some_and(|p| clans.contains(&p.clan) && (p.methuselah || !by_methuselah))
    }

    /// Is `m` of the clan `card` requires, itself — what the card's effects
    /// in play read: a vampire playing as the clan does not qualify (Lady
    /// Scarlett chosen by Toreador Grand Ball unlocks as normal, ANK
    /// 20220816-2).
    pub fn of_card_clan(&self, m: MinionId, card: u16) -> bool {
        let clans = LIBRARY[card as usize].clans;
        clans.is_empty()
            || self.minions[m.index()]
                .clan()
                .is_some_and(|c| clans.contains(&c))
    }

    /// What `m`'s printed text takes off the cost of `card`, played in
    /// `mode`, that it plays or puts in play ("Equipment cost Anne-Marie -1
    /// pool"; "Animals … cost Kuyén -1 blood or pool"; "Cards requiring
    /// Hecata and/or Oblivion cost Roger -1 blood"): reductions reaching a
    /// card put in play by another (Piper, Contraband; rules-qa 36.5, ANK
    /// 20210913).
    pub fn printed_cost_off(&self, m: MinionId, card: u16, mode: u8) -> CostOff {
        let ctx = PrintedCtx {
            card: Some((card, mode)),
            ..PrintedCtx::default()
        };
        let n = |stat| self.printed_mod(m, stat, ctx).max(0) as u8;
        CostOff {
            pool: n(Stat::PoolCostOff),
            blood: n(Stat::BloodCostOff),
            either: n(Stat::CostOff),
        }
    }

    /// A printed-modifier predicate, read on the vampire whose text it is
    /// (`source`), the modified minion (`actor`) and the opposing minion.
    fn printed_holds(
        &self,
        r: Requirement,
        source: MinionId,
        actor: MinionId,
        ctx: PrintedCtx,
    ) -> bool {
        use crate::cards::Requirement as R;
        let src = &self.minions[source.index()];
        let me = &self.minions[actor.index()];
        let opp = ctx.opponent.map(|o| &self.minions[o.index()]);
        match r {
            R::ThisVampireReady => src.ready(),
            R::ActionDirected => ctx.directed == Some(true),
            R::ActionUndirected => ctx.directed == Some(false),
            R::IsBleed => ctx.kind == Some(ActionKind::Bleed),
            R::TargetSeatControlsClan(clans) => ctx.target.is_some_and(|s| {
                self.minions_of(s).any(|m| {
                    self.minions[m.index()]
                        .clan()
                        .is_some_and(|c| clans.contains(&c))
                })
            }),
            R::IsPolitical => ctx.kind == Some(ActionKind::Political),
            R::OpponentClan(c) => opp.and_then(|o| o.clan()) == Some(c),
            // "In combat with a werewolf" (Mark Decker): an ally of the
            // subtype.
            R::OpponentSubtype(l) => opp
                .and_then(|o| o.kind.ally_card())
                .is_some_and(|card| LIBRARY[card as usize].labels.contains(&l)),
            // Younger and older compare vampires: an ally has no capacity.
            R::OpponentYounger => ctx.opponent.is_some_and(|o| self.younger_than(o, actor)),
            R::OpponentAllyOrYounger => {
                opp.is_some_and(|o| o.is_ally())
                    || ctx.opponent.is_some_and(|o| self.younger_than(o, actor))
            }
            R::OpponentOlder => ctx.opponent.is_some_and(|o| self.younger_than(actor, o)),
            // The modified minion carries the corruption counters of the
            // printed vampire's Methuselah (Faruq Abd al-Qadir).
            R::ActorCorruptedBySource => me.corruption_of(src.owner) > 0,
            R::OpponentTitled => ctx
                .opponent
                .is_some_and(|o| self.title_of(o) != Title::None),
            R::ActorClan(c) => me.clan() == Some(c),
            R::ActorTitleIn(classes) => classes.iter().any(|c| c.matches(self.title_of(actor))),
            R::PoolAtMost(n) => self.seats[src.owner.index()].pool <= n as i32,
            R::ControlsLocation | R::ControlsNoLocation => {
                let any = self.cards.iter().enumerate().any(|(i, c)| {
                    c.present()
                        && self.card_controller(CardId(i as u32)) == src.owner
                        && LIBRARY[c.card as usize].labels.contains(&Label::Location)
                });
                any == (r == R::ControlsLocation)
            }
            // The card played, in the mode played ("requiring" = played
            // using, rules-qa 38.4).
            R::PlayedCard(f) => ctx.card.is_some_and(|(c, _)| f.matches(c)),
            R::PlayedRequiresDiscipline(d) => ctx.card.is_some_and(|(c, mode)| {
                crate::plays::played_disciplines(c, mode)
                    .iter()
                    .any(|t| cards::discipline_key(t) == d)
            }),
            R::AnyOf(alternatives) => alternatives.iter().any(|reqs| {
                reqs.iter()
                    .all(|&q| self.printed_holds(q, source, actor, ctx))
            }),
            // Unique: a crypt card — not a created vampire (Childe of the
            // Revolution, Waters of Duat; rules-qa 41.5).
            R::ControlsNoReadyOlderOfClan(clan) => !self.minions_of(src.owner).any(|o| {
                let w = &self.minions[o.index()];
                matches!(w.kind, MinionKind::Vampire(_))
                    && w.ready()
                    && w.clan() == Some(clan)
                    && self.younger_than(source, o)
            }),
            R::ControlsReadyTitledOther(classes) => self.minions_of(src.owner).any(|o| {
                o != source
                    && self.minions[o.index()].ready()
                    && classes.iter().any(|c| c.matches(self.title_of(o)))
            }),
            R::PredatorMoreReadyMinions => {
                let ready = |s: SeatId| {
                    self.minions_of(s)
                        .filter(|&o| self.minions[o.index()].ready())
                        .count()
                };
                let predator = self.predator(src.owner);
                predator != src.owner && ready(predator) > ready(src.owner)
            }
            R::PreyPoolAtMost(n) => {
                let prey = self.prey(src.owner);
                prey != src.owner && self.seats[prey.index()].pool <= n as i32
            }
            R::ControlsVampireNamed(name) => self
                .minions
                .iter()
                .any(|v| v.owner == src.owner && v.present() && v.name() == name),
            _ => unreachable!("{r:?} is not a printed-modifier predicate"),
        }
    }

    fn printed_count(&self, per: cards::Per, source: MinionId) -> i32 {
        match per {
            cards::Per::One => 1,
            cards::Per::UniqueEquipmentOn => self
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
            .map(|m| self.printed_mod(m, Stat::DiscardPhaseActions, PrintedCtx::default()))
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
                    .and_then(cards::vampire)
                    .map(|d| (MinionId(i as u32), d))
            })
            .flat_map(|(m, d)| {
                d.printed
                    .iter()
                    .filter(|p| p.stat == Stat::HandSize)
                    .map(move |p| (m, p))
            })
            .filter(|(m, p)| {
                p.requires
                    .iter()
                    .all(|&r| self.printed_holds(r, *m, *m, PrintedCtx::default()))
            })
            .map(|(_, p)| p.n as i32)
            .sum();
        let seat = &self.seats[s.index()];
        let mut size =
            HAND_SIZE as i32 + seat.hand_bonus as i32 + seat.hand_bonus_discard as i32 + printed;
        // "While the employer is in combat, the opposing minion's
        // controller gets -1 hand size" (Raptor [ANI]).
        if let Some(pair) = self.combatants {
            for (i, &m) in pair.iter().enumerate() {
                if self.minions[pair[1 - i].index()].owner == s {
                    size -= self.combat_hand_penalty(m) as i32;
                }
            }
        }
        for l in self.controlled_layers(s) {
            match l.modifier {
                Modifier::HandSize(n) => size += n as i32,
                Modifier::HandSizePerCounter => {
                    size += self.cards[l.source.index()].counters as i32
                }
                _ => {}
            }
        }
        size.max(0) as usize
    }

    /// Transfers seat `s` gets from cards in play (Information Highway).
    pub fn transfer_bonus(&self, s: SeatId) -> u8 {
        let printed: i32 = self
            .minions_of(s)
            .map(|m| self.printed_mod(m, Stat::Transfers, PrintedCtx::default()))
            .sum();
        let layers: u8 = sum_n!(self.controlled_layers(s), Modifier::Transfers);
        layers + printed.max(0) as u8
    }

    /// Blood a hunt gains above the rulebook's 1 ("Sabbat vampires you
    /// control get +1 hunt", The Hungry Coyote).
    pub fn hunt_bonus(&self, m: MinionId) -> u8 {
        let owner = self.minions[m.index()].owner;
        self.in_play_layers()
            .map(|l| match l.modifier {
                Modifier::OwnSectHunt(sect, n)
                    if self.card_controller(l.source) == owner && self.sect_of(m) == Some(sect) =>
                {
                    n
                }
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
    pub fn rider_holds(&self, r: &cards::Rider, id: CardId) -> bool {
        let c = &self.cards[id.index()];
        !self.text_suppressed(id)
            && r.requires.iter().all(|&req| match req {
                Requirement::CountersAtLeast(n) => c.counters >= n,
                Requirement::CountersAtMost(n) => c.counters <= n,
                Requirement::SuperiorVersion => c.mode > 0,
                Requirement::BasicVersion => c.mode == 0,
                Requirement::OneCopyPerTurn => true,
                Requirement::ActiveControlsNonSect(sect) => self
                    .minions_of(self.active)
                    .any(|m| self.minions[m.index()].is_vampire() && self.sect_of(m) != Some(sect)),
                _ => match c.bearer() {
                    Some(h) => cards::holds_for_host(req, self, h),
                    None => true,
                },
            })
    }

    /// The riders on `hook` of every card in the arena, in arena order —
    /// in play or not: the caller reads each card as it gets to it (one
    /// rider per hook on a card, but for `Hook::UnlockPhase`).
    pub fn card_riders(&self, hook: cards::Hook) -> Vec<(CardId, &'static cards::Rider)> {
        self.cards
            .iter()
            .enumerate()
            .filter_map(|(i, c)| Some((CardId(i as u32), cards::lookup(c.card)?)))
            .flat_map(|(id, d)| {
                d.riders
                    .iter()
                    .filter(move |r| r.hook == hook)
                    .map(move |r| (id, r))
            })
            .collect()
    }

    /// Does minion `m` have a card in play of this kind on it?
    pub fn holds_a(&self, m: MinionId, h: Holding) -> bool {
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
        let Some(def) = cards::lookup(card) else {
            return true;
        };
        def.modes
            .iter()
            .flat_map(|mode| mode.requires.iter())
            .all(|r| match *r {
                Requirement::ActorWithout(h) => {
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
                    && !cards::became(c.card, c.mode)
                    && (cards::CardFilter::EquipmentWithout(&[]).matches(c.card)
                        || cards::becomes_weapon(c.card))
            })
            .map(|(i, _)| CardId(i as u32))
            .collect()
    }

    /// What the cards in play on minion `m` add to `stat` during its
    /// action of `kind` (`Modifier::Bearer`; contested cards give
    /// nothing).
    /// How card `c` in play reduces its controller's costs, if it does.
    pub fn cost_source_of(&self, c: CardId) -> Option<cards::CostSource> {
        self.in_play_layers()
            .filter(|l| l.source == c)
            .find_map(|l| match l.modifier {
                Modifier::CostSource(k) => Some(k),
                _ => None,
            })
    }

    /// Secure Havens on `m`.
    pub fn havens_on(&self, m: MinionId) -> u8 {
        self.borne(m)
            .filter(|l| l.modifier == Modifier::Haven)
            .count() as u8
    }

    /// British Museum: what `m`'s equip with `card` costs less.
    pub fn equip_discount(&self, m: MinionId, card: u16) -> u8 {
        if !cards::plain_equipment(card) {
            return 0;
        }
        let owner = self.minions[m.index()].owner;
        sum_n!(self.controlled_layers(owner), Modifier::PlainEquipDiscount)
    }

    /// A Mithraic Mysteries member: a Mithraic Cultist on it.
    pub fn mithraic_member(&self, m: MinionId) -> bool {
        self.bears(m, Modifier::MithraicMember)
    }

    /// /Mithras/, or a Mithraic Mysteries member.
    pub fn is_mithraic(&self, m: MinionId) -> bool {
        self.minions[m.index()].name() == "Mithras" || self.mithraic_member(m)
    }

    /// Votes the host's cards give it (Mithraic Cultist: "+1 vote while you
    /// control /Mithras/").
    pub fn host_votes(&self, m: MinionId) -> i32 {
        let owner = self.minions[m.index()].owner;
        self.borne(m)
            .map(|l| match l.modifier {
                Modifier::HostVotesWhileControlling { name, n }
                    if self
                        .minions_of(owner)
                        .any(|o| self.minions[o.index()].name() == name) =>
                {
                    n as i32
                }
                _ => 0,
            })
            .sum()
    }

    /// Charisma on `m`: its recruit ally actions cost this much less.
    pub fn recruit_discount(&self, m: MinionId) -> u8 {
        sum_n!(self.borne(m), Modifier::HostRecruitDiscount)
    }

    /// Optional maneuvers each combat for `m` from clan-wide layers
    /// (Brujah Debate).
    pub fn clan_combat_maneuvers(&self, m: MinionId) -> u8 {
        let clan = self.minions[m.index()].clan();
        self.in_play_layers()
            .filter(|l| matches!(l.modifier, Modifier::ClanCombatManeuver(c) if clan == Some(c)))
            .count() as u8
    }

    /// The votes `m`'s title cards give it in referendums it calls, while
    /// it is of their clans (Fee Stake: Boston).
    pub fn calling_votes(&self, m: MinionId) -> i32 {
        let clan = self.minions[m.index()].clan();
        self.borne(m)
            .filter(|l| {
                matches!(l.modifier, Modifier::HostVoteWhenCalling(cs)
                    if clan.is_some_and(|c| cs.contains(&c)))
            })
            .count() as i32
    }

    /// The layers reading who a bleed is against (Tier of Souls [ANI],
    /// Aranthebes, The Immortal), for `m` bleeding `target`.
    pub fn bleed_against_mod(&self, m: MinionId, target: SeatId) -> i32 {
        self.in_play_layers()
            .map(|l| {
                let c = &self.cards[l.source.index()];
                let controller = self.card_controller(l.source);
                match l.modifier {
                    Modifier::HostBleedAgainstPrey(n)
                        if c.bearer() == Some(m) && target == self.prey(controller) =>
                    {
                        n as i32
                    }
                    Modifier::OwnBleedWhilePreyTorpor(n)
                        if self.minions[m.index()].owner == controller
                            && self.minions[m.index()].is_vampire()
                            && self.minions_of(self.prey(controller)).any(|t| {
                                self.minions[t.index()].is_vampire()
                                    && self.minions[t.index()].in_torpor
                            }) =>
                    {
                        n as i32
                    }
                    Modifier::YoungBleedAgainstController { max, n }
                        if !c.locked
                            && target == controller
                            && self.minions[m.index()].is_vampire()
                            && self.capacity_of(m).is_some_and(|cap| cap <= max) =>
                    {
                        n as i32
                    }
                    _ => 0,
                }
            })
            .sum()
    }

    pub fn bearer_mod(&self, m: MinionId, stat: Stat, kind: ActionKind) -> i32 {
        self.bearer_mods(m, stat, kind).map(|(_, n)| n).sum()
    }

    /// Can `actor` perform an action directed at `m`? Not without one of
    /// the Disciplines a card on `m` names ("minions without Auspex cannot
    /// perform actions directed at this vampire", Shadow Cloak [OBL]).
    pub fn may_direct_at(&self, actor: MinionId, m: MinionId) -> bool {
        // Secure Haven: "cannot be the target of other Methuselahs'
        // actions" (the caller reads only another Methuselah's minions).
        if self.havens_on(m) > 0
            && self.minions[actor.index()].owner != self.minions[m.index()].owner
        {
            return false;
        }
        self.borne(m).all(|l| {
            !matches!(l.modifier, Modifier::DirectedOnlyBy(ds)
                if !ds.iter().any(|d| self.has_discipline(actor, d)))
        })
    }

    /// Cards on `m` whose counter burns instead of its unlocking as normal
    /// (Touch of Oblivion).
    pub fn counter_instead_of_unlock(&self, m: MinionId) -> Vec<CardId> {
        self.borne(m)
            .filter(|l| l.modifier == Modifier::CounterInsteadOfUnlock)
            .map(|l| l.source)
            .collect()
    }

    /// The discount on `rescuer`'s rescue of `target` from its cards in play
    /// and its printed text (Adelino Da Silva, Doc Martina), and the blood
    /// the rescued vampire gains on success (Saulot's Healing Touch).
    pub fn rescue_discount(&self, rescuer: MinionId, target: MinionId) -> (u8, u8) {
        let clan = self.minions[target.index()].clan();
        let printed = self
            .printed_mod(
                rescuer,
                Stat::RescueDiscount,
                PrintedCtx {
                    opponent: Some(target),
                    ..PrintedCtx::default()
                },
            )
            .max(0) as u8;
        self.borne(rescuer)
            .fold((printed, 0), |(d, g), l| match l.modifier {
                Modifier::RescueDiscount { not_clan, n, gain }
                    if not_clan.is_none() || clan != not_clan =>
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
        stat: Stat,
        kind: ActionKind,
    ) -> i32 {
        self.bearer_mods(m, stat, kind)
            .map(|(card, n)| {
                let equipment = cards::is_equipment_in_play(card);
                if equipment {
                    n.min(0)
                } else {
                    n
                }
            })
            .sum()
    }

    /// Does a clan-wide layer from `source` reach `m`?
    fn clan_stat_reaches(
        &self,
        source: CardId,
        m: MinionId,
        clan: &str,
        own: bool,
        titled: bool,
    ) -> bool {
        self.minions[m.index()].clan() == Some(clan)
            && (!own || self.minions[m.index()].owner == self.card_controller(source))
            && (!titled || self.title_of(m) != crate::crypt::Title::None)
    }

    /// Votes the clan-wide layers give `m` (New Carthage: "titled Brujah get
    /// +1 vote, Ventrue get -1 vote").
    pub fn clan_votes(&self, m: MinionId) -> i32 {
        self.in_play_layers()
            .map(|l| match l.modifier {
                Modifier::ClanStat {
                    clan,
                    stat: Stat::Votes,
                    n,
                    own,
                    titled,
                    ..
                } if self.clan_stat_reaches(l.source, m, clan, own, titled) => n as i32,
                _ => 0,
            })
            .sum()
    }

    /// The bearer modifiers of the cards in play on minion `m` for `stat`
    /// during its action of `kind`, with the card each comes from.
    fn bearer_mods(
        &self,
        m: MinionId,
        stat: Stat,
        kind: ActionKind,
    ) -> impl Iterator<Item = (u16, i32)> + '_ {
        self.in_play_layers().filter_map(move |l| match l.modifier {
            Modifier::Bearer {
                stat: s,
                n,
                during,
                who,
            } if s == stat
                && self.cards[l.source.index()].bearer() == Some(m)
                && (during.is_empty() || during.contains(&kind))
                && who.is(self, m) =>
            {
                Some((self.cards[l.source.index()].card, n as i32))
            }
            // Every minion of the clan (Week of Nightmares: "Ravnos get
            // +1 bleed and +1 strength").
            Modifier::ClanStat {
                clan,
                stat: s,
                n,
                own,
                titled,
                during,
            } if s == stat
                && self.clan_stat_reaches(l.source, m, clan, own, titled)
                && (during.is_empty() || during.contains(&kind)) =>
            {
                Some((self.cards[l.source.index()].card, n as i32))
            }
            _ => None,
        })
    }

    /// Damage the retainers on minion `m` inflict on its opponent each
    /// round during normal strike resolution (`Modifier::RetainerDamage`).
    pub fn retainer_damage(&self, m: MinionId) -> u8 {
        sum_n!(self.hosted(m), Modifier::RetainerDamage)
    }

    /// What `viewer` legally knows of the state (DESIGN §5): a copy with
    /// every card identity they may not see replaced by [`HIDDEN`], in
    /// place, so every zone keeps its size and every counter, blood, pool
    /// and flag stays. Hidden:
    ///
    /// - every seat's **library** and **crypt**: the order is unknown to
    ///   all (a search shuffles afterward; the cards Heart of Nizchetus
    ///   puts at the bottom, known to their player, are hidden too —
    ///   remembering them is the policy's business, from the decisions it
    ///   made). The viewer's own keep their contents, sorted: a Methuselah
    ///   knows their decklist and sees every card leaving or entering
    ///   their library and crypt, so what remains is theirs to know;
    /// - another seat's **hand**, unless `hand_visible_to(seat, viewer)`
    ///   (Revelations [AUS], Owl Companion);
    /// - another seat's face-down **uncontrolled** vampires (rulebook
    ///   "Setup": a Methuselah looks at their own at any time); a banished
    ///   vampire's slot stays (it left play in view of all, its arena entry
    ///   and the cards on it are public), as do every slot's counters;
    /// - the **cards stored face down** on a card in play (Shilmulo Tarot,
    ///   `Effect::StoreTopCards`: "you can look at the cards at any time"),
    ///   except to its controller; face-up ones stay (Black Market Cache,
    ///   Fleshforge Chamber).
    ///
    /// Everything else is public and kept: ash heaps (`ash`, `ash_crypt`),
    /// removed cards, minions and cards in play (a contested card turned
    /// face down keeps its identity: it was in play face up), pool, blood,
    /// counters, the cards played but not replaced yet (`held`: a count).
    /// The engine's random source is not in the state. The contexts of the
    /// action, combat and referendum under way (`Engine::action`,
    /// `Engine::combat`, `Engine::referendum`) and the as-played stack hold
    /// announced, public information, with one exception an observation
    /// adding them must redact: a card named from hand as a play or action
    /// is announced (`PlayCtx::named`, Pack Alpha; `ActionCtx::exchange`'s
    /// hand card, Lenelle) need not be shown (advanced rules §1.14.2).
    pub fn legal_view(&self, viewer: SeatId) -> GameState {
        let mut v = self.clone();
        let hide = |cards: &mut Vec<u16>| cards.iter_mut().for_each(|c| *c = HIDDEN);
        for (i, seat) in v.seats.iter_mut().enumerate() {
            let s = SeatId(i as u8);
            if s == viewer {
                seat.library.sort_unstable();
                seat.crypt.sort_unstable();
            } else {
                hide(&mut seat.library);
                hide(&mut seat.crypt);
            }
            if !self.hand_visible_to(s, viewer) {
                hide(&mut seat.hand);
            }
            if s != viewer {
                for slot in &mut seat.uncontrolled {
                    if slot.banished.is_none() {
                        slot.card = HIDDEN;
                    }
                }
            }
        }
        for (i, c) in v.cards.iter_mut().enumerate() {
            let face_down = cards::lookup(c.card).is_some_and(|d| {
                d.modes.iter().chain(d.abilities).any(|m| {
                    m.effects
                        .iter()
                        .any(|e| matches!(e, cards::Effect::StoreTopCards(_)))
                })
            });
            if face_down && self.card_controller(CardId(i as u32)) != viewer {
                hide(&mut c.stored);
            }
        }
        v
    }

    /// Whether `viewer` sees `seat`'s hand: their own, or an open hand — the
    /// prey of a Revelations [AUS]'s controller, or `seat`'s minion in
    /// combat with the employer of an Owl Companion. The legal view
    /// (`legal_view`) reads it.
    pub fn hand_visible_to(&self, seat: SeatId, viewer: SeatId) -> bool {
        viewer == seat
            || self.in_play_layers().any(|l| {
                l.modifier == Modifier::PreyOpenHand
                    && self.prey(self.card_controller(l.source)) == seat
            })
            || self.combatants.is_some_and(|pair| {
                (0..2).any(|i| {
                    self.minions[pair[1 - i].index()].owner == seat
                        && self.hosts(pair[i], Modifier::OpponentOpenHandInCombat)
                })
            })
    }

    /// Hand size the retainers on combatant `m` take from its opponent's
    /// controller (`Modifier::OpponentHandSizeInCombat`).
    pub fn combat_hand_penalty(&self, m: MinionId) -> u8 {
        sum_n!(self.hosted(m), Modifier::OpponentHandSizeInCombat)
    }

    /// `m` cannot have or use equipment or retainers (Bone Shambler's own
    /// text): it takes none, by any means.
    pub fn no_equipment_or_retainers(&self, m: MinionId) -> bool {
        self.bears(m, Modifier::NoEquipmentOrRetainers)
    }

    /// `m` cannot be the target of a directed action requiring one of the
    /// Disciplines of `tags` (a mode's tags, any level).
    pub fn immune_to_directed(&self, m: MinionId, tags: &[&str]) -> bool {
        self.borne(m).any(|l| {
            matches!(l.modifier, Modifier::ImmuneToDirectedActions(ds)
                if tags.iter().any(|t| ds.contains(&t.to_ascii_lowercase().as_str())))
        })
    }

    /// What `m` — an ally — may play "as a vampire", if anything.
    pub fn plays_as_vampire(&self, m: MinionId) -> Option<cards::AsVampire> {
        self.borne(m).find_map(|l| match l.modifier {
            Modifier::PlaysAsVampire(g) => Some(g),
            _ => None,
        })
    }

    /// Allies cannot block `m` (Libertas on it; Stanislava's own text).
    pub fn allies_cannot_block(&self, m: MinionId) -> bool {
        self.hosts(m, Modifier::AlliesCannotBlockBearer)
            || self.printed_mod(m, Stat::AlliesCannotBlock, PrintedCtx::default()) > 0
    }

    /// The card's text is lost: a retainer of a minion in combat with
    /// Stanislava (rules-qa 49.3: its abilities, layers and riders, for
    /// that combat; it stays a retainer whose life takes damage).
    pub fn text_suppressed(&self, id: CardId) -> bool {
        let Some(pair) = self.combatants else {
            return false;
        };
        let c = &self.cards[id.index()];
        let Some(host) = c.host else {
            return false;
        };
        let Some(i) = pair.iter().position(|&m| m == host) else {
            return false;
        };
        LIBRARY[c.card as usize].types.contains(&CardType::Retainer)
            && self.printed_mod(
                pair[1 - i],
                Stat::OpposingRetainersLoseText,
                PrintedCtx::default(),
            ) > 0
    }

    /// The strikes of `m` cannot be dodged (Aggressive Corpse's own text).
    pub fn undodgeable(&self, m: MinionId) -> bool {
        self.bears(m, Modifier::UndodgeableStrikes)
    }

    /// `m` does not hunt as normal (Week of Nightmares: its clan).
    pub fn cannot_hunt(&self, m: MinionId) -> bool {
        let clan = self.minions[m.index()].clan();
        self.in_play_layers().any(|l| match l.modifier {
            Modifier::ClanCannotHunt(c) => Some(c) == clan,
            // "Vampires cannot hunt unless they must hunt" (Restricted
            // Vitae).
            Modifier::OnlyMustHunt => !self.must_hunt(m),
            _ => false,
        })
    }

    /// Must `m` hunt: a vampire with no blood, or made to?
    pub fn must_hunt(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.is_vampire() && (v.blood == 0 || v.must_hunt)
    }

    /// `m` cannot block (Pentex™ Subversion): a card on it.
    pub fn cannot_block(&self, m: MinionId) -> bool {
        self.hosts(m, Modifier::HostCannotBlock)
    }

    /// `m`'s actions of `kind` cannot be blocked (Toreador Grand Ball: "the
    /// other Toreador's non-bleed actions cannot be blocked", still after it
    /// bled, LSJ 19970814).
    pub fn unblockable(&self, m: MinionId, kind: ActionKind) -> bool {
        kind != ActionKind::Bleed
            && self.in_play_layers().any(|l| {
                l.modifier == Modifier::SecondChosenUnblockable
                    && self.cards[l.source.index()].chosen.get(1) == Some(&m)
                    && self.of_card_clan(m, self.cards[l.source.index()].card)
            })
    }

    /// `m` cannot block vampires (Vagabond Mystic's own text).
    pub fn cannot_block_vampires(&self, m: MinionId) -> bool {
        self.bears(m, Modifier::CannotBlockVampires)
    }

    /// "Does not unlock as normal": a card on `m` (Puppet Master [DOM]),
    /// or one that chose it to (Toreador Grand Ball's locked Toreador).
    pub fn does_not_unlock(&self, m: MinionId) -> bool {
        self.in_play_layers().any(|l| {
            let c = &self.cards[l.source.index()];
            match l.modifier {
                Modifier::HostDoesNotUnlock => c.host == Some(m),
                Modifier::FirstChosenDoesNotUnlock => {
                    c.chosen.first() == Some(&m) && self.of_card_clan(m, c.card)
                }
                _ => false,
            }
        })
    }

    /// Whether `m` cannot play `card` (Preternatural Strength: "they cannot
    /// play cards named Torn Signpost").
    pub fn bars_playing(&self, m: MinionId, card: u16) -> bool {
        let name = LIBRARY[card as usize].name;
        self.bears(m, Modifier::BearerCannotPlay(name))
    }

    /// `striker`, in combat with `m`, cannot strike: combat ends — a
    /// retainer on `m` (Dog Pack), or `m`'s printed text (Faruq Abd
    /// al-Qadir: "minions with any of your corruption counters").
    pub fn combat_ends_barred(&self, striker: MinionId, m: MinionId) -> bool {
        let ctx = PrintedCtx {
            opponent: Some(m),
            ..PrintedCtx::default()
        };
        self.printed_mod(striker, Stat::NoCombatEnds, ctx) > 0
            || self.hosts(m, Modifier::NoCombatEndsAgainstHost)
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

    /// Events in play other than `but` (The Uncoiling's X).
    pub fn other_events(&self, but: CardId) -> usize {
        (0..self.cards.len())
            .filter(|&i| {
                let c = &self.cards[i];
                i != but.index()
                    && c.present()
                    && !c.effect_only
                    && LIBRARY[c.card as usize].types.contains(&CardType::Event)
            })
            .count()
    }

    /// Gehenna events in play (uncontested, not a lingering effect).
    pub fn gehenna_in_play(&self) -> usize {
        self.cards
            .iter()
            .filter(|c| {
                c.present()
                    && !c.effect_only
                    && LIBRARY[c.card as usize].labels.contains(&Label::Gehenna)
            })
            .count()
    }

    /// "Gehenna events cannot be played" (The Fourth Cycle in play).
    pub fn gehenna_barred(&self) -> bool {
        self.global(Modifier::NoGehennaEvents).is_some()
    }

    /// `m`'s sect: its own, unless its sect is no more (Fall of the
    /// Camarilla, Fall of the Sabbat: considered Independent instead, the
    /// most recent change included; LSJ 20080510).
    pub fn sect_of(&self, m: MinionId) -> Option<Sect> {
        let s = self.minions[m.index()].underlying_sect()?;
        let fallen = matches!(s, Sect::Camarilla | Sect::Sabbat)
            && self.global(Modifier::SectFallen(s)).is_some();
        Some(if fallen { Sect::Independent } else { s })
    }

    /// Blood vampires attempting to block `m` burn (Archon, per copy).
    pub fn blockers_of_burn(&self, m: MinionId) -> u8 {
        sum_n!(self.hosted(m), Modifier::BlockersOfHostBurn)
    }

    /// Can no blood hunt be called on `m` (Archon)?
    pub fn blood_hunt_barred(&self, m: MinionId) -> bool {
        self.hosts(m, Modifier::NoBloodHuntOnHost)
    }

    /// Does `s` control a copy of this library card in play ("a Methuselah
    /// can have only one", Raising the Portcullis)?
    pub fn controls_card_copy(&self, s: SeatId, card: u16) -> bool {
        (0..self.cards.len()).any(|i| {
            let c = &self.cards[i];
            c.in_play
                && !c.effect_only
                && c.card == card
                && self.card_controller(CardId(i as u32)) == s
        })
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
        // A created vampire is non-unique (advanced rules 5.1.3).
        if matches!(me.kind, MinionKind::Created { .. }) {
            return Vec::new();
        }
        if me
            .kind
            .ally_card()
            .is_some_and(|c| !LIBRARY[c as usize].labels.contains(&Label::Unique))
        {
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
        l.scope == Scope::Action
            || (self.cards[l.source.index()].present() && !self.text_suppressed(l.source))
    }

    /// The layers of the uncontested cards in play (`Scope::InPlay`).
    pub fn in_play_layers(&self) -> impl Iterator<Item = &Layer> + '_ {
        self.layers
            .iter()
            .filter(|l| l.scope == Scope::InPlay && self.layer_active(l))
    }

    /// The in-play layers of the cards `s` controls.
    pub fn controlled_layers(&self, s: SeatId) -> impl Iterator<Item = &Layer> + '_ {
        self.in_play_layers()
            .filter(move |l| self.card_controller(l.source) == s)
    }

    /// The in-play layers of the cards `m` bears (`CardInPlay::bearer`: on
    /// it, or the ally it is).
    pub fn borne(&self, m: MinionId) -> impl Iterator<Item = &Layer> + '_ {
        self.in_play_layers()
            .filter(move |l| self.cards[l.source.index()].bearer() == Some(m))
    }

    /// The in-play layers of the cards on `m` (`CardInPlay::host`).
    pub fn hosted(&self, m: MinionId) -> impl Iterator<Item = &Layer> + '_ {
        self.in_play_layers()
            .filter(move |l| self.cards[l.source.index()].host == Some(m))
    }

    /// Does a card `m` bears give `modifier`?
    pub fn bears(&self, m: MinionId, modifier: Modifier) -> bool {
        self.borne(m).any(|l| l.modifier == modifier)
    }

    /// Does a card on `m` give `modifier`?
    pub fn hosts(&self, m: MinionId, modifier: Modifier) -> bool {
        self.hosted(m).any(|l| l.modifier == modifier)
    }

    /// The title card on vampire `m`, with the title it represents: face
    /// down under a title contest included, not under a card contest (a
    /// unique title card contests as a card first, its title out of play
    /// with it: rules-qa 19.1).
    pub fn title_card_on(&self, m: MinionId) -> Option<(CardId, Title)> {
        let title_contested = self.minions[m.index()].title_contested;
        self.cards.iter().enumerate().find_map(|(i, c)| {
            (c.in_play && c.host == Some(m) && (!c.contested || title_contested))
                .then(|| cards::card_title(c.card))
                .flatten()
                .map(|t| (CardId(i as u32), t))
        })
    }

    /// The title vampire `m` claims, contested or not: its title card's,
    /// a title gained without a card, or its printed one unless lost.
    pub fn claimed_title(&self, m: MinionId) -> Title {
        let v = &self.minions[m.index()];
        if v.is_ally() {
            return Title::None;
        }
        let printed = v.printed_title();
        if let Some((_, t)) = self.title_card_on(m) {
            return t;
        }
        match v.granted_title {
            Some(t) => t,
            None if !v.title_lost => printed,
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
        let Some(sect) = self.sect_of(m) else {
            return Title::None;
        };
        let fits = match t {
            Title::None => true,
            Title::Primogen | Title::Prince(_) | Title::Justicar(_) | Title::InnerCircle(_) => {
                sect == Sect::Camarilla
            }
            Title::Baron(_) => sect == Sect::Anarch,
            Title::Archbishop(_)
            | Title::Bishop
            | Title::Cardinal
            | Title::Priscus
            | Title::Regent => sect == Sect::Sabbat,
            Title::Independent { .. } => sect == Sect::Independent,
        };
        if fits {
            t
        } else {
            Title::None
        }
    }

    /// Does `m` follow `path` — a Sabbat vampire following it (the path's
    /// benefits only while Sabbat: Black Chantry, 2025-06-12; read live)?
    pub fn follows(&self, m: MinionId, path: &str) -> bool {
        let v = &self.minions[m.index()];
        v.present() && self.sect_of(m) == Some(Sect::Sabbat) && v.path() == Some(path)
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

    /// FNV-1a over every field; used for replay equivalence.
    pub fn hash(&self) -> u64 {
        let mut h = Fnv(0xcbf2_9ce4_8422_2325);
        std::hash::Hash::hash(self, &mut h);
        h.0
    }
}

/// FNV-1a: a stable hash, the same across runs and builds.
struct Fnv(u64);

impl std::hash::Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

fn holding_matches(card: u16, h: Holding) -> bool {
    match h {
        Holding::Label(l) => LIBRARY[card as usize].labels.contains(&l),
        Holding::Named(n) => LIBRARY[card as usize].name == n,
    }
}
