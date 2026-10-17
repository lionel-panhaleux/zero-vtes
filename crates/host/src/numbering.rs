//! Copies as one viewer knows them (docs/protocol.md §3): the engine's
//! copies (dealt in decklist order, `zvtes::copies`) renamed per viewer.
//! A copy the viewer sees keeps its number; out of sight it is forgotten,
//! and back in sight it takes a fresh one, never used before for this
//! viewer. `nth` labels the visible copies of a card from one deck: the
//! lowest number no other holds.

use std::collections::{BTreeSet, HashMap};

use zvtes::copies::{census, CopyId, Place};
use zvtes::game::Engine;

/// Each engine copy's deck: the seat it was dealt to.
#[derive(Clone, Debug, Default)]
pub struct Decks(HashMap<u16, u8>);

impl Decks {
    /// From the game as dealt.
    pub fn dealt(e: &Engine) -> Decks {
        Decks(
            census(e)
                .into_iter()
                .filter_map(|(c, _, place)| match place {
                    Place::Zone(s, _) | Place::Uncontrolled(s) => Some((c.0, s)),
                    _ => None,
                })
                .collect(),
        )
    }

    fn of(&self, c: CopyId) -> u8 {
        self.0.get(&c.0).copied().unwrap_or(u8::MAX)
    }
}

/// Minions' names as one viewer meets them: a minion whose name another
/// minion of the game had first is numbered (`Card::numbered`), by wire
/// copy, kept once given.
#[derive(Clone, Debug, Default)]
pub struct MinionNames {
    named: HashMap<u32, (String, Option<u8>)>,
}

impl MinionNames {
    /// The number of the minion `wire`, named `base` (a ghoul token never
    /// numbered): its own while its name holds, else the lowest no other
    /// minion of the name holds (the first unnumbered, 1).
    pub fn label(&mut self, wire: u32, base: &str, token: bool) -> Option<u8> {
        if let Some((b, n)) = self.named.get(&wire) {
            if b == base {
                return *n;
            }
        }
        let held: BTreeSet<u8> = self
            .named
            .iter()
            .filter(|(&o, (b, _))| o != wire && b == base)
            .map(|(_, (_, n))| n.unwrap_or(1))
            .collect();
        let k = (1..=u8::MAX).find(|k| !held.contains(k)).unwrap_or(u8::MAX);
        let n = (k > 1 && !token).then_some(k);
        self.named.insert(wire, (base.into(), n));
        n
    }
}

/// A visible card: its engine copy and face (KRCG id, crypt).
pub type Seen = (CopyId, u32, bool);

#[derive(Clone, Debug, Default)]
pub struct Numbering {
    /// Per engine copy in sight: its wire copy and `nth`, and its face
    /// (KRCG id, crypt).
    wire: HashMap<u16, Named>,
    /// The copies the last update put out of sight, as they were named:
    /// what the events since name them by.
    gone: HashMap<u16, Named>,
    next: u32,
}

#[derive(Clone, Copy, Debug)]
struct Named {
    wire: u32,
    nth: u8,
    id: u32,
    crypt: bool,
}

impl Numbering {
    /// The viewer now sees exactly `seen`: copies out of sight forgotten,
    /// new ones numbered and labelled.
    pub fn update(&mut self, decks: &Decks, seen: &[Seen]) {
        let now: HashMap<u16, (u32, bool)> = seen
            .iter()
            .filter(|(c, _, _)| *c != CopyId::NONE)
            .map(|&(c, id, crypt)| (c.0, (id, crypt)))
            .collect();
        let gone: Vec<u16> = self
            .wire
            .keys()
            .copied()
            .filter(|c| !now.contains_key(c))
            .collect();
        self.gone.clear();
        for c in gone {
            if let Some(n) = self.wire.remove(&c) {
                self.gone.insert(c, n);
            }
        }
        let mut fresh: Vec<(u16, u32, bool)> = now
            .iter()
            .filter(|(c, _)| !self.wire.contains_key(c))
            .map(|(&c, &(id, crypt))| (c, id, crypt))
            .collect();
        // In engine order: the same table numbers alike for every run.
        fresh.sort_unstable();
        for (c, id, crypt) in fresh {
            self.fresh(decks, CopyId(c), id, crypt);
        }
    }

    /// A new wire copy for `c`, with the lowest `nth` no copy of its card
    /// from its deck in sight holds.
    fn fresh(&mut self, decks: &Decks, c: CopyId, id: u32, crypt: bool) -> Named {
        let deck = decks.of(c);
        let used: BTreeSet<u8> = self
            .wire
            .iter()
            .filter(|(&o, n)| n.id == id && n.crypt == crypt && decks.of(CopyId(o)) == deck)
            .map(|(_, n)| n.nth)
            .collect();
        let nth = (1..=u8::MAX).find(|n| !used.contains(n)).unwrap_or(u8::MAX);
        self.next += 1;
        let named = Named {
            wire: self.next,
            nth,
            id,
            crypt,
        };
        self.wire.insert(c.0, named);
        named
    }

    /// The wire copy and label of a card an event names, a public one:
    /// as in sight, or as it was just before going out of sight, else a
    /// fresh one (it came and went between two views).
    pub fn name(&mut self, decks: &Decks, c: CopyId, id: u32, crypt: bool) -> Option<(u32, u8)> {
        if c == CopyId::NONE {
            return None;
        }
        let n = match self.wire.get(&c.0).or(self.gone.get(&c.0)) {
            Some(&n) => n,
            None => self.fresh(decks, c, id, crypt),
        };
        Some((n.wire, n.nth))
    }

    /// The wire copy and label of a visible engine copy.
    pub fn get(&self, c: CopyId) -> Option<(u32, u8)> {
        self.wire.get(&c.0).map(|n| (n.wire, n.nth))
    }
}
