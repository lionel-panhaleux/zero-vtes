use crate::cards::{self, AbilitySource, Holding, Layer, Modifier, Requirement, Scope, Stat};
use crate::copies::{CopyCounter, CopyId, Pile, Untracked};
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

/// The `turn_uses` key of a once-each-turn printed cost reduction
/// (Nergal), spent by the first blood-costing card he pays for.
pub fn once_cost_off_key(m: MinionId) -> (crate::cards::AbilitySource, u8) {
    (crate::cards::AbilitySource::Vampire(m), u8::MAX)
}

/// The `turn_uses` key of a once-each-turn redraw (Blackhorse Tanner).
pub fn once_redraw_key(m: MinionId) -> (crate::cards::AbilitySource, u8) {
    (crate::cards::AbilitySource::Vampire(m), u8::MAX - 2)
}

/// The `turn_uses` key of a once-each-turn printed unlock (Baba Yaga).
pub fn once_unlock_key(m: MinionId) -> (crate::cards::AbilitySource, u8) {
    (crate::cards::AbilitySource::Vampire(m), u8::MAX - 3)
}

/// The `turn_uses` key of a seat-wide once-each-turn cost reduction
/// (Baldesar Rossellini), apart from Nergal's.
pub fn once_seat_cost_off_key(m: MinionId) -> (crate::cards::AbilitySource, u8) {
    (crate::cards::AbilitySource::Vampire(m), u8::MAX - 5)
}

/// The `turn_uses` key of a once-each-turn printed hand size (Sascha
/// Vykos).
pub fn once_hand_size_key(m: MinionId) -> (crate::cards::AbilitySource, u8) {
    (crate::cards::AbilitySource::Vampire(m), u8::MAX - 4)
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
/// Sects"): one city's prince, baron and archbishop contest each other; a
/// clan's justicar and Inner Circle member do not; primogen is not unique.
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
        // Nor is the Laibon magaji (rulebook "Laibon").
        Title::Bishop | Title::Cardinal | Title::Priscus | Title::Magaji | Title::Votes(_) => None,
        Title::Independent { name, .. } | Title::Laibon { name, .. } => Some(TitleKey::Named(name)),
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
        Title::Magaji => 2,
        // A priscus votes in the prisci block (its ballot), not with votes.
        Title::Priscus => 0,
        Title::Independent { votes, .. } | Title::Laibon { votes, .. } | Title::Votes(votes) => {
            votes
        }
    }
}

/// What a vampire's printed modifier reads besides the state
/// (`GameState::printed_mod`): the action and the opposing minion.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrintedCtx {
    pub kind: Option<ActionKind>,
    /// The action is one a card makes a hunt (`ActionCtx::card_hunt`).
    pub card_hunt: bool,
    pub directed: Option<bool>,
    pub opponent: Option<MinionId>,
    /// The card (and mode) the minion plays or puts in play (a printed
    /// cost reduction).
    pub card: Option<(u16, u8)>,
    /// The Methuselah the action is directed at ("+1 bleed against a
    /// Methuselah controlling …", Osvaldo Kühnemann).
    pub target: Option<SeatId>,
    /// Every Methuselah the action is directed at, `target` among them
    /// (Vaticination, Taunt the Caged Beast's second target: Kestrelle
    /// Hayes, PIB 20150526).
    pub targets: crate::ids::SeatSet,
    /// Not the acting minion: an "Action Modifier/Reaction" card is played
    /// as a reaction (PIB 20150924-2).
    pub reacting: bool,
    /// The combat round its strength is read in, 0 out of combat ("on any
    /// round of combat after the first", Don Michael Antonio Giovanni).
    pub round: u8,
    /// The action is granted by the modified vampire's own text (Black
    /// Annis: "other … actions").
    pub own_printed: bool,
    /// The action equips with a weapon card from hand (Adhiambo).
    pub equip_weapon: bool,
    /// An equip action (`ActionCtx::is_equip_action`: Roderick Phillips
    /// March).
    pub equip_action: bool,
    /// The action enters combat with a minion (Adhiambo).
    pub enters_combat: bool,
    /// The action's card puts a vampire in play (João Bilé).
    pub puts_vampire_in_play: bool,
    /// The library card of the card in play the action targets (Bulscu).
    pub target_card: Option<u16>,
    /// The vampire calling the referendum votes are read in (Maldavis).
    pub calling: Option<MinionId>,
}

impl PrintedCtx {
    /// Is the action a hunt (`crate::actions::is_hunt`)?
    pub fn is_hunt(&self) -> bool {
        self.kind
            .is_some_and(|k| crate::actions::is_hunt(k, self.card_hunt))
    }
}

/// What a minion's printed text takes off a card's cost
/// (`GameState::printed_cost_off`): pool, blood, "blood or pool".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CostOff {
    pub pool: u8,
    pub blood: u8,
    pub either: u8,
    /// Blood added instead ("cards that require Sanguinus cost Tom an
    /// additional blood"): a negative `Stat::BloodCostOff`.
    pub blood_extra: u8,
}

impl CostOff {
    /// Take the reductions off a `(pool, blood)` cost, the surcharges on.
    pub fn apply(self, pool: &mut i32, blood: &mut u8) {
        *pool = (*pool - self.pool as i32).max(0);
        *blood = blood.saturating_sub(self.blood) + self.blood_extra;
        crate::plays::reduce(pool, blood, self.either);
    }
}

/// What a minion is. An ally is a library card in play (`id`, its
/// `CardInPlay::body` points back) with its entering version's type line
/// (`body`; rulebook "Ally cards", advanced rules 5.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MinionKind {
    Vampire(u16),
    /// An imbued (CRYPT index): a mortal ally from the crypt, its blood its
    /// life (rulebook "Appendix: Imbued Rules" 1).
    Imbued(u16),
    /// Call the Great Beast become a vampire: 9-capacity, clanless,
    /// Independent, infernal, 4 strength, 3 bleed (`beast_disciplines`).
    GreatBeast {
        card: u16,
        id: CardId,
    },
    /// Unleash Hell's Fury attempting to block: "considered a 9-capacity
    /// infernal vampire with +2 intercept during that attempt", 0 blood.
    Fury {
        card: u16,
        id: CardId,
    },
    Ally {
        card: u16,
        id: CardId,
        body: Body,
    },
    /// A library card in play as a vampire (Childe of the Revolution, Web of
    /// Knives Recruit): only what `born` describes, the card's own name,
    /// type and text inert (advanced rules 1.4, 5.1.3).
    Created {
        card: u16,
        id: CardId,
        /// Its clan; none, a clanless one's copy (Dual Form on the Great
        /// Beast).
        clan: Option<&'static str>,
        sect: Sect,
        born: &'static Born,
    },
    /// "A nameless, textless ghoul ally" (Absimiliard's Army, `source`): no
    /// card in play, no contest (LSJ 20040623); removed as it leaves.
    Token {
        card: u16,
        body: Body,
        source: CardId,
    },
}

/// What a created vampire is beyond its clan and sect, as its card prints
/// it: "a 3-capacity (non-unique) Independent Assamite with Celerity [cel],
/// Obfuscate [obf], Potence [pot], Quietus [qui] and 3 blood who is Blood
/// cursed" (Web of Knives Recruit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Born {
    pub capacity: u8,
    pub disciplines: &'static [&'static str],
    pub blood_cursed: bool,
}

/// A 1-capacity vampire with no Discipline (Childe of the Revolution,
/// Waters of Duat, The Embrace).
pub const NEWBORN: Born = Born {
    capacity: 1,
    disciplines: &[],
    blood_cursed: false,
};

/// The distinct master Discipline cards of a pile (Legion).
pub fn master_disciplines(pile: &[u16]) -> Vec<u16> {
    distinct(pile, |c| {
        let l = &LIBRARY[c as usize];
        l.types.contains(&CardType::Master) && l.labels.contains(&Label::Discipline)
    })
}

/// Raw Recruit's Gargoyles: Fortitude, Potence, Visceratika and flight, by
/// capacity (index capacity - 1).
pub static RECRUITED_GARGOYLES: [Born; 11] = {
    let mut out = [NEWBORN; 11];
    let mut i = 0;
    while i < 11 {
        out[i] = Born {
            capacity: i as u8 + 1,
            disciplines: &["for", "pot", "vis", "FLIGHT"],
            blood_cursed: false,
        };
        i += 1;
    }
    out
};

/// Legion's fledglings: a 1-capacity vampire with 1 level of a
/// Discipline, by `cards::DISCIPLINE_KEYS` index.
pub static FLEDGLINGS: [Born; 34] = {
    let mut out = [NEWBORN; 34];
    let mut i = 0;
    while i < 34 {
        out[i] = Born {
            capacity: 1,
            disciplines: std::slice::from_ref(&FLEDGLING_KEYS[i]),
            blood_cursed: false,
        };
        i += 1;
    }
    out
};

/// `cards::DISCIPLINE_KEYS` as a static, for `FLEDGLINGS` to borrow from.
static FLEDGLING_KEYS: [&str; 34] = crate::cards::DISCIPLINE_KEYS;

/// The fledgling with 1 level of Discipline `tag`; none known, none.
pub fn fledgling(tag: &str) -> &'static Born {
    crate::cards::DISCIPLINE_KEYS
        .iter()
        .position(|k| k.eq_ignore_ascii_case(tag))
        .map_or(&NEWBORN, |i| &FLEDGLINGS[i])
}

impl MinionKind {
    /// The ally's LIBRARY index; `None` for a vampire.
    pub fn ally_card(self) -> Option<u16> {
        match self {
            MinionKind::Ally { card, .. } => Some(card),
            // Textless: no card to read.
            MinionKind::Vampire(_)
            | MinionKind::Imbued(_)
            | MinionKind::Created { .. }
            | MinionKind::Token { .. }
            | MinionKind::GreatBeast { .. }
            | MinionKind::Fury { .. } => None,
        }
    }
}

/// A minion in play: a vampire or an ally.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Minion {
    /// `GameState::entry_clock` as it last entered play, 0 out of play
    /// (Chameleon); a merge or a control change is no entry.
    pub entered_at: u32,
    pub owner: SeatId,
    /// Its card's copy, for a vampire or a token (others:
    /// `GameState::minion_copy`).
    pub copy: Untracked<CopyId>,
    pub kind: MinionKind,
    /// A vampire's blood; an ally's life (advanced rules 5.5.1: an ally
    /// has life, not blood, and never pays a blood cost).
    pub blood: u8,
    pub locked: bool,
    pub in_torpor: bool,
    /// An imbued with no life, in the incapacitated region: controlled,
    /// not ready (rulebook "Appendix: Imbued Rules" 5).
    pub incapacitated: bool,
    /// A conviction played on this imbued this unlock phase (one each,
    /// rulebook "Appendix: Imbued Rules" 2).
    pub conviction_played: bool,
    pub in_play: bool,
    /// Face down and out of play, blood kept (rulebook "Contested Cards").
    pub contested: bool,
    /// Contested yet in play and usable: a crypt card in a two-player game
    /// (`Format::TwoPlayer`), its upkeep due in each unlock phase.
    pub contested_in_play: bool,
    /// Out of play with its cards and counters, remembering its effects
    /// (Descent into Darkness). `in_play` stays true: a site reading
    /// `in_play` must also exclude this, or read `present()`.
    pub out_of_play: bool,
    /// The title it claims is under contest.
    pub title_contested: bool,
    /// A disease counter (Vampiric Disease): one at most.
    pub disease: bool,
    /// The printed title is gone for the game: yielded, stolen or replaced
    /// (rulebook "Vampiric Sects": one title per vampire).
    pub title_lost: bool,
    /// A title gained with no card for it (Valerius's steal; a title card
    /// represents its own, `GameState::claimed_title`).
    pub granted_title: Option<Title>,
    /// The NRA special cap on bleeds (rules-qa 1.4).
    pub bled_this_turn: bool,
    /// Bleeds it may take this turn past the cap (Taking the Skin: Minion).
    pub extra_bleeds: u8,
    /// Performed a political action whose referendum passed this turn
    /// (Antonio Delgado: "he can unlock during the discard phase").
    pub referendum_passed_turn: bool,
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
    /// Actions it performed this turn that reached resolution (NRA,
    /// rules-qa 1.4).
    pub acted: Vec<ActionId>,
    /// Action cards it may perform again this turn, one entry a repeat
    /// (Nocturn [OBT]).
    pub repeats: Vec<u16>,
    /// The minions its two-action ability has had one action at, and the
    /// turn (Quincy, The Trapper).
    pub progress: Vec<(MinionId, u32)>,
    /// Recruited this turn, so cannot act (rulebook "Recruit Ally"; one put
    /// in play otherwise can, advanced rules 5.5.4).
    pub recruited: bool,
    /// Successful recruit ally actions this turn (Spectral Servitor
    /// unlocks its recruiter on the first).
    pub recruits_this_turn: u8,
    /// Card names played under `Limit::BetweenUnlockPhases`.
    pub played_since_unlock: Vec<u16>,
    /// "This ally cannot gain life (instead, any life it would gain goes
    /// to the blood bank)" (Aggressive Corpse, Rotting Behemoth).
    pub no_life_gain: bool,
    /// "Unlock this vampire during the next discard phase" (Feral Hound
    /// [ani]): cleared by that phase.
    pub unlock_at_discard: bool,
    /// As `unlock_at_discard`, if then of this clan (Crusade: Istanbul).
    pub unlock_at_discard_if: Option<&'static str>,
    /// Became Black Hand in play (Mateusz Kazimirski).
    pub gained_black_hand: bool,
    /// Bonuses lasting until an expiry, the seat whose phase ends them
    /// (Dive into Madness [dem], Gear Up).
    pub timed: Vec<(crate::cards::Lasting, Expiry, SeatId)>,
    /// Stealth its next action this turn gets, taken as it is declared
    /// (Tangle Atropos' Hand [TEM]).
    pub next_action_stealth: u8,
    /// The circle New Blood chose as he entered play (`Minion::circle`).
    pub chosen_circle: Option<&'static str>,
    /// Fiendish Tongue: the clan it must still be and the blood it can burn
    /// in the next discard phase to unlock.
    pub discard_unlock: Option<(&'static str, u8)>,
    /// Corruption counters, by the Methuselah whose they are; a banished
    /// vampire keeps them (owner, 2026-09-26: rules-qa 25.1).
    pub corruption: Vec<(SeatId, u8)>,
    /// Graft counters (Dr. Morrow): any of them count once (LSJ 20070306).
    pub grafts: u8,
    /// Committed diablerie: ignores Blood Weakens and The Slow Withering
    /// until a Gehenna event is played.
    pub diablerie_shield: bool,
    /// Its successful bleeds: (turn, the Methuselah bled, the bleed
    /// amount) (Public Enemy, Retribution: "since your last turn").
    pub bleeds: Vec<(u32, SeatId, u8)>,
    /// Made Red List by a card (Public Enemy), for good.
    pub made_red_list: bool,
    /// The game turn of its last diablerie (Carlton Van Wyk).
    pub diablerie_turn: Option<u32>,
    /// Has diablerized an older vampire (Smudge the Ignored).
    pub diablerized_older: bool,
    /// Took a hunt action this turn (Thirst: "who did not hunt during that
    /// minion phase").
    pub hunted: bool,
    /// It announced an action this turn, whatever became of it (Tori
    /// Longwood; "performed" as rules-qa 32.10 reads it).
    pub announced_action: bool,
    /// It cannot act, block or play cards while this vampire is not ready
    /// (Legion's fledgling).
    pub tethered_to: Option<MinionId>,
    /// The clan a created vampire is enslaved to (Raw Recruit's Gargoyle);
    /// a crypt vampire's is `CryptCard::slave`.
    pub slave_of: Option<&'static str>,
    /// Held on temporary control: the Methuselah it returns to at the end
    /// of its controller's turn (Puppet Master [DOM]).
    pub temp_from: Option<SeatId>,
    /// Ends of its controller's turn the temporary control outlasts
    /// ("until the end of your next turn", Ayelea: 1).
    pub temp_skip: u8,
    /// The temporary control ends as its controller's next unlock phase
    /// begins, not at a turn's end (Malkavian Dementia).
    pub temp_until_unlock: bool,
    /// The vampire's owner, when its control changed for good
    /// (Graverobbing): its ash heap is the owner's.
    pub origin: Option<SeatId>,
    /// A sect a card made it (Twilight Camp: "make that vampire Anarch").
    pub sect_override: Option<Sect>,
    /// "Considered the same clan and sect as" a vampire, while the card
    /// stays on it (Derange): over the underlying clan and sect (LSJ
    /// 20100811), set as played, moving with the card (LSJ 20070707).
    pub deranged: Option<(&'static str, Sect, CardId)>,
    /// Taint counters (Barbaro Lucchese): at 2, burned and it becomes
    /// infernal.
    pub taint: u8,
    /// The clan it changed to (Barbaro Lucchese), for good; Derange still
    /// overrides it while on it (advanced rules §5.9).
    pub clan_change: Option<&'static str>,
    /// Pact counters on it (Diabolic Lure).
    pub pact: u8,
    /// Capacity lost for the rest of the game (Dual Form [PRO]; never below
    /// 1, RTR 19990712).
    pub capacity_less: u8,
    /// A copy of a vampire (Dual Form [PRO]): its capacity and Discipline
    /// levels (`cards::DISCIPLINE_KEYS`), read from the original as the
    /// action resolved (ANK 20170226).
    pub copy_of: Option<(u8, [u8; crate::cards::DISCIPLINE_KEYS.len()])>,
    /// The vampire this copy was made of (Dual Form): if either leaves the
    /// ready region, the others are burned (LSJ 20050117).
    pub dual_of: Option<MinionId>,
    /// Made infernal by a card (Diabolic Lure's pact counters).
    pub made_infernal: bool,
    /// Merged (rulebook "Advanced"): the base card under the advanced one,
    /// its text still applying (`cards::vampire_of`).
    pub merged: Option<u16>,
    /// The base card's copy, when merged.
    pub merged_copy: Untracked<CopyId>,
    /// The Great Beast's three Disciplines, superior.
    pub beast_disciplines: [Option<&'static str>; 3],
    /// The Sabbat path it follows when not the one printed on it (Black
    /// Chantry, 2025-06-12: one path at a time): one followed in an
    /// influence phase (`Choice::FollowPath`).
    pub path_override: Option<&'static str>,
    /// "Must hunt this turn" (Childe of the Revolution): as a vampire with
    /// no blood, until it hunts or the turn ends.
    pub must_hunt: bool,
    /// Hostage counters (Carver's Meat Packing and Storage).
    pub hostage: u8,
    /// Torture counters (The Path of Lilith).
    pub torture: u8,
    /// Nightmare counters (Week of Nightmares).
    pub nightmare: u8,
}

/// Identity of an action for the no-repeat-action rule (spec §3.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionId {
    /// An action card played from hand: same printed name.
    Card(u16),
    /// An action granted by a card in play: the same copy's same ability
    /// (Open War: LSJ 20100909).
    InPlay(CardId, u8),
    /// A vampire's printed action, by vampire and ability index: never a
    /// card's or a rulebook action (spec §3.8.1).
    Printed(MinionId, u8),
    /// A rulebook action and its target, recorded only where a card bars
    /// repeating it (Change of Target); otherwise repeatable (LSJ 20060522).
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
    /// May it bleed this turn: under the once-a-turn cap, or with a bleed
    /// past it (rules-qa 1.4; Taking the Skin: Minion)?
    pub fn may_bleed(&self) -> bool {
        !self.bled_this_turn || self.extra_bleeds > 0
    }

    /// A bleed reached resolution: the cap, or a bleed past it, spent.
    pub fn mark_bled(&mut self) {
        if self.bled_this_turn {
            self.extra_bleeds = self.extra_bleeds.saturating_sub(1);
        }
        self.bled_this_turn = true;
    }

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
    fn burn_corruption(&mut self, seat: SeatId, n: u8) {
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
            copy: Untracked(CopyId::NONE),
            kind,
            blood,
            locked: false,
            in_torpor: false,
            incapacitated: false,
            conviction_played: false,
            in_play: true,
            contested: false,
            contested_in_play: false,
            out_of_play: false,
            title_contested: false,
            disease: false,
            title_lost: false,
            granted_title: None,
            bled_this_turn: false,
            extra_bleeds: 0,
            referendum_passed_turn: false,
            political_this_turn: false,
            expelled: false,
            hunting_ground_fed: false,
            skip_unlock: false,
            acted: Vec::new(),
            repeats: Vec::new(),
            progress: Vec::new(),
            recruited: false,
            recruits_this_turn: 0,
            played_since_unlock: Vec::new(),
            unlock_at_discard: false,
            unlock_at_discard_if: None,
            gained_black_hand: false,
            timed: Vec::new(),
            next_action_stealth: 0,
            chosen_circle: None,
            discard_unlock: None,
            corruption: Vec::new(),
            grafts: 0,
            diablerie_shield: false,
            diablerie_turn: None,
            diablerized_older: false,
            bleeds: Vec::new(),
            made_red_list: false,
            hunted: false,
            announced_action: false,
            tethered_to: None,
            slave_of: None,
            temp_from: None,
            temp_until_unlock: false,
            temp_skip: 0,
            origin: None,
            capacity_less: 0,
            copy_of: None,
            dual_of: None,
            pact: 0,
            made_infernal: false,
            taint: 0,
            clan_change: None,
            deranged: None,
            sect_override: None,
            merged: None,
            merged_copy: Untracked(CopyId::NONE),
            beast_disciplines: [None; 3],
            path_override: None,
            no_life_gain: false,
            must_hunt: false,
            hostage: 0,
            torture: 0,
            nightmare: 0,
            entered_at: 0,
        }
    }

    /// The vampire's crypt card; `None` for an ally (no clan, sect,
    /// Disciplines, capacity or title).
    pub fn vampire(&self) -> Option<&'static CryptCard> {
        match self.kind {
            MinionKind::Vampire(c) => Some(&CRYPT[c as usize]),
            MinionKind::Imbued(_)
            | MinionKind::Ally { .. }
            | MinionKind::Created { .. }
            | MinionKind::Token { .. }
            | MinionKind::GreatBeast { .. }
            | MinionKind::Fury { .. } => None,
        }
    }

    /// The vampire's or imbued's CRYPT index (printed text lookup); `None`
    /// for an ally or a created vampire.
    pub fn crypt_card(&self) -> Option<u16> {
        match self.kind {
            MinionKind::Vampire(c) | MinionKind::Imbued(c) => Some(c),
            MinionKind::Ally { .. }
            | MinionKind::Created { .. }
            | MinionKind::Token { .. }
            | MinionKind::GreatBeast { .. }
            | MinionKind::Fury { .. } => None,
        }
    }

    /// An ally: a library card's, a token, or an imbued ("considered a
    /// mortal ally", rulebook "Appendix: Imbued Rules" 1).
    pub fn is_ally(&self) -> bool {
        matches!(
            self.kind,
            MinionKind::Ally { .. } | MinionKind::Token { .. } | MinionKind::Imbued(_)
        )
    }

    /// An imbued (rulebook "Appendix: Imbued Rules").
    pub fn is_imbued(&self) -> bool {
        matches!(self.kind, MinionKind::Imbued(_))
    }

    /// Its blood back to the bank as it leaves play: no loss (rules-qa
    /// 95.1).
    pub fn clear_blood(&mut self) {
        self.blood = 0;
    }

    /// Its blood (life) set to `b`, never less (a recovery, a capacity
    /// filled): no gain noted (`GameState::set_blood` notes one).
    pub fn raise_blood(&mut self, b: u8) {
        debug_assert!(
            b >= self.blood,
            "raise_blood lowering {} to {b}",
            self.blood
        );
        self.blood = b;
    }

    /// A vampire: from the crypt or created from a library card.
    pub fn is_vampire(&self) -> bool {
        !self.is_ally()
    }

    /// The ally's card in play; `None` for a vampire.
    pub fn ally_id(&self) -> Option<CardId> {
        match self.kind {
            MinionKind::Ally { id, .. } => Some(id),
            MinionKind::Vampire(_)
            | MinionKind::Imbued(_)
            | MinionKind::Created { .. }
            | MinionKind::Token { .. }
            | MinionKind::GreatBeast { .. }
            | MinionKind::Fury { .. } => None,
        }
    }

    /// Printed name: the contest identity of a vampire or unique ally (a
    /// created vampire's is for display only, advanced rules 5.1.3).
    pub fn name(&self) -> &'static str {
        match self.kind {
            MinionKind::Vampire(c) | MinionKind::Imbued(c) => CRYPT[c as usize].name,
            MinionKind::Ally { card, .. }
            | MinionKind::Created { card, .. }
            | MinionKind::GreatBeast { card, .. }
            | MinionKind::Fury { card, .. } => LIBRARY[card as usize].name,
            // "Nameless": the name on its card is ignored (LSJ 20040623).
            MinionKind::Token { .. } => "",
        }
    }

    /// A Blood Brother's circle: printed, or chosen (New Blood); `None`, a
    /// circle of one (LSJ 20020609).
    pub fn circle(&self) -> Option<&'static str> {
        match self.kind {
            MinionKind::Vampire(c) => CRYPT[c as usize].circle.or(self.chosen_circle),
            _ => None,
        }
    }

    /// A vampire's clan; `None` for an ally.
    pub fn clan(&self) -> Option<&'static str> {
        if let Some((clan, ..)) = self.deranged.filter(|_| !self.is_ally()) {
            return Some(clan);
        }
        if let Some(clan) = self.clan_change.filter(|_| !self.is_ally()) {
            return Some(clan);
        }
        match self.kind {
            MinionKind::Vampire(c) => Some(CRYPT[c as usize].clan),
            MinionKind::Created { clan, .. } => clan,
            // Clanless; an imbued's creed is `GameState::creed`.
            MinionKind::Imbued(_)
            | MinionKind::Ally { .. }
            | MinionKind::Token { .. }
            | MinionKind::GreatBeast { .. }
            | MinionKind::Fury { .. } => None,
        }
    }

    /// The path the vampire follows, its benefits aside (`GameState::follows`
    /// adds the sect): the one printed on it unless another replaced it.
    pub fn path(&self) -> Option<&'static str> {
        match self.kind {
            MinionKind::Vampire(c) => self.path_override.or(CRYPT[c as usize].path),
            _ if self.is_vampire() => self.path_override,
            _ => None,
        }
    }

    /// A vampire's own sect, before any global effect (`GameState::sect_of`
    /// reads the Falls); `None` for an ally.
    pub fn underlying_sect(&self) -> Option<Sect> {
        if let Some((_, sect, _)) = self.deranged.filter(|_| !self.is_ally()) {
            return Some(sect);
        }
        if self.sect_override.is_some() && !self.is_ally() {
            return self.sect_override;
        }
        match self.kind {
            MinionKind::Vampire(c) => CRYPT[c as usize].sect,
            MinionKind::Created { sect, .. } => Some(sect),
            MinionKind::GreatBeast { .. } => Some(Sect::Independent),
            MinionKind::Ally { .. }
            | MinionKind::Imbued(_)
            | MinionKind::Token { .. }
            | MinionKind::Fury { .. } => None,
        }
    }

    /// A minion's printed Disciplines (lower case inferior, upper case
    /// superior): a crypt vampire's, a created vampire's born ones, an
    /// ally's printed flight (`Label::Flight`); none for other allies.
    pub fn printed_disciplines(&self) -> &'static [&'static str] {
        match self.kind {
            MinionKind::Created { born, .. } => born.disciplines,
            // An ally's printed flight (Herald of Topheth).
            MinionKind::Ally { card, .. }
                if LIBRARY[card as usize]
                    .labels
                    .contains(&crate::library::Label::Flight) =>
            {
                &["FLIGHT"]
            }
            // An imbued's virtues.
            MinionKind::Imbued(c) => CRYPT[c as usize].disciplines,
            _ => self.vampire().map_or(&[], |v| v.disciplines),
        }
    }

    /// A vampire's printed title. Merged: the advanced card's ([MERGED]
    /// first), else the base card's (rulebook "Advanced": precedence).
    pub fn printed_title(&self) -> Title {
        let printed = self.vampire().map_or(Title::None, |v| v.title);
        match (self.merged, self.vampire()) {
            (Some(base), Some(v)) => v.merged_title.unwrap_or(if printed == Title::None {
                CRYPT[base as usize].title
            } else {
                printed
            }),
            _ => printed,
        }
    }

    /// A vampire's capacity before cards in play (`GameState::capacity_of`);
    /// `None` for an ally (life uncapped, advanced rules 5.5.2).
    pub fn printed_capacity(&self) -> Option<u8> {
        if let Some((cap, _)) = self.copy_of {
            return Some(cap.saturating_sub(self.capacity_less).max(1));
        }
        let printed = match self.kind {
            MinionKind::Vampire(c) => Some(CRYPT[c as usize].capacity),
            MinionKind::Created { born, .. } => Some(born.capacity),
            MinionKind::GreatBeast { .. } | MinionKind::Fury { .. } => Some(9),
            MinionKind::Ally { .. } | MinionKind::Token { .. } | MinionKind::Imbued(_) => None,
        };
        printed.map(|c| c.saturating_sub(self.capacity_less).max(1))
    }

    /// In play and not contested.
    pub fn present(&self) -> bool {
        self.in_play && !self.contested && !self.out_of_play
    }

    /// Ready: present, not in torpor nor incapacitated (locked or not).
    pub fn ready(&self) -> bool {
        self.present() && !self.in_torpor && !self.incapacitated
    }
}

/// A library card in play (a master on a vampire, a location, …).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CardInPlay {
    pub card: u16,
    /// Its copy (`copies`): the card that entered play.
    pub copy: Untracked<CopyId>,
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
    /// The mode (version) it entered play with (the level used to employ or
    /// recruit it decides, RTR 19941109).
    pub mode: u8,
    /// The minion that played it, when its text names it ("this acting
    /// vampire", Slaughtering the Herd [DOM]).
    pub by: Option<MinionId>,
    pub in_play: bool,
    /// Contested: out of play, its layers suspended (rulebook "Contested
    /// Cards"; spec §1.13.3).
    pub contested: bool,
    /// Out of play with the minion it is on (Descent into Darkness, a
    /// contest: `Engine::cards_out_with`).
    pub out_of_play: bool,
    pub locked: bool,
    pub counters: u8,
    /// Library cards stored on it, top last (Black Market Cache, Shilmulo
    /// Tarot): burned with the card (LSJ 20021128).
    pub stored: Pile,
    /// The lasting effect of a card in the ash heap (Touch of Oblivion,
    /// Sculpt the Flesh [PRO]): burning it moves no card (owner, 2026-09-26:
    /// rules-qa 21.1).
    pub effect_only: bool,
    /// The Methuselahs its action was directed at (Spying Mission [OBF],
    /// Shadow Cast [OBL]); kept when one is ousted.
    pub against: crate::ids::SeatSet,
    /// The game turn it entered play ("during your next minion phase",
    /// Puppet Master [DOM]).
    pub entered: u32,
    /// `GameState::action_serial` as it entered play: "during any action
    /// after this one" (Under My Skin).
    pub entered_action: u32,
    /// Removed from the game whenever it would burn: played from The
    /// Erciyes Fragments.
    pub removed_when_burned: bool,
    /// The owner of the card stored on it, if not its controller's (The
    /// Erciyes Fragments: the prey's card).
    pub stored_owner: Option<SeatId>,
    /// A vampire held on it, face up and out of play, with its owner (Raw
    /// Recruit).
    pub held_vampire: Option<(u16, Untracked<CopyId>, SeatId)>,
    /// A number the card wrote as it was played, fixed (Leadership
    /// Vacuum's X; advanced rules §2.5.3).
    pub fixed_x: u8,
    /// The Discipline it was played for (Sanguine Instruction:
    /// `Modifier::ChosenDisciplineLevel`).
    pub discipline: Option<&'static str>,
    /// The vampires it chose, kept through their leaving play and coming
    /// back (advanced rules 6.4.3; The Rack, Toreador Grand Ball).
    pub chosen: Vec<MinionId>,
    /// In the uncontrolled region with training counters (Web of Knives
    /// Recruit), out of play.
    pub training: bool,
    /// The turn its once-each-turn effect was spent (Harvest Rites), 0 if
    /// never.
    pub spent_turn: u32,
}

impl CardInPlay {
    /// Test fixture: a card with no copy entering play (`entering`).
    #[cfg(feature = "fixtures")]
    pub fn new(card: u16, owner: SeatId, host: Option<MinionId>, mode: u8) -> Self {
        Self::entering(card, CopyId::NONE, owner, host, mode)
    }

    /// A library card (`copy`) entering play under `owner`'s control, on
    /// `host` if any, in the version of `mode`.
    pub fn entering(
        card: u16,
        copy: CopyId,
        owner: SeatId,
        host: Option<MinionId>,
        mode: u8,
    ) -> Self {
        CardInPlay {
            card,
            copy: Untracked(copy),
            owner,
            controller: owner,
            host,
            body: None,
            mode,
            by: None,
            in_play: true,
            out_of_play: false,
            contested: false,
            locked: false,
            counters: 0,
            stored: Pile::new(),
            effect_only: false,
            against: Default::default(),
            entered: 0,
            entered_action: 0,
            removed_when_burned: false,
            stored_owner: None,
            held_vampire: None,
            fixed_x: 0,
            spent_turn: 0,
            discipline: None,
            chosen: Vec::new(),
            training: false,
        }
    }

    /// In play and controlled: not contested.
    pub fn present(&self) -> bool {
        self.in_play && !self.contested && !self.out_of_play
    }

    /// The minion whose card text this is, for its `Actor::Host`
    /// abilities: the minion it is on, or the ally it is.
    pub fn bearer(&self) -> Option<MinionId> {
        self.host.or(self.body)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CardId(pub u32);

/// The virtual-hand source of a combat card played from the ash heap as if
/// from hand (Spiritual Guidance): no card in play.
pub const ASH_PLAY: CardId = CardId(u32::MAX);

/// The virtual-hand source of a conviction played from the ash heap in its
/// player's unlock phase (rulebook "Appendix: Imbued Rules" 2): not removed
/// when burned.
pub const ASH_UNLOCK: CardId = CardId(u32::MAX - 1);

/// A crypt card that is a vampire, not an imbued (rulebook "Appendix:
/// Imbued Rules", card rulings: a card naming a vampire seen).
pub fn vampire_card(c: u16) -> bool {
    !CRYPT[c as usize].imbued
}

/// A virtual-hand source in the ash heap, no card in play.
pub fn from_ash(from: CardId) -> bool {
    from == ASH_PLAY || from == ASH_UNLOCK
}

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
    pub copy: Untracked<CopyId>,
    pub counters: u8,
    /// A banished vampire: the arena entry it left, its cards face down on
    /// it (rules-qa 5.7).
    pub banished: Option<Banished>,
}

/// A banished vampire's arena entry and the cards on it as it left.
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

/// When cards held back by a delayed replacement are replaced
/// (`Until::release`, Blood Weakens, The Meddling of Semsith).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Release {
    /// "Do not replace until your next unlock phase" (Old Friends).
    NextUnlockPhase,
    /// "Do not replace until your next discard phase" (Mirror Walk).
    NextDiscardPhase,
    /// Replaced at the end of the current action (Consign to Oblivion
    /// [OBL]).
    EndOfAction,
    /// "Do not replace until after combat" (Lam Into): as the combat ends.
    AfterCombat,
    /// Replaced at the end of the action or as the hand empties (Blood
    /// Weakens).
    Weakens,
    /// "Until a Methuselah is ousted" (Absimiliard's Army).
    MethuselahOusted,
    /// "Until a vampire commits diablerie" (Blood Weakens).
    Diablerie,
    /// "Until a titled vampire goes to torpor" (The New Inquisition).
    TitledToTorpor,
    /// "Until a vampire successfully hunts" (Restricted Vitae).
    SuccessfulHunt,
    /// "Until a vampire moves from torpor to the ready region" (Torpid
    /// Blood).
    LeavesTorpor,
    /// "Until a vampire becomes a liaison" (Rise of the Nephtali).
    Liaison,
    /// Discarded to Nightmares upon Nightmares: replaced as the
    /// Methuselah's master phase begins.
    MasterPhase,
    /// Not replaced during a minion phase (The Meddling of Semsith):
    /// replaced as it ends.
    MinionPhaseEnd,
}

impl Release {
    const COUNT: usize = Release::MinionPhaseEnd as usize + 1;
}

/// Cards played but not replaced yet, by the moment their replacement is
/// drawn (spec §1.9.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Held([u8; Release::COUNT]);

/// The counters' bytes in order, as a struct of `u8` fields hashed: the
/// fuzz goldens' state hashes.
impl std::hash::Hash for Held {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        for &n in &self.0 {
            h.write_u8(n);
        }
    }
}

impl std::ops::Index<Release> for Held {
    type Output = u8;
    fn index(&self, r: Release) -> &u8 {
        &self.0[r as usize]
    }
}

impl std::ops::IndexMut<Release> for Held {
    fn index_mut(&mut self, r: Release) -> &mut u8 {
        &mut self.0[r as usize]
    }
}

impl Held {
    /// Add what `after` holds over `before`, moment by moment (Learjet's
    /// `Seat::learjet_held`).
    pub fn add_diff(&mut self, before: Held, after: Held) {
        for (i, n) in self.0.iter_mut().enumerate() {
            *n += after.0[i].saturating_sub(before.0[i]);
        }
    }

    pub fn total(self) -> usize {
        self.0.iter().map(|&n| n as usize).sum()
    }
}

/// A seat's number that a timed modifier raises or lowers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimedStat {
    /// Hand size, over or under the rulebook's 7 (Dreams of the Sphinx,
    /// Fotini Katsikaris, Rage of Apedemak).
    HandSize,
    /// Stealth of every vampire's hunt actions (Sunset Strip, Hollywood:
    /// this Methuselah's locks of it).
    HuntStealth,
    /// Cards drawn besides each replacement, then discarded down to hand
    /// size (Infernal Pursuit [CEL]).
    ExtraDraws,
}

/// When a timed modifier ends (`Timed::expire`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Expiry {
    /// The end of their turn (the end of their discard phase).
    EndOfTurn,
    /// As their next discard phase begins.
    DiscardPhase,
    /// Their next unlock phase.
    UnlockPhase,
    /// As their next master phase begins.
    MasterPhase,
    /// The end of the combat under way.
    Combat,
    /// The end of the combat round under way (Patagia's flight).
    Round,
    /// The end of the action under way, its combats included (Alvusia's
    /// strength).
    Action,
    /// The rest of the game.
    Never,
}

/// A seat's timed modifiers: the amount added to each stat until each
/// expiry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Timed([[i8; Expiry::Never as usize + 1]; TimedStat::ExtraDraws as usize + 1]);

impl Timed {
    pub fn add(&mut self, stat: TimedStat, until: Expiry, n: i8) {
        self.0[stat as usize][until as usize] += n;
    }

    /// What `stat` gets until `until`.
    pub fn of(&self, stat: TimedStat, until: Expiry) -> i8 {
        self.0[stat as usize][until as usize]
    }

    /// What `stat` gets, every expiry together.
    pub fn total(&self, stat: TimedStat) -> i32 {
        self.0[stat as usize].iter().map(|&n| n as i32).sum()
    }

    /// The modifiers lasting until `until` end.
    pub fn expire(&mut self, until: Expiry) {
        for row in &mut self.0 {
            row[until as usize] = 0;
        }
    }
}

/// The Sabbat paths (Black Chantry, "New Sabbat rules, terms and errata",
/// 2025-06-12), as the crypt names them.
pub const SABBAT_PATHS: [&str; 4] = [
    "Caine",
    "Cathari",
    "Death and the Soul",
    "Power and the Inner Voice",
];

/// A withdrawal from the game (rulebook "Withdrawing from the Game"):
/// announced in an unlock phase, broken by the Methuselah's pool or a
/// vampire's blood lost or spent, or one of their minions entering combat,
/// before their next unlock phase, which it ends with them out of the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Withdrawal {
    #[default]
    None,
    Announced,
    Broken,
}

impl Withdrawal {
    /// A condition failed: a gain later does not repair it (ANK 20180813).
    pub fn r#break(&mut self) {
        if *self == Withdrawal::Announced {
            *self = Withdrawal::Broken;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Seat {
    pub pool: i32,
    pub withdrawal: Withdrawal,
    /// Out of the game by a withdrawal (`ousted` too: out of the game).
    pub withdrew: bool,
    /// This turn began with an exhausted library and less than a full hand:
    /// a withdrawal may be announced (rulebook "Withdrawing from the Game").
    pub began_short: bool,
    pub ousted: bool,
    /// Victory points in half-point units (tournament rules 3.7: 0.5 VP for
    /// surviving at the time limit).
    pub half_vps: u32,
    pub has_edge: bool,
    /// Web of Knives Recruits whose training counter burned this unlock
    /// phase ("no more than two … each unlock phase").
    pub training_burned: u8,
    /// Remaining crypt cards (CRYPT indices), top at the end.
    pub crypt: Pile,
    pub uncontrolled: Vec<UncontrolledSlot>,
    /// Library (LIBRARY indices), top at the end; hand; ash heap.
    pub library: Pile,
    pub hand: Pile,
    pub ash: Pile,
    /// Cards of this Methuselah removed from the game (Rotting Behemoth;
    /// public, spec §1.10).
    pub removed: Pile,
    /// Burned vampires (CRYPT indices): the ash heap's crypt cards.
    pub ash_crypt: Pile,
    /// Vampires removed from the game (CRYPT indices).
    pub removed_crypt: Pile,
    /// Master phase actions left in the current master phase (Anson's
    /// second one aside: `GameState::mpa_left`).
    pub mpa: u8,
    /// Anson's second master phase action was spent this master phase.
    pub second_mpa_spent: bool,
    /// "+n … until …": hand size, hunt stealth, extra draws.
    pub timed: Timed,
    /// Spiritual Guidance resolved: their next decision may play a combat
    /// card from the ash heap (`ASH_PLAY` in the virtual hand).
    pub ash_combat_play: bool,
    /// Transfers added at the start of their next influence phase (The
    /// Capuchin burned).
    pub transfers_next: u8,
    /// Plays with an open hand for the combat under way (Aura Reading
    /// [aus], played by the opposing minion).
    pub combat_open_hand: bool,
    /// One additional master phase action from trifles per master phase.
    pub trifle_used: bool,
    /// Día de los Muertos this turn: the first referendum a Sabbat vampire
    /// of theirs calls passes automatically.
    pub auto_pass_pending: bool,
    /// Cryptic Rider: the next referendum a vampire of theirs calls this
    /// turn passes automatically.
    pub auto_pass_next: bool,
    /// Malkavian Rider Clause: the next referendum a vampire of theirs calls
    /// passes automatically, whenever it comes.
    pub auto_pass_lasting: bool,
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
    /// One of its minions announced an action this turn, whatever became
    /// of it (NSA Trio).
    pub acted_this_turn: bool,
    /// Discard phase actions left in the current discard phase.
    pub dpa: u8,
    /// Of `dpa`, those granted by Powerbase: Los Angeles and not used yet.
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
    /// `GameState::entry_clock` as this Methuselah's last minion phase
    /// ended (Chameleon).
    pub minion_phase_end: u32,
    /// No transfers in this Methuselah's next influence phase (Leandro).
    pub no_transfers_next: bool,
    /// `GameState::entry_clock` as this Methuselah's last influence phase
    /// ended (Legendary Vampire).
    pub influence_end: u32,
    /// Cards played but not replaced yet, counted against the hand size
    /// (rulebook "Drawing Cards"; spec §6.8.1).
    pub held: Held,
    /// Of `held`, the cards Learjet's bearer played as the acting minion:
    /// an extra draw each as it comes due (PIB 20110718).
    pub learjet_held: Held,
    /// "Do not replace as long as this card is in play" (The Uncoiling):
    /// replaced as it leaves play, even burned (LSJ 20080805).
    pub held_in_play: Vec<CardId>,
    /// An event card was put in play this discard phase (rulebook: no more
    /// than one per phase).
    pub event_played: bool,
    /// Burned a vampire of capacity 6 or more to Recalled to the Founder:
    /// ignores it until the end of the game.
    pub recalled_ignored: bool,
    /// Cards whose mandatory discard-phase effect resolved this phase (The
    /// Uncoiling); one played during the phase is due too (rules-qa 40.5,
    /// 57.5).
    pub discard_resolved: Vec<CardId>,
    /// Cards whose mandatory master-phase effect resolved this phase
    /// (Brujah Debate); one played during the phase is due too (rules-qa
    /// 33.1).
    pub master_resolved: Vec<CardId>,
    /// Political actions the seat's minions cannot perform again this
    /// turn (Delaying Tactics; LSJ 20020201).
    pub forbidden_political: Vec<ActionId>,
}

impl Seat {
    /// Lose or spend `n` pool (a negative `n` gains), the one way pool goes
    /// down (`tests/lint.rs`): it breaks a withdrawal.
    pub fn lose_pool(&mut self, n: i32) {
        self.pool -= n;
        if n > 0 {
            self.withdrawal.r#break();
        }
    }

    /// Pool set to `v`: a decrease is a loss (`lose_pool`).
    pub fn set_pool(&mut self, v: i32) {
        self.lose_pool(self.pool - v);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct GameState {
    /// Dual Form pairs leaving the ready region now: their partners burn,
    /// not they again (LSJ 20050117). A recursion guard, empty between
    /// decisions.
    pub linked_leaving: Vec<MinionId>,
    pub seats: Vec<Seat>,
    /// The next copy number (`copies`): the dealt cards took the first.
    pub copy_counter: Untracked<CopyCounter>,
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
    /// Actions declared in the game so far: the current one's number.
    pub action_serial: u32,
    /// Library cards taken from another's ash heap (The Erciyes Fragments):
    /// zones do not track owners, so the fuzz conservation check reads it.
    pub taken_across: u32,
    /// A superior Veil the Legions was played this turn, by anyone ("only
    /// one … at superior each turn"; owner, 2026-09-26: rules-qa 23.6).
    pub veil_superior: bool,
    /// Actions announced in the current minion phase (Channel 10); one
    /// canceled as its card was played never was (LSJ 20100505).
    pub phase_actions: u8,
    /// The active Methuselah's minion phase is under way, whatever the flow
    /// handles meanwhile (The Meddling of Semsith).
    pub in_minion_phase: bool,
    /// Ticks with each minion entering play and each minion phase ending,
    /// ordering the two (Chameleon).
    pub entry_clock: u32,
    /// A non-mandatory action was performed this turn (Khabar: Glory).
    pub optional_action: bool,
    /// Cards played this game that "can be played only once in a game"
    /// (Week of Nightmares), by LIBRARY index.
    pub once_played: Vec<u16>,
    /// The Methuselahs whose ready titled vampire left the ready region,
    /// with its votes then (Leadership Vacuum's play and X), this turn.
    pub titled_left: Vec<(SeatId, u8)>,
    /// Cards each Methuselah played that one can play only once in a game
    /// (Retribution).
    pub once_played_by: Vec<(SeatId, u16)>,
    /// Cards played this turn that "can be played only once each turn"
    /// (Ashur Tablets), by LIBRARY index; cleared as each turn begins.
    pub turn_played: Vec<u16>,
    /// The cards that cannot be played again this turn (Dark Influences:
    /// "That card cannot be played again this turn"), by LIBRARY index.
    pub barred_this_turn: Vec<u16>,
    /// Cards each minion played this turn under `Limit::PerTurn` (Forced
    /// March).
    pub turn_plays: Vec<(MinionId, u16)>,
    /// The active Methuselah's unlock phase is under way (its convictions
    /// played from the ash heap, `Engine::virtual_hand`).
    pub unlock_phase: bool,
    /// Gestalt [SAN]: (the reacting vampire, a chosen Blood Brother whose
    /// Disciplines it plays cards with), until the action ends.
    pub lent_disciplines: Vec<(MinionId, MinionId)>,
    /// "+X capacity during this action" (Coagulated Entity): the acting
    /// vampire's, until the action ends.
    pub action_capacity: Option<(MinionId, u8)>,
    /// The blood gains since the last look (`GameState::set_blood`,
    /// `Engine::note_blood_gains`).
    pub gains_pending: Vec<(MinionId, u8)>,
    /// Blood gains still usable (Slake the Thirst), with the window of the
    /// first real decision after each: kept while decisions stay in it.
    pub blood_gains: Vec<(MinionId, u8, Option<crate::decision::WindowLabel>)>,
    /// Continuous effects (DESIGN §4.5 scope layers).
    pub layers: Vec<Layer>,
    pub active: SeatId,
    /// Game turn counter, 1-based once the first turn starts.
    pub turn: u32,
    /// The turn of the last combat between an acting vampire and the
    /// vampire that blocked it (NSA Trio: "the first … each turn").
    pub vampire_block_combat_turn: Option<u32>,
    /// The minions in the combat under way, if any (the acting side
    /// first): "while the employer is in combat" (Raptor [ANI]).
    pub combatants: Option<[MinionId; 2]>,
    /// Combatants that cannot use equipment this combat (Drawing Out the
    /// Beast, a block's rider): the Disciplines their equipment gives are
    /// not theirs meanwhile (RTR 20010711).
    pub equipment_barred: Vec<MinionId>,
    /// Hands looked at, (viewer, seat), until the current action ends
    /// (SchreckNET: "look at any Methuselah's hand").
    pub peeks: Vec<(SeatId, SeatId)>,
    /// Donatello Giovanni's marks: (the chosen vampire, Donatello), until
    /// his controller's next unlock phase.
    pub bleed_tolls: Vec<(MinionId, MinionId)>,
    /// Red List minions marked this turn: (the marking Methuselah, the
    /// minion).
    pub red_list_marks: Vec<(SeatId, MinionId)>,
    /// Vampires chosen this turn by a vampire's ability: (the Methuselah
    /// using it, the chooser, the chosen) (Lucinde, Alastor G7).
    pub chosen_this_turn: Vec<(SeatId, MinionId, MinionId)>,
    /// The Methuselahs who burned corruption counters during the action
    /// under way, whoever's (Rise of the Fallen, rules-qa 77.28).
    pub corruption_burned: Vec<SeatId>,
    /// Cards in a hand not their owner's, by copy, with the owner (a card
    /// taken from a stolen Storage Annex): out of the hand, routed back to
    /// the owner (`settle_foreign_cards`; LSJ 20001031, PIB 20121220; owner,
    /// 2026-10-25: keyed by copy, rules-qa 96.2).
    pub foreign: Vec<(crate::copies::Untracked<CopyId>, SeatId)>,
    /// Minions woken in an action card's as-played window, before its
    /// action is declared: woken for it once declared, dropped if it is not
    /// (LSJ 20021011, rules-qa 2.4, 96.4).
    pub pre_woken: Vec<MinionId>,
    /// The minion performing the action under way (Midget: "while Midget is
    /// acting").
    pub acting: Option<MinionId>,
    /// The rules the game is played under, set as it is dealt.
    pub format: Format,
}

/// The rules a game is played under.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Format {
    /// The rulebook's, for 3 to 6 Methuselahs.
    #[default]
    Standard,
    /// VEKN's two-player variant ("Two-player variant for Vampire: The
    /// Eternal Struggle", vekn.net, 2025): a contested crypt card stays in
    /// play, its upkeep 1 pool in each unlock phase; two contested copies'
    /// title is not contested; 3 transfers in the first player's first
    /// influence phase, then 4.
    TwoPlayer,
}

impl GameState {
    /// The game win (tournament rules 3.7; rules-qa 6.9): 2+ VP and strictly
    /// more than every other, ousted or not; `None` on a draw.
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

    /// `s`'s prey with `seated` counted as in the game, ousted or not.
    pub fn prey_counting(&self, s: SeatId, seated: SeatId) -> SeatId {
        let n = self.seats.len();
        let mut i = (s.index() + 1) % n;
        while self.seats[i].ousted && i != seated.index() && i != s.index() {
            i = (i + 1) % n;
        }
        SeatId(i as u8)
    }

    pub fn predator(&self, s: SeatId) -> SeatId {
        self.predator_keeping(s, s)
    }

    /// `s`'s predator, `keep` counted in the game though ousted.
    pub fn predator_keeping(&self, s: SeatId, keep: SeatId) -> SeatId {
        let n = self.seats.len();
        let mut i = (s.index() + n - 1) % n;
        while self.seats[i].ousted && i != s.index() && i != keep.index() {
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
                !m.locked && !m.in_torpor && !m.incapacitated
            })
            .collect()
    }

    /// The conviction cards on imbued `m` (rulebook "Appendix: Imbued
    /// Rules" 2).
    pub fn convictions(&self, m: MinionId) -> Vec<CardId> {
        self.cards
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.present()
                    && c.host == Some(m)
                    && LIBRARY[c.card as usize]
                        .types
                        .contains(&CardType::Conviction)
            })
            .map(|(i, _)| CardId(i as u32))
            .collect()
    }

    /// A mortal: an imbued, or an ally whose type line says so (rulebook
    /// "Appendix: Imbued Rules" 4).
    pub fn is_mortal(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.is_imbued()
            || v.kind
                .ally_card()
                .is_some_and(|c| LIBRARY[c as usize].labels.contains(&Label::Mortal))
    }

    /// An ally whose type line has one of `labels`, an imbued Mortal
    /// (rulebook "Appendix: Imbued Rules" 1).
    pub fn ally_of(&self, m: MinionId, labels: &[Label]) -> bool {
        let v = &self.minions[m.index()];
        if v.is_imbued() {
            return labels.contains(&Label::Mortal);
        }
        v.kind.ally_card().is_some_and(|c| {
            labels
                .iter()
                .any(|l| LIBRARY[c as usize].labels.contains(l))
        })
    }

    /// A monster: a vampire, or an ally neither mortal nor animal (rulebook
    /// "Appendix: Imbued Rules" 4).
    pub fn is_monster(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.is_vampire()
            || (!self.is_mortal(m)
                && !v
                    .kind
                    .ally_card()
                    .is_some_and(|c| LIBRARY[c as usize].labels.contains(&Label::Animal)))
    }

    /// An ally's type line, of the version that entered play (`None` for
    /// a vampire).
    pub fn ally_body(&self, m: MinionId) -> Option<Body> {
        match self.minions[m.index()].kind {
            MinionKind::Ally { body, .. } | MinionKind::Token { body, .. } => Some(body),
            // Its cost its starting life; 1 strength, 1 bleed (rulebook
            // "Appendix: Imbued Rules" 1).
            MinionKind::Imbued(c) => Some(Body {
                life: CRYPT[c as usize].capacity,
                strength: 1,
                bleed: 1,
            }),
            MinionKind::Vampire(_)
            | MinionKind::Created { .. }
            | MinionKind::GreatBeast { .. }
            | MinionKind::Fury { .. } => None,
        }
    }

    /// The Discipline each Discipline master card on `m` gives; nothing out
    /// of play (advanced rules 5.1.1).
    fn discipline_grants(&self, m: MinionId) -> impl Iterator<Item = &'static str> + '_ {
        self.layers.iter().filter_map(move |l| match l.modifier {
            Modifier::DisciplineLevel(d)
                if self.layer_active(l) && self.cards[l.source.index()].host == Some(m) =>
            {
                Some(d)
            }
            Modifier::ChosenDisciplineLevel
                if self.layer_active(l) && self.cards[l.source.index()].host == Some(m) =>
            {
                self.cards[l.source.index()].discipline
            }
            _ => None,
        })
    }

    /// A vampire's capacity, read live: printed, +1 per Discipline master
    /// card on it, at least 1 (advanced rules 5.1.1); `None` for an ally.
    pub fn capacity_of(&self, m: MinionId) -> Option<u8> {
        let printed = self.minions[m.index()].printed_capacity()?;
        let raised: u8 = self
            .hosted(m)
            .map(|l| match l.modifier {
                Modifier::HostCapacity(n) => n,
                _ => 0,
            })
            .sum();
        let cap = (printed + self.discipline_grants(m).count() as u8 + raised).max(1)
            + self
                .action_capacity
                .filter(|&(v, _)| v == m)
                .map_or(0, |(_, n)| n);
        // Wormwood ceils the capacity as other effects changed it (LSJ
        // 20051205); not Karen's (a Gehenna event).
        Some(match self.capacity_ceiling() {
            Some(x) if !self.gehenna_immune(m) => cap.min(x),
            _ => cap,
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

    /// Torpid Blood's +1: on leaving torpor (`rescuer` none) and on rescuing
    /// an older vampire.
    pub fn torpid_surcharge(&self, rescuer: Option<MinionId>, target: MinionId) -> u8 {
        if self.global(Modifier::TorpidBlood).is_none() {
            return 0;
        }
        // Not on Karen's own action, nor on her rescue of another (a
        // Gehenna event does not affect her).
        match rescuer {
            None => u8::from(!self.gehenna_immune(target)),
            Some(r) => u8::from(self.younger_than(r, target) && !self.gehenna_immune(r)),
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

    /// `m`'s level in Discipline `d` (lower case): its own, or a lender's
    /// for the action (Gestalt [SAN]).
    pub fn discipline_level(&self, m: MinionId, d: &str) -> u8 {
        let own = self.own_discipline_level(m, d);
        self.lent_disciplines
            .iter()
            .filter(|&&(b, _)| b == m)
            .map(|&(_, l)| self.own_discipline_level(l, d))
            .fold(own, u8::max)
    }

    /// `m`'s own level in Discipline `d` (lower case): 0 none, 1 inferior,
    /// 2 superior.
    fn own_discipline_level(&self, m: MinionId, d: &str) -> u8 {
        let copied = self.minions[m.index()].copy_of.map(|(_, levels)| {
            cards::DISCIPLINE_KEYS
                .iter()
                .position(|k| k.eq_ignore_ascii_case(d))
                .map_or(0, |i| levels[i])
        });
        let printed = copied.unwrap_or_else(|| self.own_printed_level(m, d));
        if printed == 2 {
            return 2;
        }
        self.granted_level(m, d, printed)
    }

    /// `m`'s printed level in Discipline `d`.
    fn own_printed_level(&self, m: MinionId, d: &str) -> u8 {
        self.minions[m.index()]
            .printed_disciplines()
            .iter()
            .find(|p| p.eq_ignore_ascii_case(d))
            .map_or(0, |p| {
                if p.chars().all(|c| c.is_ascii_uppercase()) {
                    2
                } else {
                    1
                }
            })
    }

    /// `m`'s level in Discipline `d` from its `printed` level and what
    /// grants it.
    fn granted_level(&self, m: MinionId, d: &str, printed: u8) -> u8 {
        // The Great Beast's three, superior.
        if self.minions[m.index()]
            .beast_disciplines
            .iter()
            .flatten()
            .any(|g| g.eq_ignore_ascii_case(d))
        {
            return 2;
        }
        if self.minions[m.index()].timed.iter().any(
            |&(l, ..)| matches!(l, crate::cards::Lasting::Superior(g) if g.eq_ignore_ascii_case(d)),
        ) {
            return 2;
        }
        let barred = self.equipment_barred.contains(&m);
        // The Sargon Fragment: superior, on a vampire.
        if self.minions[m.index()].is_vampire()
            && !barred
            && self.layers.iter().any(|l| {
                matches!(l.modifier, Modifier::BearerSuperiorDiscipline(g)
                    if g.eq_ignore_ascii_case(d)
                        && self.layer_active(l)
                        && self.cards[l.source.index()].host == Some(m))
            })
        {
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
                    !barred
                        && self.layer_active(l)
                        && g.eq_ignore_ascii_case(d)
                        && self.cards[l.source.index()].host == Some(m)
                        && self.minions[m.index()].is_vampire()
                }
                _ => false,
            })
            .count() as u8;
        let levels = cards::vampire_of(&self.minions[m.index()]).map_or(0, |v| {
            v.levels
                .iter()
                .filter(|g| g.eq_ignore_ascii_case(d))
                .count()
        }) as u8;
        // Seal of Veddartha: a level a counter, in order.
        let counted = self
            .layers
            .iter()
            .map(|l| match l.modifier {
                Modifier::BearerCounterLevels(order)
                    if !barred
                        && self.layer_active(l)
                        && self.cards[l.source.index()].host == Some(m)
                        && self.minions[m.index()].is_vampire() =>
                {
                    let n = self.cards[l.source.index()].counters as usize;
                    order
                        .iter()
                        .take(n)
                        .filter(|g| g.eq_ignore_ascii_case(d))
                        .count() as u8
                }
                _ => 0,
            })
            .sum::<u8>();
        let granted = granted + equipped + levels + counted;
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

    /// The capacity and clan of `seat`'s uncontrolled vampire `i`; cards on
    /// a banished one give nothing (advanced rules 5.1.1).
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

    /// The printed sect of `seat`'s uncontrolled vampire `i`; none for an
    /// imbued.
    pub fn slot_sect(&self, seat: SeatId, i: usize) -> Option<Sect> {
        let slot = &self.seats[seat.index()].uncontrolled[i];
        match slot.banished {
            Some(ref b) => self.minions[b.minion.index()].underlying_sect(),
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

    /// Add `n` blood to `seat`'s uncontrolled vampire `i`, over capacity too
    /// (owner, 2026-09-26: rules-qa 28.3).
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

    /// Blood or life gained "not to exceed starting life" (Seraphina),
    /// never below what it already has.
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
        self.set_blood(m, b);
    }

    /// Set `m`'s blood (life) to `b`, noting what it gained (Slake the
    /// Thirst's "after this Blood Brother gains blood") or lost.
    pub fn set_blood(&mut self, m: MinionId, b: u8) {
        let old = self.minions[m.index()].blood;
        if b < old {
            self.lose_blood(m, old - b);
            return;
        }
        self.minions[m.index()].blood = b;
        if b > old {
            self.gains_pending.push((m, b - old));
        }
    }

    /// `m` loses or spends `n` blood (an ally or an imbued life), the one way
    /// blood goes down (`tests/lint.rs`): a vampire's breaks its
    /// controller's withdrawal (an ally has life, not blood: advanced rules
    /// 5.5.1).
    pub fn lose_blood(&mut self, m: MinionId, n: u8) {
        let v = &mut self.minions[m.index()];
        v.blood -= n;
        if n > 0 && v.is_vampire() {
            let owner = v.owner;
            self.seats[owner.index()].withdrawal.r#break();
        }
    }

    /// Burn up to `n` blood (life) from `m`.
    pub fn burn_blood(&mut self, m: MinionId, n: u8) {
        self.lose_blood(m, n.min(self.minions[m.index()].blood));
    }

    /// `m` burns up to `n` blood (an ally life); true when an ally in play
    /// is left with none, for the caller to burn (owner, rules-qa 73.5).
    #[must_use]
    pub fn burn_blood_or_life(&mut self, m: MinionId, n: u8) -> bool {
        self.burn_blood(m, n);
        let v = &self.minions[m.index()];
        v.is_ally() && v.blood == 0 && v.in_play
    }

    /// "Burn all torture counters when this card leaves play" (The Path of
    /// Lilith): none left once no such card is in play.
    pub fn release_torture(&mut self) {
        if !self.minions.iter().any(|m| m.torture > 0) {
            return;
        }
        let held = self.cards.iter().any(|c| {
            c.present()
                && cards::lookup(c.card).is_some_and(|d| {
                    d.riders
                        .iter()
                        .any(|r| r.hook == cards::Hook::OwnNonCamarillaSendsToTorpor)
                })
        });
        if !held {
            for m in &mut self.minions {
                m.torture = 0;
            }
        }
    }

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

    /// Blood above capacity drains off every vampire in play (advanced
    /// rules 5.1.1); out of play, a vampire keeps it.
    pub fn drain_all_excess(&mut self) {
        for i in 0..self.minions.len() {
            let m = MinionId(i as u32);
            let v = &self.minions[i];
            // Only cards on it raise its capacity: at or below the printed
            // one, nothing to read — unless Wormwood ceils it.
            let ceiled =
                self.capacity_ceiling().is_some_and(|x| v.blood > x) && !self.gehenna_immune(m);
            if !v.in_play
                || v.out_of_play
                || v.printed_capacity().is_none_or(|p| v.blood <= p) && !ceiled
            {
                continue;
            }
            if let Some(c) = self.capacity_of(m) {
                self.burn_blood(m, self.minions[i].blood.saturating_sub(c));
            }
        }
    }

    /// Bleed amount before modifiers (rulebook "Bleed": all vampires bleed
    /// for 1 by default; an ally for its printed bleed).
    pub fn base_bleed(&self, m: MinionId) -> u8 {
        if matches!(self.minions[m.index()].kind, MinionKind::GreatBeast { .. }) {
            return 3;
        }
        self.ally_body(m).map_or(1, |b| b.bleed)
    }

    /// `m`'s hunt gets n blood from the bank: whether it successfully hunts
    /// (at capacity too, rules-qa 89.26; none with a disease counter, 59.4),
    /// its once-each-turn bonus then spent (`spend_once_hunt`).
    pub fn hunt_blood(&mut self, m: MinionId, n: u8) -> bool {
        if self.minions[m.index()].disease {
            return false;
        }
        self.gain_blood(m, n);
        self.spend_once_hunt(m);
        true
    }

    /// A successful hunt spends `m`'s once-each-turn hunt bonus (Harvest
    /// Rites, at capacity too: owner, rules-qa 89.1).
    pub fn spend_once_hunt(&mut self, m: MinionId) {
        let spent: Vec<CardId> = self
            .borne(m)
            .filter(|l| matches!(l.modifier, Modifier::BearerHuntOncePerTurn(_)))
            .map(|l| l.source)
            .collect();
        for c in spent {
            self.cards[c.index()].spent_turn = self.turn;
        }
    }

    /// Infernal for the unlock phase (rulebook "Infernal"): a vampire printed
    /// or made so (Diabolic Lure), or an ally whose card is (Herald of
    /// Topheth).
    pub fn unlocks_as_infernal(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.made_infernal
            || v.vampire().is_some_and(|c| c.infernal)
            || self.bears(m, Modifier::BearerInfernal)
            || v.kind.ally_card().is_some_and(|c| {
                crate::library::LIBRARY[c as usize]
                    .labels
                    .contains(&crate::library::Label::Infernal)
            })
    }

    /// `m`'s prisci ballots: a priscus's, and printed ones ("Gratiano gets
    /// +1 ballot", his even without the title: LSJ 20040519).
    pub fn ballots(&self, m: MinionId) -> u8 {
        let priscus = u8::from(self.title_of(m) == Title::Priscus);
        priscus
            + self
                .printed_mod(m, cards::Stat::Ballots, PrintedCtx::default())
                .max(0) as u8
    }

    /// The Disciplines `m` has at inferior (Camarilla Vitae Slave's choice).
    pub fn inferior_disciplines(&self, m: MinionId) -> Vec<&'static str> {
        disciplines()
            .into_iter()
            .filter(|d| self.discipline_level(m, d) == 1)
            .collect()
    }

    /// No combat under way: no combatants, and no equipment bar left
    /// (`equipment_barred` lives with its combat).
    pub(crate) fn leave_combat(&mut self) {
        self.combatants = None;
        self.equipment_barred.clear();
        // Every combat end, a Psyche! or Coordinate Attacks combat next
        // included: a round's or a combat's bonus is not carried into it.
        for m in &mut self.minions {
            m.timed
                .retain(|&(_, until, _)| !matches!(until, Expiry::Round | Expiry::Combat));
        }
    }

    /// `m` cannot perform political actions (Bela's printed bar).
    pub fn cannot_act_politically(&self, m: MinionId) -> bool {
        self.printed_mod(m, Stat::NoPoliticalActions, PrintedCtx::default()) > 0
    }

    /// `m` cannot commit diablerie: Blood Cursed, or barred by a card on
    /// it (Alastor).
    pub fn cannot_diablerize(&self, m: MinionId) -> bool {
        self.blood_cursed(m) || self.bears(m, Modifier::HostCannotDiablerize)
    }

    /// "Blood cursed" (rulebook "Traits"): it cannot commit diablerie — nor
    /// be compelled to (advanced rules §3.7.8.4).
    pub fn blood_cursed(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        match v.kind {
            MinionKind::Created { born, .. } => born.blood_cursed,
            _ => v.vampire().is_some_and(|c| c.blood_cursed),
        }
    }

    /// Is `m` an infernal vampire (its card's trait)?
    pub fn is_infernal(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        matches!(
            v.kind,
            MinionKind::GreatBeast { .. } | MinionKind::Fury { .. }
        ) || v.vampire().is_some_and(|v| v.infernal)
            || v.made_infernal
            || self.bears(m, Modifier::BearerInfernal)
    }

    /// Unlock a locked `m`; a disease counter burns 1 blood, else itself
    /// (Vampiric Disease; any unlock, ANK 20221210-2; an unlocked minion or
    /// one entering play is not unlocked, ANK 20170227, LSJ 20060213).
    pub fn unlock(&mut self, m: MinionId) {
        let v = &mut self.minions[m.index()];
        if !v.locked {
            return;
        }
        v.locked = false;
        if v.disease {
            if v.blood > 0 {
                self.lose_blood(m, 1);
            } else {
                v.disease = false;
            }
        }
    }

    /// `m` may follow a Sabbat path: a Sabbat vampire in play following
    /// none, a blood to burn (Black Chantry 2025-06-12; rules-qa 97.1).
    pub fn may_follow_path(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.present()
            && v.is_vampire()
            && v.path().is_none()
            && v.blood > 0
            && self.sect_of(m) == Some(crate::crypt::Sect::Sabbat)
    }

    /// `s` may announce a withdrawal: the turn began with the library
    /// exhausted and less than a full hand, none announced (rulebook
    /// "Withdrawing from the Game").
    pub fn may_announce_withdrawal(&self, s: SeatId) -> bool {
        let seat = &self.seats[s.index()];
        seat.began_short && seat.library.is_empty() && seat.withdrawal == Withdrawal::None
    }

    /// The "current bleed" of `m` outside a bleed action, against its prey,
    /// no action modifier (Justicar Retribution, RTR 19980623).
    pub fn current_bleed(&self, m: MinionId) -> u8 {
        let prey = self.prey(self.minions[m.index()].owner);
        let ctx = PrintedCtx {
            kind: Some(ActionKind::Bleed),
            directed: Some(true),
            target: Some(prey),
            ..PrintedCtx::default()
        };
        (self.base_bleed(m) as i32
            + self.lasting_bleed(m)
            + self.bleed_against_mod(m, prey)
            + self.bearer_mod(m, Stat::Bleed, ActionKind::Bleed)
            + self.printed_mod(m, Stat::Bleed, ctx))
        .max(0) as u8
    }

    /// `m`'s lasting stealth and strength bonuses (`Minion::timed`).
    pub fn lasting(&self, m: MinionId) -> (i32, i32) {
        self.minions[m.index()]
            .timed
            .iter()
            .fold((0, 0), |(st, sg), &(l, ..)| match l {
                crate::cards::Lasting::Stealth(n) => (st + n as i32, sg),
                crate::cards::Lasting::Strength(n) => (st, sg + n as i32),
                crate::cards::Lasting::Superior(_)
                | crate::cards::Lasting::AggravatedHands
                | crate::cards::Lasting::Bleed(_)
                | crate::cards::Lasting::UnlockAtEndOfTurn => (st, sg),
            })
    }

    /// The bleed `m`'s lasting bonuses add (`Minion::timed`).
    pub fn lasting_bleed(&self, m: MinionId) -> i32 {
        self.minions[m.index()]
            .timed
            .iter()
            .map(|&(l, ..)| match l {
                crate::cards::Lasting::Bleed(n) => n as i32,
                _ => 0,
            })
            .sum()
    }

    /// `m` unlocks at the end of the turn (`Lasting::UnlockAtEndOfTurn`).
    pub fn unlock_at_end_of_turn(&mut self, m: MinionId) {
        let seat = self.active;
        let timed = &mut self.minions[m.index()].timed;
        let entry = (
            crate::cards::Lasting::UnlockAtEndOfTurn,
            Expiry::EndOfTurn,
            seat,
        );
        if !timed.contains(&entry) {
            timed.push(entry);
        }
    }

    /// Strength before modifiers (rulebook "Hand Strike": vampires have a
    /// default strength of 1; an ally has its printed strength).
    pub fn base_strength(&self, m: MinionId) -> u8 {
        if matches!(self.minions[m.index()].kind, MinionKind::GreatBeast { .. }) {
            return 4;
        }
        self.ally_body(m).map_or(1, |b| b.strength)
    }

    /// Hand indices of the seat's cards `of` matches.
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
    /// controller's; a minion card on a minion is its host's controller's
    /// (owner, 2026-09-06; spec §1.12.1).
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
    /// no self-contest (rulebook "Contested Cards"; spec §1.13.1; rules-qa
    /// 7.1).
    pub fn controls_unique_copy(&self, seat: SeatId, card: u16) -> bool {
        LIBRARY[card as usize].labels.contains(&Label::Unique)
            && self.cards.iter().enumerate().any(|(i, c)| {
                c.in_play
                    && self.card_controller(CardId(i as u32)) == seat
                    && LIBRARY[c.card as usize].name == LIBRARY[card as usize].name
            })
    }

    /// Printed modifiers to `m`'s `stat` from the vampires in play, its own
    /// and others', read live: in torpor too (LSJ 20011022).
    pub fn printed_mod(&self, m: MinionId, stat: Stat, ctx: PrintedCtx) -> i32 {
        use crate::cards::PrintedTo;
        let owner = self.minions[m.index()].owner;
        let mut total = 0;
        for (i, v) in self.minions.iter().enumerate() {
            let source = MinionId(i as u32);
            let Some(def) =
                cards::vampire_of(v).filter(|_| v.present() && v.crypt_card().is_some())
            else {
                continue;
            };
            for p in def.printed.iter().filter(|p| p.stat == stat) {
                let applies = match p.to {
                    PrintedTo::Itself => source == m,
                    PrintedTo::Opponent => ctx.opponent == Some(source),
                    PrintedTo::OthersControlled => source != m && v.owner == owner,
                    PrintedTo::Others => source != m,
                };
                if applies
                    && p.requires
                        .iter()
                        .all(|&r| self.printed_holds(r, source, m, ctx))
                {
                    total += p.n as i32 * self.printed_count(p.per, source, ctx);
                }
            }
        }
        total
    }

    /// Does `m` (or with `by_methuselah` its Methuselah) meet a clan
    /// requirement to play a card: its clan or a printed "as if" (Abraham
    /// Mellon, Lady Scarlett). Plays only (LSJ 20050119; LSJ 20101013).
    pub fn meets_clan(&self, m: MinionId, clans: &[&str], by_methuselah: bool) -> bool {
        let v = &self.minions[m.index()];
        v.clan().is_some_and(|c| clans.contains(&c))
            || v.present()
                && cards::vampire_of(v)
                    .and_then(|d| d.plays_as)
                    .is_some_and(|p| clans.contains(&p.clan) && (p.methuselah || !by_methuselah))
    }

    /// Is `m` itself of the clan `card` requires (its effects in play; not
    /// one playing as the clan, ANK 20220816-2).
    pub fn of_card_clan(&self, m: MinionId, card: u16) -> bool {
        let clans = LIBRARY[card as usize].clans;
        clans.is_empty()
            || self.minions[m.index()]
                .clan()
                .is_some_and(|c| clans.contains(&c))
    }

    /// What `m`'s printed text takes off the cost of `card` in `mode` that
    /// it plays or puts in play, by another's card too (rules-qa 36.5, ANK
    /// 20210913).
    pub fn printed_cost_off(&self, m: MinionId, card: u16, mode: u8) -> CostOff {
        self.printed_cost_off_x(m, card, mode, 0)
    }

    /// As `printed_cost_off`, an X cost chosen at `x` (The Ankara Citadel
    /// halves it once chosen; owner, 2026-10-14: rules-qa 70.17).
    pub fn printed_cost_off_x(&self, m: MinionId, card: u16, mode: u8, x: u8) -> CostOff {
        let ctx = PrintedCtx {
            card: Some((card, mode)),
            ..PrintedCtx::default()
        };
        let n = |stat| self.printed_mod(m, stat, ctx).max(0) as u8;
        // The Ankara Citadel: halved (rounded down) unless played as an
        // action modifier (owner, 2026-10-14: rules-qa 70.17).
        let lib = &LIBRARY[card as usize];
        let as_modifier = lib.types.contains(&CardType::ActionModifier)
            && (lib.types.len() == 1
                || self.acting == Some(m)
                    && !cards::lookup(card)
                        .and_then(|d| d.modes.get(mode as usize))
                        .is_some_and(|md| md.windows.iter().all(|w| w.is_combat())));
        let base = match lib.blood_cost {
            crate::library::Cost::Fixed(c) => c,
            crate::library::Cost::X => x,
            crate::library::Cost::None => 0,
        };
        let half = if !as_modifier && self.hosts(m, Modifier::HalvesBloodCosts) {
            base - base / 2
        } else {
            0
        };
        // "This card costs Followers of Set 1 fewer blood" (Mental Maze).
        let clan_off = cards::lookup(card)
            .and_then(|d| d.cheaper_for)
            .filter(|(clan, _)| self.minions[m.index()].clan() == Some(*clan))
            .map_or(0, |(_, n)| n);
        // "Combat cards cost this vampire 1 fewer blood" (Eye of
        // Hazimel): on a vampire, its equipment usable.
        let as_combat = lib.types.contains(&CardType::Combat)
            && (lib.types.len() == 1
                || cards::lookup(card)
                    .and_then(|d| d.modes.get(mode as usize))
                    .is_some_and(|md| md.windows.iter().all(|w| w.is_combat())));
        let equip_off: u8 = if as_combat
            && self.minions[m.index()].is_vampire()
            && !self.equipment_barred.contains(&m)
        {
            self.borne(m)
                .map(|l| match l.modifier {
                    Modifier::BearerCombatCostOff(k) => k,
                    _ => 0,
                })
                .sum()
        } else {
            0
        };
        // The Path of Bone: the played mode requiring Necromancy, on a
        // Giovanni (rules-qa 38.4).
        let tags = lib.modes.get(mode as usize).map_or(&[][..], |md| md.tags);
        let clan = self.minions[m.index()].clan();
        let path_off: u8 = self
            .in_play_layers()
            .map(|l| match l.modifier {
                Modifier::ClanDisciplineBloodOff {
                    clan: c,
                    discipline,
                    n,
                } if clan == Some(c) && tags.iter().any(|t| t.eq_ignore_ascii_case(discipline)) => {
                    n
                }
                _ => 0,
            })
            .sum();
        CostOff {
            pool: n(Stat::PoolCostOff),
            blood: n(Stat::BloodCostOff) + half + clan_off + equip_off + path_off,
            either: n(Stat::CostOff),
            blood_extra: (-self.printed_mod(m, Stat::BloodCostOff, ctx)).max(0) as u8,
        }
    }

    /// Is a vampire (not a created one, rules-qa 41.5) of this name ready,
    /// `of` this Methuselah's or any's?
    pub fn ready_named(&self, name: &str, of: Option<SeatId>) -> bool {
        self.minions.iter().any(|o| {
            o.ready() && o.is_vampire() && o.name() == name && of.is_none_or(|s| o.owner == s)
        })
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
            R::ActorGraftedAlly => me.is_ally() && me.grafts > 0,
            // "While Annabelle is locked" (in torpor too, LSJ 20011022).
            R::HostLocked => src.locked,
            R::HostUnlocked => !src.locked,
            R::SourceActing => self.acting == Some(source),
            R::ActorClaimsTitleIn(classes) => {
                let t = self.claimed_title(actor);
                classes.iter().any(|c| c.matches(t))
            }
            // A reaction card, or one with two types played as a reaction
            // (Scout Youngwood; PIB 20150924-2).
            R::PlayedAsReaction => ctx.card.is_some_and(|(c, _)| {
                let types = crate::library::LIBRARY[c as usize].types;
                types.contains(&crate::library::CardType::Reaction)
                    && (!types.contains(&crate::library::CardType::ActionModifier) || ctx.reacting)
            }),
            // A combat card, or one with two types played in a combat
            // window (Swift Cover's combat mode, not its action modifier).
            R::PlayedAsCombat => ctx.card.is_some_and(|(c, mode)| {
                let types = crate::library::LIBRARY[c as usize].types;
                types.contains(&crate::library::CardType::Combat)
                    && (types.len() == 1
                        || cards::lookup(c)
                            .and_then(|d| d.modes.get(mode as usize))
                            .is_some_and(|m| m.windows.iter().all(|w| w.is_combat())))
            }),
            R::PlayedInStrikeMode => ctx
                .card
                .is_some_and(|(c, mode)| crate::plays::is_strike_mode(c, mode)),
            R::DiablerizedOlder => src.diablerized_older,
            R::ActionDirected => ctx.directed == Some(true),
            R::ActionUndirected => ctx.directed == Some(false),
            R::IsBleed => ctx.kind == Some(ActionKind::Bleed),
            R::IsHunt => ctx.is_hunt(),
            R::RulebookHunt => ctx.kind == Some(ActionKind::Hunt),
            R::IsDiablerie => ctx.kind == Some(ActionKind::Diablerie),
            R::IsRecruitOrEmploy => {
                matches!(ctx.kind, Some(ActionKind::Recruit | ActionKind::Employ))
            }
            R::IsEquipWeaponFromHand => ctx.equip_weapon,
            R::IsEquipAction => ctx.equip_action,
            R::ActionEntersCombat => ctx.enters_combat,
            R::ActionNotEnteringCombat => ctx.kind.is_some() && !ctx.enters_combat,
            R::ActionPutsVampireInPlay => ctx.puts_vampire_in_play,
            R::OpponentAlly => opp.is_some_and(|o| o.is_ally()),
            R::OpponentInfernal => ctx.opponent.is_some_and(|o| self.is_infernal(o)),
            R::PreyControlsReadyTitled(classes) => {
                let prey = self.prey(src.owner);
                self.minions_of(prey).any(|m| {
                    self.minions[m.index()].ready()
                        && classes.iter().any(|c| c.matches(self.title_of(m)))
                })
            }
            R::DirectedAtCard(filters) => {
                ctx.directed == Some(true)
                    && ctx
                        .target_card
                        .is_some_and(|c| filters.iter().any(|f| f.matches(c)))
            }
            R::SourceCalledReferendum => ctx.calling == Some(source),
            R::NoEdge => !self.seats[src.owner.index()].has_edge,
            // "While you have the Edge" (Anxo Vilela).
            R::HasEdge => self.seats[src.owner.index()].has_edge,
            R::TargetSeatControlsClan(clans) => ctx.target.is_some_and(|s| {
                self.minions_of(s).any(|m| {
                    self.minions[m.index()]
                        .clan()
                        .is_some_and(|c| clans.contains(&c))
                })
            }),
            // Queen Anne, Cornelius Ottavio: read as the bleed is.
            R::TargetSeatControlsReadyClan { clans, not } => ctx.target.is_some_and(|s| {
                self.minions_of(s).any(|m| {
                    let v = &self.minions[m.index()];
                    v.ready() && v.clan().is_some_and(|c| clans.contains(&c))
                }) != not
            }),
            R::TargetSeatControlsReadySect { sect, not } => ctx.target.is_some_and(|s| {
                self.minions_of(s).any(|m| {
                    let v = &self.minions[m.index()];
                    v.ready() && v.is_vampire() && (self.sect_of(m) == Some(sect)) != not
                })
            }),
            // Any of the Methuselahs it is directed at (PIB 20150526).
            R::TargetSeatControlsNoReadySect(sect) => (0..self.seats.len())
                .map(|i| SeatId(i as u8))
                .filter(|&s| ctx.targets.contains(s) || ctx.target == Some(s))
                .any(|s| {
                    !self.minions_of(s).any(|m| {
                        let v = &self.minions[m.index()];
                        v.ready() && v.is_vampire() && self.sect_of(m) == Some(sect)
                    })
                }),
            R::SourceTitleIn(classes) => {
                let t = self.title_of(source);
                classes.iter().any(|c| c.matches(t))
            }
            R::ActorSectIn(sects) => self.sect_of(actor).is_some_and(|s| sects.contains(&s)),
            R::OpponentSectIn(sects) => ctx
                .opponent
                .and_then(|o| self.sect_of(o))
                .is_some_and(|s| sects.contains(&s)),
            // "Whose controller controls any Gehenna cards" (Céleste
            // Lamontagne): an event or a card in play of theirs labeled so.
            R::OpponentControllerHasGehenna => opp.is_some_and(|o| {
                self.cards.iter().enumerate().any(|(i, c)| {
                    c.present()
                        && self.card_controller(CardId(i as u32)) == o.owner
                        && LIBRARY[c.card as usize].labels.contains(&Label::Gehenna)
                })
            }),
            R::NotFirstRound => ctx.round > 1,
            R::OtherNonHuntAction => ctx.kind.is_some() && !ctx.is_hunt() && !ctx.own_printed,
            R::ReadyNamed(name) => self.ready_named(name, None),
            R::OwnReadyNamed(name) => self.ready_named(name, Some(src.owner)),
            R::IsPolitical => ctx.kind == Some(ActionKind::Political),
            R::OpponentClan(c) => opp.and_then(|o| o.clan()) == Some(c),
            R::OpponentRedList => ctx.opponent.is_some_and(|o| self.is_red_list(o)),
            // "In combat with a werewolf" (Mark Decker): an ally of the
            // subtype.
            R::OpponentSubtype(l) => opp
                .and_then(|o| o.kind.ally_card())
                .is_some_and(|card| LIBRARY[card as usize].labels.contains(&l)),
            R::ActorSubtype(l) => me
                .kind
                .ally_card()
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
            R::ActorVampire => me.is_vampire(),
            R::ActorTitleIn(classes) => classes.iter().any(|c| c.matches(self.title_of(actor))),
            R::ActorUntitled => self.title_of(actor) == crate::crypt::Title::None,
            R::PoolAtMost(n) => self.seats[src.owner.index()].pool <= n as i32,
            R::ControlsLocation | R::ControlsNoLocation => {
                (self.locations_controlled(src.owner) > 0) == (r == R::ControlsLocation)
            }
            R::TargetSeatControlsLocation => {
                ctx.target.is_some_and(|s| self.locations_controlled(s) > 0)
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
            R::NoReadyNamed(name) => !self.ready_named(name, None),
            R::ControlsNoReadyOlderOfClan(clan) => !self.minions_of(src.owner).any(|o| {
                let w = &self.minions[o.index()];
                matches!(w.kind, MinionKind::Vampire(_))
                    && w.ready()
                    && w.clan() == Some(clan)
                    && self.younger_than(source, o)
            }),
            R::ControlsNoReadyOfClan(clan) => !self.minions_of(src.owner).any(|o| {
                let w = &self.minions[o.index()];
                w.is_vampire() && w.ready() && w.clan() == Some(clan)
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
            R::PreyControlsVampireNotOfSect(sect) => {
                let prey = self.prey(src.owner);
                prey != src.owner
                    && self.minions_of(prey).any(|v| {
                        self.minions[v.index()].is_vampire() && self.sect_of(v) != Some(sect)
                    })
            }
            R::ControlsVampireNamed(name) => self
                .minions
                .iter()
                .any(|v| v.owner == src.owner && v.present() && v.name() == name),
            _ => unreachable!("{r:?} is not a printed-modifier predicate"),
        }
    }

    fn printed_count(&self, per: cards::Per, source: MinionId, ctx: PrintedCtx) -> i32 {
        match per {
            cards::Per::One => 1,
            cards::Per::LocationsControlled => {
                self.locations_controlled(self.minions[source.index()].owner) as i32
            }
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
            // In torpor too: controlled.
            cards::Per::TitledOfTarget => ctx.target.map_or(0, |t| {
                self.minions_of(t)
                    .filter(|&m| {
                        self.minions[m.index()].is_vampire()
                            && self.title_of(m) != crate::crypt::Title::None
                    })
                    .count() as i32
            }),
            cards::Per::ReadyTitledOfOthers => {
                let owner = self.minions[source.index()].owner;
                (0..self.minions.len())
                    .map(|i| MinionId(i as u32))
                    .filter(|&m| {
                        let v = &self.minions[m.index()];
                        v.owner != owner
                            && v.ready()
                            && v.is_vampire()
                            && self.title_of(m) != crate::crypt::Title::None
                    })
                    .count() as i32
            }
        }
    }

    /// Master phase actions `s` has left: `Seat::mpa`, and Anson's second
    /// one while unspent, read live (rules-qa 62.5); Rumors of Gehenna's
    /// chosen gets the same second one, not a third ("instead of one").
    pub fn mpa_left(&self, s: SeatId) -> u8 {
        let seat = &self.seats[s.index()];
        let rumors = self.in_play_layers().any(|l| {
            l.modifier == Modifier::TwoMasterPhaseActionsForChosen
                && self.cards[l.source.index()].against.contains(s)
        });
        let two = !seat.second_mpa_spent
            && (rumors
                || self.minions_of(s).any(|m| {
                    self.printed_mod(m, Stat::TwoMasterPhaseActions, PrintedCtx::default()) > 0
                }));
        seat.mpa + u8::from(two)
    }

    /// Stamp the minions that entered play since the last settle, and
    /// clear those out of play (`Minion::entered_at`).
    pub fn settle_entries(&mut self) {
        for i in 0..self.minions.len() {
            let v = &self.minions[i];
            if !v.in_play {
                self.minions[i].entered_at = 0;
            } else if v.entered_at == 0 {
                self.entry_clock += 1;
                self.minions[i].entered_at = self.entry_clock;
            }
        }
    }

    /// The pool a discard phase action to discard a card costs, over its
    /// action: 1 for each Camarilla Threat in play, anyone's.
    pub fn discard_toll(&self) -> u8 {
        self.in_play_layers()
            .map(|l| match l.modifier {
                Modifier::DiscardActionCostsPool(n) => n,
                _ => 0,
            })
            .sum()
    }

    /// `s` is the Methuselah a Meddling of Semsith in play chose: no
    /// replacement during any minion phase.
    pub fn meddled(&self, s: SeatId) -> bool {
        self.in_play_layers().any(|l| {
            l.modifier == Modifier::NoReplaceInMinionPhaseForChosen
                && self.cards[l.source.index()].against.contains(s)
        })
    }

    /// The most pool `s` burns to a bleed (Protected Resources), if capped.
    pub fn bleed_burn_cap(&self, s: SeatId) -> Option<u8> {
        self.controlled_layers(s)
            .filter_map(|l| match l.modifier {
                Modifier::BleedBurnCap(n) => Some(n),
                _ => None,
            })
            .min()
    }

    /// Spend a master phase action of `s`: the ordinary ones first, Anson's
    /// second one last (lost if he leaves the ready region first).
    pub fn spend_mpa(&mut self, s: SeatId) {
        let seat = &mut self.seats[s.index()];
        if seat.mpa > 0 {
            seat.mpa -= 1;
        } else {
            seat.second_mpa_spent = true;
        }
    }

    /// Discard phase actions printed on the seat's vampires (Sreelekha),
    /// read as the phase begins.
    pub fn printed_discard_actions(&self, s: SeatId) -> i32 {
        self.minions_of(s)
            .map(|m| self.printed_mod(m, Stat::DiscardPhaseActions, PrintedCtx::default()))
            .sum()
    }

    /// Hand size (rulebook "Drawing Cards"): seven, timed bonuses, printed
    /// ones and the layers of the seat's uncontested cards in play (spec
    /// §1.13.3: a contested location gives none).
    pub fn hand_size(&self, s: SeatId) -> usize {
        // Printed (André): one pass, not `printed_mod` per minion, as it is
        // read on every request; only ever `PrintedTo::Itself`.
        let printed: i32 = self
            .minions
            .iter()
            .enumerate()
            .filter(|(_, v)| v.owner == s && v.present())
            .filter_map(|(i, v)| cards::vampire_of(v).map(|d| (MinionId(i as u32), d)))
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
            .map(|(m, p)| p.n as i32 * self.printed_count(p.per, m, PrintedCtx::default()))
            .sum();
        let seat = &self.seats[s.index()];
        let mut size = HAND_SIZE as i32 + seat.timed.total(TimedStat::HandSize) + printed;
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
        // The Bitter and Sweet Story: every Methuselah, per whole VP.
        let vps = (seat.half_vps / 2) as i32;
        for l in self.in_play_layers() {
            if let Modifier::HandSizePerVp(n) = l.modifier {
                size += n as i32 * vps;
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
        // This action's (Hospital Food): only one action at a time.
        let action: u8 = self
            .layers
            .iter()
            .filter(|l| l.scope == Scope::Action)
            .map(|l| match l.modifier {
                Modifier::ActionHunt(n) => n,
                _ => 0,
            })
            .sum();
        action
            + self
                .in_play_layers()
                .map(|l| match l.modifier {
                    Modifier::OwnSectHunt(sect, n)
                        if self.card_controller(l.source) == owner
                            && self.sect_of(m) == Some(sect) =>
                    {
                        n
                    }
                    Modifier::BearerHunt(n) if self.cards[l.source.index()].bearer() == Some(m) => {
                        n
                    }
                    Modifier::BearerHuntOncePerTurn(n)
                        if self.cards[l.source.index()].bearer() == Some(m)
                            && self.cards[l.source.index()].spent_turn != self.turn =>
                    {
                        n
                    }
                    _ => 0,
                })
                .sum::<u8>()
            // Printed "+1 hunt" (Ariadne).
            + self.printed_mod(m, Stat::Hunt, PrintedCtx::default()).max(0) as u8
    }

    /// Cards over hand size, discarded down at once (rulebook "Drawing
    /// Cards"; spec §6.8.1).
    pub fn hand_excess(&self, s: SeatId) -> usize {
        self.hand_load(s).saturating_sub(self.hand_size(s))
    }

    /// The cards counted against the hand size: the hand and the cards not
    /// replaced yet (spec §6.8.1).
    pub fn hand_load(&self, s: SeatId) -> usize {
        let seat = &self.seats[s.index()];
        seat.hand.len() + seat.held.total() + seat.held_in_play.len()
    }

    /// A rider's card and host predicates hold (a hostless card has no
    /// host predicate to fail).
    pub fn rider_holds(&self, r: &cards::Rider, id: CardId) -> bool {
        let c = &self.cards[id.index()];
        !self.text_suppressed(id)
            && r.requires.iter().all(|&req| {
                self.rider_card_holds(req, id).unwrap_or_else(|| {
                    c.bearer()
                        .is_none_or(|h| cards::holds_for_host(req, self, h))
                })
            })
    }

    /// A rider's card predicate, read on its card `id` (`None`: not one).
    pub fn rider_card_holds(&self, req: Requirement, id: CardId) -> Option<bool> {
        let c = &self.cards[id.index()];
        Some(match req {
            Requirement::CountersAtLeast(n) => c.counters >= n,
            Requirement::CountersAtMost(n) => c.counters <= n,
            Requirement::SuperiorVersion => c.mode > 0,
            Requirement::BasicVersion => c.mode == 0,
            Requirement::CardMode(n) => c.mode == n,
            Requirement::OneCopyPerTurn => true,
            Requirement::ActiveIsAgainst => c.against.contains(self.active),
            Requirement::ActiveControlsNonSect(sect) => self
                .minions_of(self.active)
                .any(|m| self.minions[m.index()].is_vampire() && self.sect_of(m) != Some(sect)),
            _ => return None,
        })
    }

    /// The riders on `hook` of every card in the arena, in play or not: the
    /// caller reads each card as it gets to it.
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

    /// How many cards of kind `h` minion `m` holds.
    pub fn holds_count(&self, m: MinionId, h: Holding) -> u8 {
        self.cards
            .iter()
            .filter(|c| c.in_play && c.host == Some(m) && holding_matches(c.card, h))
            .count() as u8
    }

    /// "A vampire can have only one" `card` (Wall of Filth, Pillars Fall,
    /// a delayed "put this card on …"): one of its name on minion `m`.
    pub fn holds_its_name(&self, m: MinionId, card: u16) -> bool {
        self.holds_a(
            m,
            Holding::Named(crate::library::LIBRARY[card as usize].name),
        )
    }

    /// Could `m` take equipment `card` as well, no "only one X" limit
    /// reached by what it has or the cards `with` it in the same action? A
    /// minion with no equipment still takes a locquipment, a location in
    /// play (The Beast, Leatherface of Detroit, through Vast Wealth: LSJ
    /// 20050315).
    pub fn can_hold(&self, m: MinionId, card: u16, with: &[u16]) -> bool {
        if (self.no_equipment(m) && cards::is_equipment_in_play(card))
            || self.ranged_barred(m, card)
        {
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

    /// Uncontested equipment on `m` (a locquipment in play is a location
    /// only, spec §1.3.1).
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

    /// How card `c` in play reduces its controller's costs, if it does.
    pub fn cost_source_of(&self, c: CardId) -> Option<cards::CostSource> {
        self.in_play_layers()
            .filter(|l| l.source == c)
            .find_map(|l| match l.modifier {
                Modifier::CostSource(k) => Some(k),
                _ => None,
            })
    }

    /// What a master card targeting `m` costs `payer` more: its Secure
    /// Havens, whoever plays it (TOM 19951208-1), and Walker Grimes's pool
    /// to his own controller only (LSJ 20081124-1).
    pub fn master_surcharge_on(&self, m: MinionId, payer: SeatId) -> i32 {
        let own = if self.minions[m.index()].owner == payer {
            self.printed_mod(m, Stat::MasterTargetingCostsYou, PrintedCtx::default())
        } else {
            0
        };
        self.havens_on(m) as i32 + own.max(0)
    }

    /// Secure Havens on `m`.
    pub fn havens_on(&self, m: MinionId) -> u8 {
        self.borne(m)
            .filter(|l| l.modifier == Modifier::Haven)
            .count() as u8
    }

    /// British Museum: what `m`'s equip with `card` costs less.
    pub fn equip_discount(&self, m: MinionId, card: u16) -> u8 {
        let owner = self.minions[m.index()].owner;
        // Therbold Realty: an equipment that is a location (LSJ 20080107).
        let location = if LIBRARY[card as usize].labels.contains(&Label::Location) {
            self.location_discount(owner)
        } else {
            0
        };
        if !cards::plain_equipment(card) {
            return location;
        }
        let plain: u8 = sum_n!(self.controlled_layers(owner), Modifier::PlainEquipDiscount);
        location + plain
    }

    /// The blood or pool locations cost `s` less (Therbold Realty).
    pub fn location_discount(&self, s: SeatId) -> u8 {
        sum_n!(self.controlled_layers(s), Modifier::LocationDiscount)
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
                // Ephor: "this Ventrue gets +1 vote".
                Modifier::Bearer {
                    stat: Stat::Votes,
                    n,
                    ..
                } => n as i32,
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

    /// What the cards in play on `m` add to `stat` during its action of
    /// `kind` (`Modifier::Bearer`).
    pub fn bearer_mod(&self, m: MinionId, stat: Stat, kind: ActionKind) -> i32 {
        self.bearer_mod_in(m, stat, kind, false)
    }

    /// As `bearer_mod`, during an action a card may make a hunt (Unlicensed
    /// Taxicab "during hunt … actions"; rules-qa 72.4).
    pub fn bearer_mod_in(&self, m: MinionId, stat: Stat, kind: ActionKind, card_hunt: bool) -> i32 {
        self.bearer_mods(m, stat, kind, card_hunt)
            .map(|(_, n)| n)
            .sum()
    }

    /// The extra blood other Methuselahs' actions targeting `m` cost `seat`
    /// through the cards on it (Chanjelin Ward).
    pub fn targeted_surcharge(&self, m: MinionId, seat: SeatId) -> u8 {
        self.borne(m)
            .map(|l| match l.modifier {
                Modifier::TargetedCostsBlood(n) if self.card_controller(l.source) != seat => n,
                _ => 0,
            })
            .sum()
    }

    /// Can `actor` perform an action directed at `m` (Shadow Cloak [OBL],
    /// Secure Haven, Veneficti)?
    pub fn may_direct_at(&self, actor: MinionId, m: MinionId) -> bool {
        // Secure Haven: "cannot be the target of other Methuselahs'
        // actions" (the caller reads only another Methuselah's minions).
        if self.havens_on(m) > 0
            && self.minions[actor.index()].owner != self.minions[m.index()].owner
        {
            return false;
        }
        // Chanjelin Ward: an ally pays no blood (73.5), and a payment not
        // had is not offered (D3).
        if self.minions[actor.index()].is_ally()
            && self.targeted_surcharge(m, self.minions[actor.index()].owner) > 0
        {
            return false;
        }
        self.borne(m).all(|l| match l.modifier {
            Modifier::DirectedOnlyBy(ds) => ds.iter().any(|d| self.has_discipline(actor, d)),
            // Veneficti: "cannot be the target of a directed action
            // performed by an ally or a vampire with capacity 6 or less".
            Modifier::NotDirectedByAllyOrCapacityAtMost(n) => {
                !self.minions[actor.index()].is_ally()
                    && self.capacity_of(actor).is_none_or(|c| c > n)
            }
            _ => true,
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

    /// The discount on `rescuer`'s rescue of `target` (Adelino Da Silva, Doc
    /// Martina) and the blood gained on success (Saulot's Healing Touch).
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

    /// As `bearer_mod`, with no equipment increasing it, reductions still
    /// applying (Camera Phone grants its own +1 with the action).
    pub fn bearer_mod_without_equipment_increase(
        &self,
        m: MinionId,
        stat: Stat,
        kind: ActionKind,
    ) -> i32 {
        self.bearer_mods(m, stat, kind, false)
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
        titled: Option<bool>,
    ) -> bool {
        self.minions[m.index()].clan() == Some(clan)
            && (!own || self.minions[m.index()].owner == self.card_controller(source))
            && titled.is_none_or(|t| t == (self.title_of(m) != crate::crypt::Title::None))
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

    /// "Vampires with basic <d> can play combat cards requiring <d> as if
    /// they had superior" in play (Tremere Convocation).
    pub fn basic_plays_superior_combat(&self, d: &str) -> bool {
        self.in_play_layers().any(|l| {
            matches!(l.modifier, Modifier::BasicPlaysSuperiorCombat(g) if g.eq_ignore_ascii_case(d))
        })
    }

    /// Stealth vampire `m` gets on an action requiring Discipline `req`
    /// (Tremere Convocation: Tremere, Blood Sorcery).
    pub fn discipline_action_stealth(&self, m: MinionId, req: impl Fn(&str) -> bool) -> i32 {
        self.in_play_layers()
            .map(|l| match l.modifier {
                Modifier::ClanStealthOnActionsRequiring {
                    clan,
                    discipline,
                    n,
                } if self.minions[m.index()].is_vampire()
                    && self.minions[m.index()].clan() == Some(clan)
                    && req(discipline) =>
                {
                    n as i32
                }
                _ => 0,
            })
            .sum()
    }

    /// An Anarch Manifesto on Anarch `m`: its stealth on an action that
    /// requires an Anarch.
    pub fn anarch_action_stealth(&self, m: MinionId) -> i32 {
        if self.sect_of(m) != Some(crate::crypt::Sect::Anarch) {
            return 0;
        }
        self.borne(m)
            .map(|l| match l.modifier {
                Modifier::StealthOnAnarchActions(n) => n as i32,
                _ => 0,
            })
            .sum()
    }

    /// Strength `m` gets in combat with `foe` from the cards `foe` bears
    /// (An Anarch Manifesto).
    pub fn strength_against(&self, m: MinionId, foe: MinionId) -> i32 {
        let v = &self.minions[m.index()];
        if !v.is_vampire()
            || self.title_of(m) == crate::crypt::Title::None
            || self.sect_of(m) == Some(crate::crypt::Sect::Anarch)
        {
            return 0;
        }
        self.borne(foe)
            .map(|l| match l.modifier {
                Modifier::TitledNonAnarchStrengthVsBearer(n) => n as i32,
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
        card_hunt: bool,
    ) -> impl Iterator<Item = (u16, i32)> + '_ {
        let during_reads = move |during: &[ActionKind]| {
            during.is_empty()
                || during.contains(&kind)
                || crate::actions::is_hunt(kind, card_hunt) && during.contains(&ActionKind::Hunt)
        };
        self.in_play_layers().filter_map(move |l| match l.modifier {
            Modifier::Bearer {
                stat: s,
                n,
                during,
                who,
            } if s == stat
                && self.cards[l.source.index()].bearer() == Some(m)
                && during_reads(during)
                && who.is(self, m) =>
            {
                Some((self.cards[l.source.index()].card, n as i32))
            }
            Modifier::SubtypeStat { label, stat: s, n }
                if s == stat && self.has_subtype(m, label) =>
            {
                Some((self.cards[l.source.index()].card, n as i32))
            }
            Modifier::OwnMinionsStat {
                stat: s,
                n,
                vampires,
                during,
            } if s == stat
                && self.minions[m.index()].owner == self.card_controller(l.source)
                && (!vampires || self.minions[m.index()].is_vampire())
                && during_reads(during) =>
            {
                Some((self.cards[l.source.index()].card, n as i32))
            }
            Modifier::SectStat {
                sect,
                stat: s,
                n,
                during,
            } if s == stat
                && self.minions[m.index()].is_vampire()
                && self.sect_of(m) == Some(sect)
                && during_reads(during) =>
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
                && during_reads(during) =>
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

    /// "While Ashur-uballit is ready, zombies (allies and retainers) you
    /// recruit or employ get +1 starting life": the seat's vampires' bonus.
    pub fn zombie_life_bonus(&self, seat: SeatId) -> u8 {
        self.minions_of(seat)
            .map(|m| self.printed_mod(m, Stat::ZombieStartingLife, PrintedCtx::default()))
            .sum::<i32>()
            .max(0) as u8
    }

    /// "This vampire cannot recruit allies or employ retainers" on `m`
    /// (Depravity).
    pub fn cannot_recruit_or_employ(&self, m: MinionId) -> bool {
        self.borne(m)
            .any(|l| matches!(l.modifier, Modifier::HostCannotRecruitOrEmploy))
    }

    /// As `retainer_damage`, at close range only (Dread Mastiff).
    pub fn retainer_damage_close(&self, m: MinionId) -> u8 {
        sum_n!(self.hosted(m), Modifier::RetainerDamageClose)
    }

    /// Optional presses `m` gets each combat: printed (Serhat Gunde) and
    /// from the cards on it (Dread Mastiff [ANI]).
    pub fn combat_presses_of(&self, m: MinionId) -> u8 {
        let printed = self
            .printed_mod(m, Stat::CombatPress, PrintedCtx::default())
            .max(0) as u8;
        let hosted: u8 = sum_n!(self.hosted(m), Modifier::HostCombatPress);
        printed + hosted
    }

    /// The mandatory presses `m`'s cards give it each combat, only usable to
    /// continue in the first round (Talbot's Chainsaw).
    pub fn first_round_presses_of(&self, m: MinionId) -> u8 {
        sum_n!(self.hosted(m), Modifier::HostFirstRoundMandatoryPress)
    }

    /// What `viewer` legally knows of the state (DESIGN §5): a copy, every
    /// card identity they may not see replaced by [`HIDDEN`] in place.
    /// Hidden:
    ///
    /// - every **library** and **crypt** (order unknown; the viewer's own
    ///   keep their contents, sorted);
    /// - another seat's **hand**, unless `hand_visible_to`;
    /// - another seat's face-down **uncontrolled** vampires (rulebook
    ///   "Setup"), a banished one's slot excepted;
    /// - **cards stored face down** on a card in play (Shilmulo Tarot),
    ///   except to its controller.
    ///
    /// A card named from hand (`PlayCtx::named`, `ActionCtx::exchange`) is
    /// announced but never shown (advanced rules §1.14.2).
    pub fn legal_view(&self, viewer: SeatId) -> GameState {
        let mut v = self.clone();
        // The foreign cards' copies of another's hand would tell them.
        v.foreign
            .retain(|(c, _)| v.seats[viewer.index()].hand.copies().contains(&c.0));
        // Hidden cards keep no copy either: a copy would tell which card
        // it is, or where it went.
        let hide = |cards: &mut Pile| {
            for i in 0..cards.len() {
                cards.set_card(i, HIDDEN);
            }
            cards.set_copies(CopyId::NONE);
        };
        for (i, seat) in v.seats.iter_mut().enumerate() {
            let s = SeatId(i as u8);
            if s == viewer {
                seat.library.sort_by_key(|c| c);
                seat.crypt.sort_by_key(|c| c);
                seat.library.set_copies(CopyId::NONE);
                seat.crypt.set_copies(CopyId::NONE);
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
                        slot.copy = Untracked(CopyId::NONE);
                    }
                }
            }
        }
        for (i, c) in v.cards.iter_mut().enumerate() {
            if !self.stored_seen_by(CardId(i as u32), viewer) {
                hide(&mut c.stored);
            }
        }
        v
    }

    /// Whether `viewer` sees the cards stored on `c`: stored face up, or
    /// face down (Shilmulo Tarot, Storage Annex) on a card they control.
    pub fn stored_seen_by(&self, c: CardId, viewer: SeatId) -> bool {
        let face_down = cards::lookup(self.cards[c.index()].card).is_some_and(|d| {
            d.modes.iter().chain(d.abilities).any(|m| {
                m.effects.iter().any(|e| {
                    matches!(
                        e,
                        cards::Effect::StoreTopCards(_) | cards::Effect::StoreFromOwnHand
                    )
                })
            })
        });
        !face_down || self.card_controller(c) == viewer
    }

    /// Whether `viewer` sees `seat`'s hand: their own, or an open hand
    /// (Revelations [AUS], Owl Companion, Harold Tanner, SchreckNET).
    pub fn hand_visible_to(&self, seat: SeatId, viewer: SeatId) -> bool {
        viewer == seat
            || self.peeks.contains(&(viewer, seat))
            || self.seats[seat.index()].combat_open_hand
            || self.in_play_layers().any(|l| {
                l.modifier == Modifier::PreyOpenHand
                    && self.prey(self.card_controller(l.source)) == seat
            })
            || self.combatants.is_some_and(|pair| {
                (0..2).any(|i| {
                    self.minions[pair[1 - i].index()].owner == seat
                        && (self.hosts(pair[i], Modifier::OpponentOpenHandInCombat)
                            || self.printed_mod(
                                pair[i],
                                Stat::OpponentOpenHandInCombat,
                                PrintedCtx::default(),
                            ) > 0)
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
            || self.printed_mod(m, Stat::NoEquipmentOrRetainers, PrintedCtx::default()) > 0
    }

    /// `m` cannot have or use retainers: Bone Shambler, Angelo.
    pub fn no_retainers(&self, m: MinionId) -> bool {
        self.no_equipment_or_retainers(m)
            || self.printed_mod(m, Stat::NoRetainers, PrintedCtx::default()) > 0
    }

    /// `m` cannot have or use equipment: Bone Shambler, Gravebound Drone.
    pub fn no_equipment(&self, m: MinionId) -> bool {
        self.no_equipment_or_retainers(m)
            || self.bears(m, Modifier::NoEquipmentHeld)
            || self.printed_mod(m, Stat::NoEquipment, PrintedCtx::default()) > 0
    }

    /// `seat`'s master card `card` cannot target `m`, another's (Mimir).
    pub fn shielded_from_master(&self, m: MinionId, seat: SeatId, card: u16) -> bool {
        crate::library::LIBRARY[card as usize]
            .types
            .contains(&crate::library::CardType::Master)
            && self.minions[m.index()].owner != seat
            && self.printed_mod(
                m,
                crate::cards::Stat::NoOthersMasterTargeting,
                PrintedCtx::default(),
            ) > 0
    }

    /// `m` cannot be the target of a directed action requiring one of the
    /// Disciplines of `tags` (a mode's tags, any level).
    pub fn immune_to_directed(&self, m: MinionId, tags: &[&str]) -> bool {
        self.borne(m).any(|l| {
            matches!(l.modifier, Modifier::ImmuneToDirectedActions(ds)
                if tags.iter().any(|t| ds.contains(&t.to_ascii_lowercase().as_str())))
        })
    }

    /// Card `id` goes on minion `m`; the counters of one that says so are
    /// removed if it changes minion (Seal of Veddartha: "if another minion
    /// gains control of it").
    pub fn set_host(&mut self, id: CardId, m: MinionId) {
        let c = &mut self.cards[id.index()];
        let clears = cards::lookup(c.card).is_some_and(|d| {
            d.modes.get(c.mode as usize).is_some_and(|md| {
                md.effects
                    .contains(&cards::Effect::CountersClearOnNewBearer)
            })
        });
        if clears && c.host.is_some_and(|h| h != m) {
            c.counters = 0;
        }
        c.host = Some(m);
    }

    /// The only actions `m` may perform, if barred from the others
    /// (Ponticulus, Nosferatu Bestial).
    pub fn only_actions(&self, m: MinionId) -> Option<&'static [ActionKind]> {
        self.borne(m).find_map(|l| match l.modifier {
            Modifier::OnlyActions(kinds) => Some(kinds),
            _ => None,
        })
    }

    /// What `m` — an ally — may play "as a vampire", if anything.
    pub fn plays_as_vampire(&self, m: MinionId) -> Option<cards::AsVampire> {
        self.borne(m).find_map(|l| match l.modifier {
            Modifier::PlaysAsVampire(g) => Some(g),
            _ => None,
        })
    }

    /// How many times `m` performed `card`'s action this turn (NRA).
    pub fn performed_this_turn(&self, m: MinionId, card: u16) -> u8 {
        self.minions[m.index()]
            .acted
            .iter()
            .filter(|a| a.same(ActionId::Card(card)))
            .count() as u8
    }

    /// The repeats of `card`'s action `m` was granted this turn (Nocturn
    /// [OBT]).
    pub fn repeats_of(&self, m: MinionId, card: u16) -> u8 {
        self.minions[m.index()]
            .repeats
            .iter()
            .filter(|&&c| ActionId::Card(c).same(ActionId::Card(card)))
            .count() as u8
    }

    /// `m` is an ally of the subtype its card prints (a changeling).
    pub fn has_subtype(&self, m: MinionId, label: Label) -> bool {
        self.minions[m.index()]
            .kind
            .ally_card()
            .is_some_and(|card| LIBRARY[card as usize].labels.contains(&label))
    }

    /// Only allies of a subtype may block `m`'s undirected hunts (Ravager).
    pub fn only_subtype_blocks_undirected_hunts(&self, m: MinionId) -> Option<Label> {
        self.borne(m).find_map(|l| match l.modifier {
            Modifier::OnlySubtypeBlocksUndirectedHunts(label) => Some(label),
            _ => None,
        })
    }

    /// Allies cannot block `m` (Libertas on it; Stanislava's own text).
    pub fn allies_cannot_block(&self, m: MinionId) -> bool {
        self.hosts(m, Modifier::AlliesCannotBlockBearer)
            || self.printed_mod(m, Stat::AlliesCannotBlock, PrintedCtx::default()) > 0
    }

    /// The card's text is lost: a retainer in combat with Stanislava, for
    /// that combat (rules-qa 49.3; still a retainer taking damage).
    pub fn text_suppressed(&self, id: CardId) -> bool {
        // Midget: acting or in combat, any animal retainer loses its text.
        let c = &self.cards[id.index()];
        let lib = &LIBRARY[c.card as usize];
        if lib.types.contains(&CardType::Retainer) && lib.labels.contains(&Label::Animal) {
            let midget = |m: MinionId| {
                self.minions[m.index()].present()
                    && self.printed_mod(m, Stat::AnimalRetainersLoseText, PrintedCtx::default()) > 0
            };
            if self.acting.is_some_and(midget)
                || self
                    .combatants
                    .is_some_and(|p| p.iter().any(|&m| midget(m)))
            {
                return true;
            }
        }
        let Some(pair) = self.combatants else {
            return false;
        };
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
        self.bears(m, Modifier::HostHuntsToTorpor)
            || self.bears(m, Modifier::HostCannotHunt)
            || self.in_play_layers().any(|l| match l.modifier {
                Modifier::ClanCannotHunt(c) => Some(c) == clan,
                // "Vampires cannot hunt unless they must hunt" (Restricted
                // Vitae).
                Modifier::OnlyMustHunt => !self.must_hunt(m) && !self.gehenna_immune(m),
                _ => false,
            })
    }

    /// The ready untitled vampires of `clan` and `sect`, any Methuselah's
    /// (Osebo Kholo).
    pub fn ready_untitled(
        &self,
        clan: &'static str,
        sect: Sect,
    ) -> impl Iterator<Item = MinionId> + '_ {
        (0..self.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(move |&o| {
                let v = &self.minions[o.index()];
                v.ready()
                    && v.is_vampire()
                    && v.clan() == Some(clan)
                    && self.sect_of(o) == Some(sect)
                    && self.title_of(o) == Title::None
            })
    }

    /// Those of them older than `m`.
    pub fn older_ready_untitled(
        &self,
        clan: &'static str,
        sect: Sect,
        m: MinionId,
    ) -> impl Iterator<Item = MinionId> + '_ {
        self.ready_untitled(clan, sect)
            .filter(move |&o| o != m && self.younger_than(m, o))
    }

    /// `seat`'s n corruption counters on `m` burned by `by`, noted for the
    /// action under way (Rise of the Fallen).
    pub fn burn_corruption(&mut self, m: MinionId, seat: SeatId, n: u8, by: SeatId) {
        self.minions[m.index()].burn_corruption(seat, n);
        if n > 0 && !self.corruption_burned.contains(&by) {
            self.corruption_burned.push(by);
        }
    }

    /// Meets every sect and title requirement of the cards it plays or
    /// uses (Vlad Tepes; ANK 20201028, PIB 20150306).
    pub fn meets_sect_and_title_requirements(&self, m: MinionId) -> bool {
        self.printed_mod(
            m,
            crate::cards::Stat::MeetsSectAndTitleRequirements,
            PrintedCtx::default(),
        ) > 0
    }

    /// Is `m` an ally immune to non-aggravated damage (Tye Cooper)?
    pub fn immune_non_aggravated(&self, m: MinionId) -> bool {
        self.minions[m.index()].kind.ally_card().is_some_and(|c| {
            cards::lookup(c).is_some_and(|d| {
                d.modes
                    .iter()
                    .any(|md| md.effects.contains(&cards::Effect::ImmuneNonAggravated))
            })
        })
    }

    /// Can card `id` not be moved or stolen now: its bearer ready (Eye of
    /// Hazimel)?
    pub fn pinned(&self, id: CardId) -> bool {
        let c = &self.cards[id.index()];
        c.host.is_some_and(|h| self.minions[h.index()].ready())
            && cards::lookup(c.card).is_some_and(|d| {
                d.modes
                    .iter()
                    .any(|md| md.effects.contains(&cards::Effect::PinnedWhileBearerReady))
            })
    }

    /// May `m` take an action in another Methuselah's minion phase: of the
    /// clan an in-play card names (Madness Network)?
    pub fn acts_out_of_turn(&self, m: MinionId) -> bool {
        let clan = self.minions[m.index()].clan();
        self.cards.iter().any(|c| {
            c.present()
                && cards::lookup(c.card).is_some_and(|d| {
                    d.abilities.iter().any(|ab| {
                        ab.effects.contains(&cards::Effect::ActorTakesAnyAction)
                            && ab
                                .requires
                                .iter()
                                .any(|r| matches!(r, Requirement::ActorClan(k) if Some(*k) == clan))
                    })
                })
        })
    }

    /// Is `m` a Seraph (its card's trait)?
    pub fn is_seraph(&self, m: MinionId) -> bool {
        self.minions[m.index()].vampire().is_some_and(|c| c.seraph)
    }

    /// Is `m` Black Hand: printed, or become so (Mateusz Kazimirski)?
    pub fn is_black_hand(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.gained_black_hand || v.vampire().is_some_and(|c| c.black_hand)
    }

    /// Is `m` on the Red List (its card's trait)?
    pub fn is_red_list(&self, m: MinionId) -> bool {
        self.minions[m.index()].made_red_list
            || self.minions[m.index()]
                .vampire()
                .is_some_and(|c| c.red_list)
    }

    /// Must `m` hunt: a vampire with no blood, or made to?
    pub fn must_hunt(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.is_vampire() && (v.blood == 0 || v.must_hunt)
    }

    /// A printed stat summed over `seat`'s minions, in torpor too (rules-digest
    /// D25): what "you get" from its vampires (Fiorenza Savona).
    pub fn seat_printed(&self, seat: SeatId, stat: Stat) -> i32 {
        self.minions_of(seat)
            .map(|m| self.printed_mod(m, stat, PrintedCtx::default()))
            .sum()
    }

    /// `m` cannot block (Pentex™ Subversion): a card on it, its own text,
    /// or the acting minion's (Aisling Sturbridge).
    pub fn cannot_block(&self, m: MinionId, acting: MinionId) -> bool {
        let ctx = PrintedCtx {
            opponent: Some(acting),
            ..PrintedCtx::default()
        };
        let against = PrintedCtx {
            opponent: Some(m),
            ..PrintedCtx::default()
        };
        self.hosts(m, Modifier::HostCannotBlock)
            || self.printed_mod(m, cards::Stat::CannotBlock, ctx) > 0
            || self.printed_mod(acting, cards::Stat::CannotBeBlockedBy, against) > 0
    }

    /// `m`'s non-bleed actions cannot be blocked (Toreador Grand Ball), still
    /// after it bled (LSJ 19970814).
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

    /// `striker`, in combat with `m`, cannot strike: combat ends (Dog Pack,
    /// Faruq Abd al-Qadir).
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

    /// The clan `m` is enslaved to: a created vampire's (Raw Recruit), else
    /// its crypt card's.
    pub fn slave_clan(&self, m: MinionId) -> Option<&'static str> {
        let v = &self.minions[m.index()];
        v.slave_of.or_else(|| v.vampire().and_then(|c| c.slave))
    }

    /// `m` may act, block and play cards by its tether: none, or its vampire
    /// ready (Legion).
    pub fn tether_holds(&self, m: MinionId) -> bool {
        self.minions[m.index()]
            .tethered_to
            .is_none_or(|t| self.minions[t.index()].ready())
    }

    /// The locations `s` controls, uncontested (a contested card is not
    /// present), locations on its minions included.
    pub fn locations_controlled(&self, s: SeatId) -> usize {
        self.cards
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                c.present()
                    && self.card_controller(CardId(*i as u32)) == s
                    && LIBRARY[c.card as usize].labels.contains(&Label::Location)
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

    /// Is `m` barred from blocking or entering combat with `other`
    /// (Reginald Moore and the Tremere)?
    pub fn shuns(&self, m: MinionId, other: MinionId) -> bool {
        let ctx = PrintedCtx {
            opponent: Some(other),
            ..Default::default()
        };
        self.printed_mod(m, Stat::NoBlockOrRush, ctx) > 0
    }

    /// Could `m` take the rulebook action to become Anarch: a ready
    /// untitled vampire not Anarch already?
    pub fn may_become_anarch(&self, m: MinionId) -> bool {
        let v = &self.minions[m.index()];
        v.is_vampire()
            && v.ready()
            && self.title_of(m) == crate::crypt::Title::None
            && self.sect_of(m) != Some(Sect::Anarch)
    }

    /// Become Anarch's blood: 2, 1 if its Methuselah controls another
    /// ready Anarch, 0 under Brunhilde (advanced rules "Become Anarch").
    pub fn become_anarch_cost(&self, m: MinionId) -> u8 {
        if self.printed_mod(m, Stat::BecomeAnarchFree, PrintedCtx::default()) > 0 {
            return 0;
        }
        let owner = self.minions[m.index()].owner;
        let other = self.minions_of(owner).any(|o| {
            o != m
                && self.minions[o.index()].ready()
                && self.minions[o.index()].is_vampire()
                && self.sect_of(o) == Some(Sect::Anarch)
        });
        if other {
            1
        } else {
            2
        }
    }

    /// Gehenna events do not affect `m` (Karen Suadela while a prince):
    /// chosen for them, it stays unaffected (LSJ 20081120-2).
    pub fn gehenna_immune(&self, m: MinionId) -> bool {
        self.minions[m.index()].is_vampire()
            && self.printed_mod(m, Stat::GehennaImmune, PrintedCtx::default()) > 0
    }

    /// The blood `m` just gained (`blood_gains`), 0 if none.
    pub fn blood_gained(&self, m: MinionId) -> u8 {
        self.blood_gains
            .iter()
            .filter(|&&(g, _, _)| g == m)
            .map(|&(_, n, _)| n)
            .sum()
    }

    /// The vampires of Dual Form `root`'s set still in play, `root` included.
    pub fn dual_set(&self, root: MinionId) -> Vec<MinionId> {
        (0..self.minions.len())
            .map(|i| MinionId(i as u32))
            .filter(|&o| {
                (o == root || self.minions[o.index()].dual_of == Some(root))
                    && self.minions[o.index()].present()
            })
            .collect()
    }

    /// Are `a` and `b` two Blood Brothers of the same circle? A vampire
    /// with no circle is in a circle of one (LSJ 20020609).
    pub fn same_circle(&self, a: MinionId, b: MinionId) -> bool {
        let (x, y) = (&self.minions[a.index()], &self.minions[b.index()]);
        a != b
            && x.clan() == Some("Blood Brother")
            && y.clan() == Some("Blood Brother")
            && x.circle().is_some()
            && x.circle() == y.circle()
    }

    /// As `same_circle`, for a card requiring Sanguinus `actor` plays and
    /// the effects it has (not from being in play): `other` printing "as if
    /// it were a member of any circle" (Angelo) is of `actor`'s (LSJ
    /// 20101013).
    pub fn same_circle_for_play(&self, actor: MinionId, other: MinionId) -> bool {
        self.same_circle(actor, other)
            || (actor != other
                && self.minions[actor.index()].clan() == Some("Blood Brother")
                && self.minions[other.index()].clan() == Some("Blood Brother")
                && self.printed_mod(other, Stat::AnyCircleForSanguinus, PrintedCtx::default()) > 0)
    }

    /// `m`'s sect: its own, unless fallen (Fall of the Camarilla, Fall of the
    /// Sabbat: Independent, a recent change included; LSJ 20080510).
    pub fn sect_of(&self, m: MinionId) -> Option<Sect> {
        let s = self.minions[m.index()].underlying_sect()?;
        // A Fall of a sect is a Gehenna event: Karen keeps hers, and her
        // title (LSJ 20100121).
        let fallen = matches!(s, Sect::Camarilla | Sect::Sabbat)
            && self.global(Modifier::SectFallen(s)).is_some()
            && !self.gehenna_immune(m);
        Some(if fallen { Sect::Independent } else { s })
    }

    /// Blood vampires attempting to block `m` burn (Archon, per copy).
    pub fn blockers_of_burn(&self, m: MinionId) -> u8 {
        sum_n!(self.hosted(m), Modifier::BlockersOfHostBurn)
    }

    /// Can no blood hunt be called on `m` (Archon)?
    pub fn blood_hunt_barred(&self, m: MinionId) -> bool {
        self.hosts(m, Modifier::NoBloodHuntOnHost)
            || self.printed_mod(m, Stat::NoBloodHunt, PrintedCtx::default()) > 0
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
        // A non-unique vampire (Valkyrie): no self-contest; Jimmy Dunn is
        // never contested (RTR 20180303: a second one can come out).
        if !CRYPT[card as usize].unique || cards::prints(card, Stat::NeverContested) {
            return false;
        }
        let name = CRYPT[card as usize].name;
        self.minions
            .iter()
            .any(|m| m.owner == s && m.in_play && m.name() == name)
    }

    /// Other seats' in-play copies of `m`'s vampire or unique ally, by name
    /// across minion types (advanced rules 5.1.3).
    pub fn card_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let me = &self.minions[m.index()];
        // A created vampire is non-unique (advanced rules 5.1.3), and so is
        // one printing "Non-unique." (Valkyrie).
        if matches!(me.kind, MinionKind::Created { .. }) || me.vampire().is_some_and(|v| !v.unique)
        {
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
                    && !o.out_of_play
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
                    && !o.out_of_play
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

    /// The blood `m` pays to keep contesting its title: 1, more by its
    /// printed toll (Vasily, unless his Methuselah has the Edge).
    pub fn title_contest_cost(&self, m: MinionId) -> u8 {
        (1 + self.printed_mod(m, Stat::TitleContestToll, PrintedCtx::default())).max(0) as u8
    }

    /// Damage `from` inflicts on `m` less: its strikes and its own non-strike
    /// damage (Pulled Fangs), not environmental (Nephandus; Saulot to 0).
    pub fn damage_reduction(&self, m: MinionId, from: MinionId) -> u8 {
        let ctx = PrintedCtx {
            opponent: Some(from),
            ..PrintedCtx::default()
        };
        if self.printed_mod(m, Stat::NoDamageFromOpponent, ctx) > 0 {
            return u8::MAX;
        }
        self.borne(m)
            .map(|l| match l.modifier {
                Modifier::OpposingDamageLess(n) => n,
                _ => 0,
            })
            .sum()
    }

    /// The in-play layers of the cards `m` bears (`CardInPlay::bearer`).
    pub fn borne(&self, m: MinionId) -> impl Iterator<Item = &Layer> + '_ {
        self.in_play_layers()
            .filter(move |l| self.cards[l.source.index()].bearer() == Some(m))
    }

    /// The in-play layers of the cards on `m` (`CardInPlay::host`).
    pub fn hosted(&self, m: MinionId) -> impl Iterator<Item = &Layer> + '_ {
        self.in_play_layers()
            .filter(move |l| self.cards[l.source.index()].host == Some(m))
    }

    /// The capacity from which vampires cannot attempt to block `m` (The
    /// Signet of King Saul), if any: the lowest such.
    pub fn big_cannot_block(&self, m: MinionId) -> Option<u8> {
        self.borne(m)
            .filter_map(|l| match l.modifier {
                Modifier::BigVampiresCannotBlockBearer(n) => Some(n),
                _ => None,
            })
            .min()
    }

    /// Does a card `m` bears give `modifier`?
    pub fn bears(&self, m: MinionId, modifier: Modifier) -> bool {
        self.borne(m).any(|l| l.modifier == modifier)
    }

    /// Can `m` not have or use the weapon `card` — a ranged weapon under
    /// Anachronism, its named exception aside?
    pub fn ranged_barred(&self, m: MinionId, card: u16) -> bool {
        cards::is_ranged_weapon(card)
            && self.hosted(m).any(|l| {
                matches!(l.modifier, Modifier::NoRangedWeaponsExcept(name)
                    if LIBRARY[card as usize].name != name)
            })
    }

    /// Does a card on `m` give `modifier`?
    pub fn hosts(&self, m: MinionId, modifier: Modifier) -> bool {
        self.hosted(m).any(|l| l.modifier == modifier)
    }

    /// The title card on vampire `m` and its title: under a title contest
    /// too, not a card contest (rules-qa 19.1).
    pub fn title_card_on(&self, m: MinionId) -> Option<(CardId, Title)> {
        let title_contested = self.minions[m.index()].title_contested;
        self.cards.iter().enumerate().find_map(|(i, c)| {
            (c.in_play && c.host == Some(m) && (!c.contested || title_contested))
                .then(|| cards::card_title(c.card))
                .flatten()
                .map(|t| (CardId(i as u32), t))
        })
    }

    /// Does a vampire of `s`'s other than `besides` claim a unique title
    /// that would contest `t` (advanced rules §1.13.1: no contest with
    /// oneself)? One out of play for now counts, a contested copy or under
    /// Descent into Darkness: it contests the title as it returns (rules-qa
    /// 7.1, 100.4).
    pub fn seat_claims_title(&self, s: SeatId, t: Title, besides: Option<MinionId>) -> bool {
        let Some(key) = title_key(t) else {
            return false;
        };
        (0..self.minions.len())
            .map(|i| MinionId(i as u32))
            .any(|o| {
                let v = &self.minions[o.index()];
                v.owner == s
                    && v.in_play
                    && Some(o) != besides
                    && title_key(self.claimed_title(o)) == Some(key)
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

    /// Effective title (read live): the claimed title, none while contested,
    /// out of play or off-sect (rulebook "Vampiric Sects"). A justicar's
    /// clan is checked as the title is given: no pool card changes a clan.
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
            Title::Magaji | Title::Laibon { .. } => sect == Sect::Laibon,
            Title::Independent { .. } => sect == Sect::Independent,
            // A personal title, kept through a sect change (LSJ 20010714).
            Title::Votes(_) => true,
        };
        if fits {
            t
        } else {
            Title::None
        }
    }

    /// Does `m` follow `path`, as a Sabbat vampire (Black Chantry,
    /// 2025-06-12)?
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

    /// Votes vampire `m` has outside a referendum: its title's, printed,
    /// borne and its clan's (Island of Yiaros).
    pub fn votes_held(&self, m: MinionId) -> u8 {
        let printed = self.printed_mod(m, Stat::Votes, PrintedCtx::default());
        (i32::from(self.title_votes_of(m)) + printed + self.host_votes(m) + self.clan_votes(m))
            .clamp(0, 255) as u8
    }

    /// Other present vampires (any seat) claiming a title that contests
    /// the one `m` claims.
    pub fn title_rivals(&self, m: MinionId) -> Vec<MinionId> {
        let Some(key) = title_key(self.claimed_title(m)) else {
            return Vec::new();
        };
        let me = &self.minions[m.index()];
        self.minions
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != m.index()
                    && o.present()
                    // Two-player: two contested copies' title is not
                    // contested (their card contest entered first).
                    && !(me.contested_in_play && o.contested_in_play && o.name() == me.name())
                    && !self.seats[o.owner.index()].ousted
                    && title_key(self.claimed_title(MinionId(*i as u32))) == Some(key)
            })
            .map(|(i, _)| MinionId(i as u32))
            .collect()
    }

    /// A card that left a hand not its owner's (`foreign`): to its owner's
    /// ash heap or library if it went to another's, its owner's card if it
    /// went into play. The entry stays while the card is in a hand, stored
    /// on a card or in flight (a play under way, in no pile yet), and goes
    /// once it is found elsewhere.
    pub fn settle_foreign_cards(&mut self) {
        if self.foreign.is_empty() {
            return;
        }
        let entries = std::mem::take(&mut self.foreign);
        for (copy, owner) in entries {
            let copy = copy.0;
            let held = self.seats.iter().any(|s| s.hand.copies().contains(&copy))
                || self.cards.iter().any(|c| c.stored.copies().contains(&copy));
            let mut found = false;
            for s in 0..self.seats.len() {
                let seat = &self.seats[s];
                if let Some(i) = seat.ash.copies().iter().position(|&c| c == copy) {
                    found = true;
                    if s != owner.index() {
                        let (card, copy) = self.seats[s].ash.take(i);
                        self.seats[owner.index()].ash.put(card, copy);
                    }
                } else if let Some(i) = seat.library.copies().iter().position(|&c| c == copy) {
                    found = true;
                    if s != owner.index() {
                        let (card, copy) = self.seats[s].library.take(i);
                        let at = i.min(self.seats[owner.index()].library.len());
                        self.seats[owner.index()].library.put_at(at, card, copy);
                    }
                } else if seat.removed.copies().contains(&copy) {
                    found = true;
                }
            }
            for c in &mut self.cards {
                if c.in_play && *c.copy == copy {
                    c.owner = owner;
                    found = true;
                }
            }
            if held || !found {
                self.foreign.push((crate::copies::Untracked(copy), owner));
            }
        }
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

/// The circles printed in the crypt, in crypt order (New Blood's choices).
pub fn circles() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in CRYPT.iter().filter_map(|c| c.circle) {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// The Disciplines printed in the crypt, lower case, in crypt order
/// (Sanguine Instruction's choices); an imbued's virtues are none.
pub fn disciplines() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for d in CRYPT
        .iter()
        .filter(|c| !c.imbued)
        .flat_map(|c| c.disciplines.iter())
    {
        if d.chars().all(|c| c.is_ascii_lowercase()) && !out.contains(d) {
            out.push(d);
        }
    }
    out
}
