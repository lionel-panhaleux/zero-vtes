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

/// A visible card: its engine copy and face (KRCG id, crypt).
pub type Seen = (CopyId, u32, bool);

#[derive(Clone, Debug, Default)]
pub struct Numbering {
    wire: HashMap<u16, (u32, u8)>,
    next: u32,
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
        for c in gone {
            self.wire.remove(&c);
        }
        // Rebuild the labels in use from the copies still numbered.
        let mut labels: HashMap<(u8, u32, bool), BTreeSet<u8>> = HashMap::new();
        for (&c, &(_, nth)) in &self.wire {
            if let Some(&(id, crypt)) = now.get(&c) {
                labels
                    .entry((decks.of(CopyId(c)), id, crypt))
                    .or_default()
                    .insert(nth);
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
            let used = labels.entry((decks.of(CopyId(c)), id, crypt)).or_default();
            let nth = (1..=u8::MAX).find(|n| !used.contains(n)).unwrap_or(u8::MAX);
            used.insert(nth);
            self.next += 1;
            self.wire.insert(c, (self.next, nth));
        }
    }

    /// The wire copy and label of a visible engine copy.
    pub fn get(&self, c: CopyId) -> Option<(u32, u8)> {
        self.wire.get(&c.0).copied()
    }
}
