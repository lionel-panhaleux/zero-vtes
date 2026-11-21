#!/usr/bin/env python3
"""Draft window matrix for the V5 pool.

Classifies each library card's play window(s) and flags from KRCG card data,
by type and text patterns. This is a *draft* generator: its output seeds the
hand-reviewed per-card matrix (`data/window-matrix-reviewed.csv`); the
conventions encoded here are the owner's window arbitration (`docs/rules-qa.md`
Batch 2).

Classification is per *mode*: card text is split into a preamble (shared
restrictions such as "Only usable during a bleed action.") and one block per
discipline/type line (`[ani] …`, `[ANI] …`, `[pro] [COMBAT] …`). Each mode is
classified on preamble+mode text; "As above" modes inherit the previous mode's
windows; rider phrases that fire after the card's own play step ("with 1
optional press", "gets 1 optional maneuver", granted "can strike:"/"can
prevent") are stripped before matching so they never open a window. Windows
are the union over modes.

Within a mode, an explicit *timing* window ("as the action is announced",
"if a minion attempts to block", "during the polling step", …) suppresses the
generic stealth/intercept/bleed-modifier windows of the same mode: the printed
timing is the legality filter.

Usage: python3 scripts/window_matrix.py [path/to/vtes.json] > draft.csv
       python3 scripts/window_matrix.py [path/to/vtes.json] --check data/window-matrix-reviewed.csv
Downloads https://static.krcg.org/data/vtes.json if no path is given (KRCG's
v5 schema, read through scripts/krcg.py).
`--check` prints every card whose generated windows/flags differ from the
reviewed matrix and exits non-zero if any do (convergence is expected to hold:
the generator encodes the review's conventions).
"""
import csv
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import krcg  # noqa: E402

V5_SETS = {
    "Fifth Edition": "V5",
    "Fifth Edition (Anarch)": "V5A",
    "Fifth Edition (Companion)": "V5C",
    "New Blood": "NB",
    "New Blood II": "NB2",
    "New Blood III": "NB3",
    "Fall of London": "FoL",
    "Shadows of Berlin": "SoB",
    "Echoes of Gehenna": "EoG",
}

# Legacy cards the owner added to the pool (2026-09-25: common electronic
# equipment, so Darius Shirazi's "non-electronic" has something to exclude).
EXTRA_CARDS = {
    100287: "Camera Phone",
    100545: "Direct Intervention",
    101073: "Laptop Computer",
    102151: "Wash",
}


# Legacy cards the owner added to the pool so that tournament winning decks
# (TWDA) and the anniversary decks become playable. Unlike EXTRA_CARDS they
# are appended after the V5 tables, batch after batch in this order, sorted
# by id within a batch, never sorted in: the existing CRYPT and LIBRARY
# indices stay put (golden replays, decklist tables, trained vocabularies).
ADDED_BATCHES = [
    # 2026-09-28: TWDA decks.
    {
        100106: "Ashur Tablets",
        100298: "Carlton Van Wyk",
        100576: "Donnybrook",
        100646: "Ennoia's Theater",
        100910: "Hell-for-Leather",
        101019: "Jake Washington",
        101103: "Life Boon",
        101256: "Murmur of the False Will",
        101442: "Powerbase: Savannah",
        101654: "Rötschreck",
        101665: "Saatet-ta",
        201617: "Gilbert Duane (G6)",
    },
    # 2026-10-06: Reign of Stanislava (25th Anniversary), its new cards.
    {
        100064: "Ancient Influence",
        100645: "Enkil Cog",
        100710: "Fear of Mekhet",
        100761: "Forced March",
        100994: "Instantaneous Transformation",
        101238: "Monastery of Shadows",
        101390: "Permanent Vacation",
        101417: "Political Stranglehold",
        101591: "Reins of Power",
        102207: "Zillah's Valley",
        200575: "Hartmut Stover (G3)",
        200634: "Ingrid Rossler (G2)",
        200937: "Mark Decker (G3)",
        201312: "Stanislava (G2)",
        201478: "Xaviar (G3 ADV)",
    },
    # 2026-10-06: The Endless Dance (30th Anniversary), its new cards (the
    # deck's Ancient Influence, Political Stranglehold, Reins of Power and
    # Zillah's Valley came with Reign of Stanislava).
    {
        100119: "Awe",
        100274: "Business Pressure",
        100333: "Charismatic Aura",
        101127: "The Louvre, Paris",
        101590: "Reinforcements",
        102269: "Camarilla Conclave",
        102271: "Confiscation",
        102273: "Loup",
        201694: "Diana Iadanza (G7)",
        201696: "François Villon (G6)",
        201699: "Kazuki Hanabira (G7)",
        201702: "Marcos Belegrad (G7)",
        201703: "Modius (G7)",
        201706: "Silvian Popescu (G7)",
        201707: "Valentina Osako (G7)",
    },
    # 2026-10-07: Sabbat V5, the Path of Power and the Inner Voice deck's
    # new cards (with Under Siege and Unholy Sacrament, shared with later
    # decks).
    {
        100165: "Black Forest Base",
        100294: "Cardinal Benediction",
        100307: "Catatonic Fear",
        100541: "Día de los Muertos",
        101008: "Iron Glare",
        101437: "Powerbase: Madrid",
        101490: "Private Audience",
        101587: "Regent",
        102063: "Under Siege",
        102308: "Absolute Tyranny",
        102312: "Cold War",
        102316: "Dominant Personality",
        102318: "Empires Fall",
        102331: "Magnetic Authority",
        102334: "Privileged Position",
        102337: "Relentlessness",
        102345: "Unholy Sacrament",
        102346: "Unthinkable Humiliation",
        102348: "War of Ages",
        201733: "Aaradhya, The Callous Tyrant (G6)",
        201744: "Concordia (G6)",
        201747: "Damian (G6)",
        201760: "Khin Aye (G6)",
        201762: "Lika, The Purger (G6)",
        201764: "Lyyli Yösydän (G6)",
        201766: "Onkabetse, Arriviste (G6)",
        201769: "Rasha Tarabzouni (G6)",
        201770: "Rexton \"Savage\" Abernathy (G6)",
        201774: "Suwira (G6)",
        201775: "Üresség (G6)",
        201776: "Věnceslava, The Implacable (G6)",
    },
    # 2026-10-07: Sabbat V5, the Path of Cathari deck's new cards (The
    # Hungry Coyote a legacy reprint).
    {
        100945: "Hungry Coyote, The",
        102320: "Enthrall",
        102321: "Fever Pitch",
        102323: "Forward Momentum",
        102328: "Hedonism",
        102330: "Kiss of Cathari",
        102338: "Seeds of Terror",
        102341: "Subversion",
        102342: "Swift Cover",
        102343: "Terrifying Visage",
        201735: "Aelswith, The Irresistible (G6)",
        201736: "Agnieszka, Tempter of Legions (G6)",
        201743: "Coco Thorn (G6)",
        201746: "Cyan Sin (G6)",
        201748: "Devorah (G6)",
        201751: "Fotini Katsikaris (G6)",
        201753: "Glissando (G6)",
        201754: "Gostoso (G6)",
        201758: "Jürgen, The Libertine (G6)",
        201767: "Osvaldo Kühnemann (G6)",
        201777: "Verrix, Naughty Boy (G6)",
        201778: "Ximena (G6)",
    },
    # 2026-10-07: Sabbat V5, the Path of Caine deck's new cards (Abbot,
    # Depravity, Fiendish Tongue and Yawp Court legacy reprints).
    {
        100006: "Abbot",
        100526: "Depravity",
        100726: "Fiendish Tongue",
        102199: "Yawp Court",
        102309: "Abuse of Power",
        102310: "Augury of Doom",
        102315: "Dance with the Devil",
        102317: "Dread Mastiff",
        102319: "Ensnare a Beast",
        102329: "Hunting the Quarry",
        102333: "Pillars Fall",
        102336: "Rage of Apedemak",
        102339: "Sentry Signal",
        102340: "Soulgrinder",
        102344: "Twisted Bloodhound",
        102347: "Wall of Filth",
        201738: "Algirdas, The Solar Prophet (G6)",
        201739: "Anousha, The Maniac (G6)",
        201742: "Berenguela, The Vicious (G6)",
        201745: "Crossbreaker (G6)",
        201755: "Grażyna \"Butcher\" Olesińska (G6)",
        201756: "Hekla \"Bonecrusher\" Jónasdóttir (G6)",
        201759: "Kalyani, Agent of Doom (G6)",
        201761: "Lawbringer (G6)",
        201763: "Loyiso, The Ruthless (G6)",
        201768: "Przemysław, War Beast (G6)",
        201772: "Serhat Gunde, Combat Junkie (G6)",
        201773: "Surapong (G6)",
    },
    # 2026-10-07: Sabbat V5, the Path of Death and the Soul deck's new cards
    # (Glancing Blow, Obedience, Target Vitals and Wall Street Night legacy
    # reprints).
    {
        100834: "Glancing Blow",
        101309: "Obedience",
        101942: "Target Vitals",
        102142: "Wall Street Night, Financial Newspaper",
        102311: "Burial Site Hunting Ground",
        102313: "Cursed Abattoir",
        102314: "Dance of the Dead",
        102322: "Fiorella, Empty One",
        102324: "Freakish Conglomeration",
        102325: "Gifts From Hereafter",
        102326: "Gravebound Drone",
        102327: "Heartrender",
        102332: "Morbidity",
        102335: "Putrescent Sustenance",
        201734: "Abderrahim, Death's Hand (G6)",
        201737: "Aitana, The Unholy (G6)",
        201740: "Anxo Vilela (G6)",
        201741: "Ashur-uballit, Son of the Abyss (G6)",
        201749: "El Monseñor (G6)",
        201750: "Eulogio Sánchez de los Reyes (G6)",
        201752: "Frau Schädel (G6)",
        201757: "Huldu, The Desecrator (G6)",
        201765: "Nadezhda (G6)",
        201771: "Sakura, The Merciless (G6)",
        201779: "Yurena, The Boor (G6)",
        201780: "Zbigniew, Hunter of Mekhet (G6)",
    },
    # 2026-10-08: TWDA unlocks, step 0 (staples gating many archetypes) and
    # steps 1-3 (owner: the legacy cards opening the most new TWDA
    # archetypes).
    {
        100266: "Bum's Rush",
        100581: "Dragonbound",
        100908: "Heidelberg Castle, Germany",
        101015: "J. S. Simmons, Esq.",
        101071: "Lam Into",
        101261: "Mylan Horseed",
        101697: "Scourge of the Enochians",
        101943: "Tasha Morgan",
        200060: "Alonso Petrodon (G5)",
        200076: "Anarch Convert (ANY)",
        201527: "Gabriel Tremblay (G6)",
        201633: "Maila (G6)",
    },
    # 2026-10-08: TWDA unlocks, steps 4-9.
    {
        100555: "Disguised Weapon",
        100876: "Hackerspace",
        101179: "Mask of a Thousand Faces",
        101377: "Patsy",
        101690: "SchreckNET",
        102071: "Unflinching Persistence",
        200136: "Arishat (G6)",
        200190: "Benjamin Rose (G5)",
        200224: "Brunhilde (G3 ADV)",
        200317: "Danielle Diron (G3 ADV)",
        200349: "Dmitra Ilyanova (G5)",
        200410: "Elimelech the Twice-Damned (G5)",
        200753: "Karen Suadela (G5)",
        200968: "The Medic (G5)",
        201176: "Reginald Moore (G4)",
        201253: "Scout Youngwood (G6)",
        201341: "Tara (G5)",
        201423: "Valkyrie (G3)",
        201523: "Jacques Rouge (G6)",
        201524: "Benedito, a Lanterna (G6)",
    },
    # 2026-10-09: TWDA unlocks, steps 10-15 (Dragonbound in the eighth
    # batch).
    {
        100114: "Auspex",
        100408: "Conquer the Beast",
        100654: "Ephor",
        101031: "Justicar Retribution",
        101530: "Quick Meld",
        101781: "Signet of King Saul, The",
        101822: "Society of Leopold",
        102012: "Trap",
        102088: "Vampiric Disease",
        200034: "Aksinya Daclau (G4)",
        200373: "Dr. Allan Woodstock (G3)",
        200455: "Fakir al Sidi (G4)",
        200508: "Gerald Windham (G5)",
        200521: "Giovanni del Georgio (G5)",
        200533: "Graham Gottesman (G5)",
        200638: "Inyanga (G4)",
        200848: "Lodin (Olaf Holte) (G5)",
        200946: "Mary Anne Blaire (G5)",
        200956: "Matasuntha (G5)",
        201029: "Nadima (G3)",
        201160: "Randall (G4)",
        201334: "T.J. (G4)",
        201438: "Victor Donaldson (G5)",
        201621: "Karl Schrekt (G6)",
    },
    # 2026-10-09: TWDA unlocks, steps 16-20.
    {
        100280: "Call the Great Beast",
        100324: "Changeling",
        100400: "Condemnation: Mute",
        100450: "Crocodile's Tongue",
        100656: "Erciyes Fragments, The",
        100657: "Erebus Mask",
        100947: "I am Legion",
        101260: "My Kin Against the World",
        101402: "Plasmic Form",
        101681: "Sargon Fragment, The",
        101832: "Soul Gem of Etrius",
        101882: "Street Cred",
        102062: "Under My Skin",
        102076: "Unleash Hell's Fury",
        200179: "Beast, The Leatherface of Detroit (G2)",
        200242: "The Capuchin (G5)",
        200705: "Jimmy Dunn (G2)",
        200856: "Lord Tremere (G5)",
        200905: "Malgorzata (G4)",
        200999: "Mistress Fanchon (G4)",
        201048: "Nergal (G5)",
        201362: "Theo Bell (G2)",
        201698: "Juliet Parr (G7)",
    },
    # 2026-10-11: TWDA unlocks, round 2 steps 1-6 (owner: Nephandus/Tupdog,
    # The unnamed, Lutz Inner Circle, Goratrix, the Toreador G1/G2 wall, the
    # Ventrue grinder); Herald of Topheth, a demon for Veneficti (owner).
    {
        100290: "Canine Horde",
        100418: "Contagion",
        100426: "Corporate Hunting Ground",
        100446: "CrimethInc.",
        101112: "Liquidation",
        101199: "Memory Rift",
        101272: "Nephandus",
        101431: "Powerbase: Barranquilla",
        101508: "Psychic Assault",
        101658: "Ruins of Ceoris",
        100912: "Herald of Topheth",
        101718: "Sense the Sin",
        102079: "The Unmasking",
        102103: "Veneficti",
        200011: "Adana de Sforza (G4)",
        200049: "Alexandra (G2)",
        200106: "Anneke (G1)",
        200107: "Anson (G1)",
        200116: "Antonio d'Erlette (G4)",
        200204: "Blackhorse Tanner (G3)",
        200339: "Devin Bisley (G2)",
        200418: "Ember Wright (G3)",
        200478: "François Villon (G2)",
        200528: "Goratrix (G2)",
        200686: "Jefferson Foster (G4)",
        200767: "Keith Moody (G4)",
        200890: "Lutz von Hohenzollern (G4)",
        200910: "Marcellus (G2)",
        201046: "Neighbor John (G4)",
        201079: "Omaya (G2)",
        201087: "Owain Evans, The Wanderer (G3)",
        201151: "Rafael de Corazon (G4)",
        201222: "Saiz (G3)",
        201393: "Tupdog (G3)",
        201410: "Unmada (G5)",
        201411: "The unnamed (G6)",
        201444: "Victoria Ash (G2)",
    },
    # 2026-10-11: owner's list, round 3 (cards gating further TWDA decks).
    {
        100003: "Aaron's Feeding Razor",
        100004: "Abactor",
        100009: "Abombwe",
        100040: "Alpha Glint",
        100128: "Baltimore Purge",
        100149: "Behind You!",
        100392: "Concealed Weapon",
        100406: "Confusion of the Eye",
        100529: "Desert Eagle",
        100753: "Focus the Blood",
        100855: "Gregory Winter",
        100938: "Hospital Food",
        100955: "Illusory Resources",
        100980: "Infernal Pursuit",
        101080: "Learjet",
        101482: "Preternatural Evasion",
        101602: "Renegade Garou",
        101603: "Renewed Vigor",
        101859: "Staredown",
        101995: "Torrent",
        200062: "Alu (G5)",
        200239: "Cameron (G2)",
        200595: "Hester Reed (G3)",
        200842: "Lisette Vizquel (G2)",
        201472: "Wise Frog (G6)",
    },
    # 2026-10-11: owner's list, round 4 (Gangrel vampires).
    {
        200131: "Ariadne (G5)",
        200547: "Gunnar (G4)",
        200663: "Jacob Fermor (G4)",
    },
    # 2026-10-11: owner's list, round 5 (the Gangrel decks' missing cards).
    {
        100050: "Anachronism",
        100237: "Bone Spur",
        101062: "Kiss of Ra, The",
        101214: "Mind of the Wilds",
        101333: "Ossian",
        101363: "Path of Lilith, The",
        101479: "Predator's Transformation",
        101559: "Read the Winds",
        102208: "Zip Gun",
        200836: "Lillian (G5)",
        201192: "Robert Price (G4)",
    },
    # 2026-10-11: owner's list, round 6.
    {
        201335: "Tabitha Fisk (G3)",
        101270: "Neighborhood Watch Commander",
        101507: "Psyche!",
        200330: "Delilah Easton (G1)",
        200642: "Isabel de Leon (G2)",
        201347: "Tatiana Romanov (G1)",
    },
    # 2026-10-12: round 7 (slice 16s), the next hundred by TWDA importance.
    {
        200014: "Adonai (G2)",
        100113: "Aura Reading",
        100133: "Banshee Ironwail",
        200291: "Cornelius Ottavio (G2)",
        201089: "Ozmo (G1)",
        101404: "Poacher's Hunting Ground",
        201146: "Quira, The Bitch Queen (G2)",
        201280: "Sheila Mezarin (G2)",
        101810: "Smear Campaign",
        200158: "Babalawo Alafin (G4)",
        100249: "Breath of Thanatos",
        100552: "Disciple of Lazarus",
        100634: "Emerald Legionnaire",
        200428: "Erebus (G3)",
        200434: "Erlik (G3)",
        200992: "Mina Grotius (G3)",
        201057: "Nicomedes (G4)",
        201305: "Solomon Batanea (G4)",
        102026: "Trochomancy",
        200135: "Arika (G2)",
        200256: "Catherine du Bois (G3)",
        200824: "Le Dinh Tho (G2)",
        200873: "Lucinde, Alastor (G3)",
        200915: "Marcus Vitel (G3)",
        201139: "Queen Anne (G2)",
        200167: "The Baron (G2)",
        200185: "Beetleman (G4)",
        200259: "Céleste Lamontagne (G4)",
        200404: "Effie Lowery (G4)",
        200468: "Fish (G4)",
        200879: "Ludmijla Rakoczy (G5)",
        201116: "Petra (G4)",
        201519: "Rurik Rakoczy (G5)",
        201220: "Sahana (G4)",
        201318: "Stick (G4)",
        201494: "Yuri Kerezenski (G4)",
        100378: "Coma",
        100405: "Confusion",
        100682: "Eyes of Chaos",
        101055: "Kindred Spirits",
        101216: "Mind Tricks",
        201010: "Morel (G4)",
        201085: "Osric Vladislav (G5)",
        201110: "Persephone Tar-Anis (G4)",
        200010: "Accorri Giovanni (G5)",
        100019: "Acquired Ventrue Assets",
        200059: "Almodo Giovanni (G4)",
        100279: "Call of the Hungry Dead",
        200344: "Diego Giovanni (G5)",
        200363: "Don Michael Antonio Giovanni (G4)",
        200366: "Donatello Giovanni (G4)",
        200430: "Eric Milliner (G5)",
        100693: "Fake Out",
        200515: "Gianmaria Giovanni (G5)",
        201068: "Nunzio Giovanni (G4)",
        101433: "Powerbase: Cape Verde",
        201134: "Primo Giovanni (G4)",
        201165: "Raphaela Giovanni (G4)",
        101843: "Spectral Divination",
        101851: "Spiritual Guidance",
        101895: "Sudario Refraction",
        201369: "Thomasso Ghiberti (G4)",
        101983: "Thrown Sewer Lid",
        200080: "Andre LeRoux (G5)",
        200200: "Black Annis (G4)",
        100178: "Bleeding the Vine",
        200424: "Enkidu, The Noah (G4)",
        101018: "Jackie Therman",
        201052: "New Blood (ANY)",
        101736: "Shadow Boxing",
        100207: "Blood Puppy",
        200441: "Eugene (G4)",
        200586: "Helena (G3)",
        200639: "Ira Rivers (G3)",
        200708: "Joaquin Murietta (G3)",
        200779: "Khalu (G3)",
        101210: "Millicent Smith, Puritan Vampire Hunter",
        101531: "Quicken Sight",
        101779: "Sideslip",
        201496: "Yvette, The Hopeless (G3)",
        200008: "Abiku (G4)",
        100136: "Base Hunting Ground",
        200262: "Cesewayo (G4)",
        200600: "Homa (G4)",
        200954: "Massassi (G4)",
        101189: "Mbare Market, Harare",
        201349: "Tatu Sawosa (G4)",
        201419: "Uzoma (G4)",
        200122: "Apache Jones (G4)",
        200254: "Cassandra Langely, The Waif (G4)",
        100521: "Dementation",
        100524: "Deny",
        200400: "Eddie Gaines (G3)",
        100616: "Effective Management",
        200658: "Jackie (G4)",
        200941: "Marta (G4)",
        200985: "Midget (G3)",
        201111: "Persia, The Beautiful Statue (G3)",
        100163: "The Bitter and Sweet Story",
        201408: "Uncle George (G3)",
    },
    # Round 8 (slice 16t): Blood Brother circles, the ten Blood Brother TWDA
    # decks in unlock order (owner, 2026-10-12).
    {
        100196: "Blood Buff",
        100283: "Camaraderie",
        100352: "Claiming the Body",
        200592: "Hermana Hambrienta Mayor (G4)",
        200593: "Hermana Hambrienta Menor (G4)",
        100931: "Hive Mind",
        101764: "Shell Game",
        101797: "Slake the Thirst",
        102138: "Walk of Caine",
        102206: "Zillah's Tears",
        200628: "Ilse (G2)",
        200652: "Jack (G2)",
        200755: "Karl (G2)",
        200936: "Mark (G2)",
        101314: "Octopod",
        101327: "Oppugnant Night",
        101979: "Three's a Crowd",
        201389: "Truman (G2)",
        100369: "Coagulated Entity",
        100423: "Coordinate Attacks",
        102082: "Unwholesome Bond",
        100046: "Ambush",
        100815: "Gestalt",
        101120: "London Evening Star, Tabloid Newspaper",
        101680: "Sanguine Instruction",
        200319: "Danylo (G5)",
        100493: "Dark Influences",
        200497: "Gavrylo (G5)",
        200799: "Kyrylo (G5)",
        101119: "Loki's Gift",
        201105: "Pavlo (G5)",
        101491: "Procurer",
        201459: "Volo (G5)",
        200170: "Barry (G2)",
        200697: "Jerry (G2)",
        200814: "Larry (G2)",
        101145: "Major Boon",
        101539: "Rampage",
        101791: "Skin of Rock",
        201359: "Terry (G2)",
        201378: "Tom (G2)",
        200370: "Dorka (G6)",
        200487: "Gabi (G6)",
        101240: "Monocle of Clarity",
        101579: "Redistribution",
        101644: "Robert Carter",
        201453: "Viri (G6)",
        201508: "Zizi (G6)",
        200021: "Ági (G6)",
        100250: "Breath of the Dragon",
        100344: "Chiropteran Marauder",
        102023: "Tribute to the Master",
        100212: "Blood Tears of Kephran",
        100394: "Concoction of Vitality",
        101305: "NRA PAC",
        100774: "Fortitude",
        100934: "Horrid Form",
        100986: "Inner Essence",
        101792: "Skin of Steel",
        102116: "Vicissitude",
    },
    # Round 9 (slice 16u): Shadow Play, The Embrace and the Appolonius merge:
    # the Lasombra, Toreador antitribu and Ravnos TWDA decks and Popó (owner,
    # 2026-10-12).
    {
        101743: "Shadow Play",
        200063: "Alvaro, The Scion of Angelica (G2)",
        200070: "Ambrosio Luis Monçada, Plenipotentiary (G2)",
        200117: "Antonio Delgado (G2)",
        200476: "Francisco Domingo de Polonia (G2)",
        200534: "Gratiano (G2)",
        200092: "Angelica, The Canonicus (G2)",
        200152: "Ayelea, The Manipulator (G2)",
        101207: "Metro Underground",
        101774: "Shroud of Absence",
        101775: "Shroud of Night",
        200622: "Ignacio, The Black Priest (G2)",
        200913: "Marciana Giovanni, Investigator (G2)",
        201006: "Montano (G3)",
        201027: "Mustafa Rahman (G2)",
        201157: "Ramiro (G2)",
        201208: "Royce (G2)",
        101957: "Tenebrous Form",
        100177: "Blanket of Night",
        100871: "Gurchon Hall",
        101334: "Oubliette",
        101735: "Shadow Body",
        100633: "The Embrace",
        102165: "Web of Knives Recruit",
        100099: "Art Scam",
        200251: "Carter (G2)",
        100724: "Fetish Club Hunting Ground",
        200727: "Jost Werner (G2)",
        200849: "Lolita (G2)",
        200978: "Mercy, Knight Inquisitor (G2)",
        201055: "Nicholas Chang (G3)",
        101346: "Palla Grande",
        201178: "Remilliard, Devout Crusader (G2)",
        201509: "Zöe (G2)",
        200098: "Anka, Priestess of Thorns (G2)",
        201620: "Ashlesha (G2)",
        100831: "Gird Minions",
        200819: "Lázár Dobrescu (G2)",
        101248: "Mozambique Allure",
        201224: "Salbatore Bokkengro (G2)",
        201246: "Sasha Miklos (G2)",
        201264: "Sennuwy, The Promoter (G2)",
        201311: "Spleen, Georgio Gutterpunk (G2)",
        101904: "Sunset Strip, Hollywood",
        100622: "The Eldest Are Kholo",
        201432: "Vedel Esbreno (G2)",
        201507: "Zip (G3)",
        200084: "Andrei Puxon (G2)",
        200513: "Ghivran Dalaal, The Dead God (G2)",
        200778: "Khalil Ravana (G2)",
        100478: "Cryptic Rider",
        100115: "Autarkis Persecution",
        101406: "Poison Pill",
        200125: "Appolonius (G1)",
        200126: "Appolonius (G1 ADV)",
        100232: "Bollix",
        100740: "Flames of Insurrection",
        101085: "Legacy",
        201187: "Richter, The Templar of Du Mont (G2)",
    },
    # Round 10 (slice 16v): the 2025-2026 TWDA decks the version cap keeps
    # (Summon the Abyss, Juliet's Dream, Arika, Cardinal Cougars, The Evil
    # Marko Experience, Turbo Unnamed, Lutz goes Meddling, ...) (owner,
    # 2026-10-13).
    {
        102203: "Zaire River Ferry",
        101663: "Rumors of Gehenna",
        101036: "Kaymakli Nightmares",
        201693: "Annabelle Triabell (G6)",
        101500: "Protected Resources",
        100605: "Eat the Rich",
        101330: "Orgy of Blood",
        100320: "Chameleon",
        101429: "Power of One",
        200501: "Genevieve (G2)",
        200321: "Dark Selina (G5)",
        101973: "Third Tradition: Progeny",
        100585: "Drawing Out the Beast",
        101282: "Ni Dieu ni Maître",
        100022: "Adhocracy",
        101192: "The Meddling of Semsith",
        201313: "Stavros (G4)",
        200389: "Drozodny (G5)",
        100285: "Camarilla Threat",
        200568: "Hardestadt (G4)",
        200900: "Malabranca (G4)",
        100936: "Horrific Countenance",
        100078: "Approximation of Loyalty",
        100073: "Antediluvian Awakening",
        101577: "Redeem the Lost Soul",
        100981: "Infernal Servitor",
        102134: "Vulnerability",
        100704: "Fast Hands",
        100060: "Anathema",
        201697: "Ian Carfax (G7)",
        100071: "The Ankara Citadel, Turkey",
        101169: "Marked Path",
        201619: "Victoria Ash (G7)",
    },
    # Round 11 (slice 16w): the cards that bring the most 2025-2026 TWDA
    # decks into the kept corpus (owner, 2026-10-14: "we focus on recent
    # TWDA entries that add to our corpus").
    {
        101206: "Mesu Bedshet",
        101053: "Kindred Segregation",
        101091: "Legendary Vampire",
        101496: "Propaganda of the Deed",
        100516: "Deer Rifle",
        100861: "Groundfighting",
        101863: "The Status Perfectus",
        200720: "Josef von Bauren (G4)",
        101396: "Petra Resonance",
        102136: "Waiting Game",
        101201: "Mental Maze",
        101136: "Maabara",
        200489: "Gabrielle di Righetti (G5)",
        100053: "An Anarch Manifesto",
        100790: "Free States Rant",
        101538: "Raking Talons",
        100252: "Brick by Brick",
        100775: "Fortschritt Library",
        200571: "Harold Tanner (G4)",
        200549: "Gustaphe Brunnelle (G4)",
        201054: "Nichodemus (G5)",
        101296: "Nose of the Hound",
        101947: "Taunt the Caged Beast",
        100159: "Big Game",
        200520: "Giotto Verducci (G2)",
        200826: "Leandro (G2)",
        101035: "Kaymakli Fragment",
        201049: "Nergal (G5 ADV)",
        101386: "Perfect Clarity",
        101089: "Legal Manipulations",
        101998: "Touch of Clarity",
        100098: "The Art of Pain",
        201435: "Venere Carboni (G3)",
        200802: "Lachlan, Noddist (G2)",
        200699: "Jessica (G2)",
    },
    # Round 12 (slice 16x): the next cards that bring 2025-2026 TWDA decks
    # into the kept corpus (owner, 2026-10-14: "go for steps 1–7").
    {
        200145: "Asguresh (G5)",
        100355: "Clandestine Contract",
        100886: "Harass",
        201080: "Omer (G6)",
        200959: "Mateusz Kazimirski, Biker of Blood (G6)",
        100513: "Deed the Heart's Desire",
        200866: "Luc (G4)",
        101124: "Loss",
        100562: "Dive into Madness",
        201370: "Thucimia (G4)",
        200046: "Alessandro Garcia (G4)",
        100239: "The Book of Going Forth by Night",
        201104: "Paulo de Castille (G4)",
        200435: "Ermenegildo, The Rake (G4)",
        200829: "Leila Monroe (G4)",
        101345: "Palatial Estate",
        100739: "Flak Jacket",
        201700: "Lucinde, Alastor (G7)",
        101685: "Sawed-Off Shotgun",
        101013: "Island of Yiaros",
        100868: "Guardian Vigil",
        100813: "Gear Up",
        100016: "Acheron Vortex",
        100628: "Eluding the Arms of Morpheus",
        100566: "Divine Sign",
        201248: "Saulot, The Wanderer (G4)",
        100066: "Anesthetic Touch",
        101606: "Repulsion",
        201398: "Tyler McGill (G5)",
        100094: "Arson",
        101925: "Tainted Spring",
        200807: "Laika (G4)",
        201482: "Xipe Totec (G5)",
        101131: "Lucky Blow",
        101515: "Pulled Fangs",
        200013: "Adhiambo (G4)",
    },
    # Round 13 (slice 16y): the 2019-2024 products completed set by set
    # (First Blood, Sabbat Preconstructed, Twenty-Fifth and Thirtieth
    # Anniversary) and the TWDA greedy's first step (owner, 2026-10-15:
    # "(a) + (b) first 7 yes"; the step is 6 cards by id).
    {
        100018: "Aching Beauty",
        100038: "Alastor",
        100070: "Animalism",
        100169: "Black Metamorphosis",
        100286: "Camarilla Vitae Slave",
        100335: "Charnas the Imp",
        100390: "Computer Hacking",
        100651: "Entombment",
        100770: "Form of Corruption",
        100779: "Foundation Exhibit",
        100800: "Game of Malkav",
        100882: "Hand of Conrad",
        101048: "Khobar Towers, Al-Khubar",
        101235: "Molotov Cocktail",
        101931: "Talbot's Chainsaw",
        101963: "Thadius Zho",
        102052: "Two Wrongs",
        102075: "University Hunting Ground",
        102268: "Banu Haqim Justicar",
        102272: "Lasombra Justicar",
        200025: "Aidan Lyle (G5)",
        200114: "Antón de Concepción (G4)",
        200144: "Arthur Denholm (G5)",
        200186: "Bela (G5)",
        200235: "Calebros, The Martyr (G2)",
        200249: "Carolina Vález (G5)",
        200280: "Claus Wegener (G5)",
        200381: "Dr. Morrow, The Skindoctor (G5)",
        200391: "Duality (G4)",
        200421: "Emily Carson (G5)",
        200427: "Epikasta Rigatos (G4)",
        200485: "Frondator (G4)",
        200498: "Gem Ghastly (G5)",
        200555: "Gwendolyn Fleming (G5)",
        200627: "Ilias cel Frumos (G4)",
        200674: "Janine (G4)",
        200696: "Jeremy \"Wix\" Wyzchovsky (G4)",
        200707: "João Bilé (G4)",
        200717: "Jonathan Gursel (G4)",
        200740: "Justine Chen, Innocent (G4)",
        200763: "Kateline Nadasdy (G4)",
        200787: "Klaus Konrecht (G4)",
        200800: "L'Épuisette (G4)",
        200827: "Lectora (G4)",
        200851: "Loonar (G4)",
        200867: "Luca Italicus (G4)",
        201005: "Monique Kim (G4)",
        201013: "Mosfair (G4)",
        201020: "Muhsin Samir (G4)",
        201022: "Murat (G2)",
        201026: "Mustafa, The Heir (G4)",
        201060: "Nikolaus Vermeulen (G2)",
        201213: "Rutor (G5)",
        201262: "Selma the Repugnant (G1)",
        201281: "Sheldon, Lord of the Clog (G1)",
        201366: "Thomas De Lutrius (G4)",
        201388: "Troius (G4)",
        201407: "Una (G3)",
        201430: "Vasily (G5)",
        201501: "Zane (G5)",
        201512: "America Johnson (G5)",
        201513: "Charles Delmare (G5)",
        201514: "Conchita Morales (G5)",
        201515: "Guggenheim (G5)",
        201516: "Lady Vadislava (G5)",
        201517: "Lord Leopold Valdemar (G5)",
        201518: "Percival (G5)",
        201522: "Grimgroth (G6)",
        201695: "Dónal O'Connor (G6)",
        201704: "Molly MacDonald (G7)",
        201705: "Nikolaus Vermeulen (G6)",
    },
    # Round 14 (slice 16z): the 2018 Humble Bundle, 2015 Storyline Rewards
    # and Danse Macabre completed, and the TWDA greedy's steps 1-8 (owner,
    # 2026-10-16: "Steps 1-8 + 3 sets").
    {
        100002: "419 Operation",
        100325: "Changeling Skin Mask",
        100495: "Dark Steel",
        100522: "Demonstration",
        100663: "Esprit de Corps",
        100723: "Festivo dello Estinto",
        100990: "Inside Dirt",
        101170: "Marked Territory",
        101332: "Osebo Kholo",
        101436: "Powerbase: Luanda",
        101514: "Public Vilification",
        101570: "Recruitment Exercise",
        101633: "Rewind Time",
        101638: "Rise of the Fallen",
        101679: "Sanguine Entrapment",
        101749: "Shakar: the Hunt",
        101854: "Spoils of War",
        101897: "Summon History",
        101905: "Superior Mettle",
        101924: "Tabriz Assembly",
        101935: "Tangle Atropos' Hand",
        101944: "Taste of Death",
        200027: "Aimery Methuin (G5)",
        200047: "Alex Camille (G5)",
        200067: "Amber (G5)",
        200086: "Andrew Stuart (G4)",
        200157: "Baba Yaga, the Iron Hag (G5)",
        200160: "Badr, Shadow of Granada (G5)",
        200222: "Bruce de Guy (G4)",
        200227: "Bulscu (G5 ADV)",
        200248: "Carna, The Princess Witch (G3)",
        200277: "Claudia (G5)",
        200279: "Claudio Severino (G5 ADV)",
        200312: "Dan Murdock (G1)",
        200327: "De Sade (G5)",
        200346: "Dimple (G1)",
        200378: "Dr. Julius Sutphen (G3)",
        200385: "The Dracon (G5)",
        200442: "Eugenio Estevez (G3)",
        200507: "Gerald FitzGerald (G5)",
        200527: "Gold Pan Dan (G5)",
        200621: "Idrissa (G4)",
        200714: "Johannes Worringen (G5)",
        200754: "Karif al Numair (G5)",
        200797: "Kyle Strathcona, Cardinal of Canada (G2)",
        200803: "Ladislas Toth, The Torch (G3)",
        200823: "Lazverinus, Thrall of Lambach (G2)",
        200904: "Maldavis (G3 ADV)",
        200943: "Martin Franckel (G3)",
        200951: "Masika (G1 ADV)",
        200974: "Mélusine d'Anjou (G5)",
        200991: "Mimir (G5)",
        201001: "Mithras (G3)",
        201044: "Nehemiah (G4)",
        201084: "Orlando Oriundus (G4)",
        201100: "Paul Cordwood (G4)",
        201106: "Pearl (G5)",
        201128: "Polly Kay Fisher (G4)",
        201140: "Quentin (G2)",
        201191: "Righteous Endeavor (G5)",
        201261: "Selena (G3)",
        201297: "Skulk (G5)",
        201320: "Strohmann (G5)",
        201322: "Suhailah (G2)",
        201332: "Synner-G (G5)",
        201373: "Titi Camara (G5)",
        201424: "Valois Sang, The Watcher (G3)",
        201456: "Vlad Tepes (G3)",
        201471: "Winterlich (G6)",
    },
    # Round 15 (slice 16aa): the 2023-07-01 Promo set completed and the TWDA
    # greedy's steps 1-8 (owner, 2026-10-16: "Steps 1-8 + Promo").
    {
        100082: "Arcanum Chapterhouse, Alexandria",
        100272: "Burnt Offerings",
        100425: "Corporal Reservoir",
        100483: "Curse of Nitocris",
        100527: "Derange",
        100528: "Descent into Darkness",
        100542: "Diabolic Lure",
        100571: "Domain of Evernight",
        100592: "Dual Form",
        100643: "Engling Fury",
        100678: "Eye of Hazimel",
        100752: "Flurry of Action",
        100877: "Hag's Wrinkles",
        101001: "Into Thin Air",
        101139: "Madness Network",
        101150: "Malkavian Dementia",
        101161: "Mantle of the Bestial Majesty",
        101262: "Mythic Form",
        101338: "Outside the Hourglass",
        101405: "Pocket Out of Time",
        101888: "Stunt Cycle",
        101928: "Taking the Skin: Minion",
        101946: "Tattoo Signal",
        101982: "Thrown Gate",
        102053: "Tye Cooper",
        102054: "Typhonic Beast",
        200005: "Abdelsobek (G5)",
        200124: "Appius Claudius Corvus (G5)",
        200208: "Bloodfeud (G4)",
        200261: "César Holfield (G4)",
        200305: "Cybele (G4)",
        200336: "Denette Stensen (G4)",
        200365: "Donald Cargill (G5)",
        200396: "Earl (G3)",
        200403: "Edward Vignes (G3)",
        200462: "Feo Ramos (G3)",
        200477: "Franciscus (G2)",
        200480: "Frank Litzpar (G4)",
        200512: "Ghede (G6)",
        200587: "Helena (G3 ADV)",
        200641: "Iris Bennett (G5)",
        200660: "Jackson Asher (G4)",
        200691: "Jephta Hester (G4)",
        200738: "Juniper (G4)",
        200764: "Katherine Stoddard (G3)",
        200812: "Lana Butcher (G3)",
        200865: "Lubomira Hradok (G5)",
        201074: "Old Neddacka (G4)",
        201082: "Onaedo (G4)",
        201247: "Saul Meira (G5)",
        201274: "Shalmath (G6)",
        201295: "Skidmark (G4)",
        201462: "Walker Grimes (G5)",
        201525: "Bituin (G6)",
        201528: "Bolesław Gutowski (G6)",
        201654: "Tegyrius, Vizier (G6)",
    },
    # 2026-10-17: the contrast to "during" (owner, 2026-10-17: "you can add
    # Maris to the pool while we're at it").
    {
        200935: "Maris Streck (G3)",
    },
    # Round 16 (slice 16ab): the TWDA greedy's steps 1-6 and five promo sets
    # completed (owner, 2026-10-18: "Steps 1-6 + 5 promo sets").
    {
        100428: "Corpse Minion",
        100510: "Death Seeker",
        100511: "Decapitate",
        100705: "Fast Reaction",
        100716: "Fee Stake: Perth",
        100847: "Gran Madre di Dio, Italy",
        100996: "Institution Hunting Ground",
        101007: "IR Goggles",
        101050: "Kindred Intelligence",
        101106: "Light Intensifying Goggles",
        101156: "Malkavian Rider Clause",
        101283: "Night Moves",
        101297: "Nosferatu Bestial",
        101300: "Nosferatu Kingdom",
        101357: "Patagia: Flaps Allowing Limited Flight",
        101419: "Ponticulus",
        101635: "Righteous Aura",
        101661: "Rumble",
        101703: "Seal of Veddartha",
        101716: "Sense Death",
        101809: "Smash and Grab",
        102020: "Tremere Convocation",
        102093: "Vaticination",
        102128: "Vivienne Géroux",
        102197: "Wrong and Crosswise",
        102235: "Dark Web Procurer",
        102307: "Praxis Seizure: York",
        200002: "Aaron Bathurst (G4)",
        200031: "Aisling Sturbridge (G2)",
        200058: "Almiro Suarez (G3)",
        200207: "Blister (G4)",
        200210: "Blythe Candeleria (G2)",
        200355: "Dolphin Black (G2)",
        200383: "Dr. Solomon Grey (G5)",
        200500: "General Perfidio Díos (G3)",
        200567: "Hannigan (G2)",
        200758: "Karsh (G3 ADV)",
        200786: "Kite (G2)",
        200791: "Korah (G2)",
        200927: "Marie Faucigny (G3)",
        200952: "Masika St. John (G3)",
        200976: "Menele (G3 ADV)",
        201003: "Monica Chang (G5)",
        201138: "Qawiyya el-Ghaduba (G5)",
        201186: "Richard Tauber, Ayelea's Puppet (G2)",
        201196: "Rodolfo (G3)",
        201486: "Yazid Tamari (G3 ADV)",
        201520: "Nefertiti (G2 ADV)",
        201616: "Fiorenza Savona (G6)",
        201618: "Okulos (G7)",
        201622: "Len Konopka (G3)",
    },
    # Round 17 (slice 16ac): twenty promo sets completed and the TWDA greedy's
    # steps 1-6, bundles of 12 (owner, 2026-10-18: "Promos + steps 1-6").
    {
        100041: "Alvusia",
        100086: "Argent Baton",
        100102: "As the Crow",
        100171: "Black Sunrise",
        100241: "Botched Move",
        100282: "Call the Wild Hunt",
        100315: "Centralized Background Check",
        100376: "Cold Aura",
        100381: "Command of the Beast",
        100445: "Crematorium",
        100489: "Dam the Heart's River",
        100548: "Dis Pater",
        100561: "Dive Bomb",
        100604: "Earthshock",
        100649: "Ensemble",
        100728: "Filchware's Pawn Shop",
        100823: "Gianna di Canneto",
        100832: "Giulia Giovanni Abruzzina",
        100869: "Guide and Mentor",
        100880: "Hand Contract",
        100971: "Increased Strength",
        100975: "Infamous Insurgent",
        101072: "Lapse",
        101078: "Leadership Vacuum",
        101092: "Legion",
        101243: "Morgue Hunting Ground",
        101359: "The Path of Bone",
        101371: "The Path of the Scorched Heart",
        101420: "Port Authority",
        101494: "Promise of 1528",
        101497: "Prophecies of Gehenna",
        101503: "Provision of the Silsila",
        101512: "Public Enemy",
        101554: "Raw Recruit",
        101619: "Retribution",
        101646: "Rockheart",
        101660: "Ruins of Villers Abbey, Belgium",
        101748: "Shakar",
        101852: "Spiritual Intervention",
        101855: "Spontaneous Power",
        101876: "Stonestrength",
        101968: "Therbold Realty",
        101970: "Thin Blood",
        101976: "Thoughts Betrayed",
        102162: "Waxen Poetica",
        200020: "Agatha (G2)",
        200041: "Alan Sovereign (G3 ADV)",
        200094: "Angelo (G3)",
        200119: "Anu Diptinatpa (G6)",
        200164: "Baldesar Rossellini (G4)",
        200166: "Barbaro Lucchese (G3)",
        200278: "Claudio Severino (G5)",
        200348: "Djuhah, The Bronze Bow (G5)",
        200382: "Dr. Oliver Genet (G6)",
        200399: "Echo (G4)",
        200459: "Fatima al-Faqadi (G2)",
        200504: "Genina, The Red Poet (G3)",
        200545: "Guillaume Giovanni (G4)",
        200566: "Hannibal (G2 ADV)",
        200594: "Hesha Ruhadze (G2)",
        200700: "Jessica (G2 ADV)",
        200710: "Joe \"Boot\" Hill (G4)",
        200775: "Kestrelle Hayes (G5)",
        200785: "Kisha Bhimji (G3)",
        200811: "Lambach (G2 ADV)",
        200874: "Lucita (G2)",
        200924: "Mariana Gilbert (G1)",
        200944: "Martino della Passaglia (G6)",
        201000: "Mitchell, The Headhunter (G2)",
        201067: "Nu, The Pillar (G2)",
        201130: "Porphyry (G6)",
        201180: "Reverend Adams (G4)",
        201185: "Reza Fatir, The Dark Angel (G4)",
        201227: "Salvatore Giovanni (G6)",
        201244: "Sascha Vykos, The Angel of Caine (G2)",
        201383: "Tori Longwood (G2)",
        201436: "Verbruch (G5)",
        201440: "Victor Pelletier (G4)",
        201467: "White Lily (G4)",
        201477: "Xaviar (G3)",
        201488: "Yong-Sun, Harmonist (G2)",
    },
    # Round 18 (slice 16ad): the cards players asked for (/admin/unsupported,
    # owner via Z-main 2026-10-19: "make them your priority") and the TWDA
    # greedy's steps 1-3 from them, bundles of 12 (owner, 2026-10-19:
    # "Asked + steps 1-3").
    {
        100023: "The Admonitions",
        100385: "Communal Haven: Cathedral",
        100403: "Conflagration",
        100441: "Creation Rites",
        100457: "Crusade: Chicago",
        100463: "Crusade: Istanbul",
        100464: "Crusade: London",
        100567: "Dodge",
        100666: "Eternal Vigilance",
        100891: "Hatch the Viper",
        100909: "Helicopter",
        100916: "Hidden Lurker",
        100964: "Improvised Flamethrower",
        101098: "Leverage",
        101299: "Nosferatu Justicar",
        101306: "NSA Trio",
        101672: "Sacrificial Lamb",
        101708: "The Secret Library of Alexandria",
        101753: "Shanty Town Hunting Ground",
        101877: "Storage Annex",
        200112: "Antoinette DuChamp (G2)",
        200211: "Bobby Lemon (G3)",
        200265: "Chandler Hungerford (G2)",
        200273: "Christine Boscacci (G2)",
        200443: "Eurayle Gelasia Mylonas (G4)",
        200463: "Fergus Alexander (G5)",
        200536: "Greensleeves (G4)",
        200737: "Julius (G2)",
        200880: "Lukas (G4)",
        200994: "Mirembe Kabbada (G2)",
        201039: "Navar McClaren (G1)",
        201073: "Ohanna (G2)",
        201231: "Samson (G2)",
        201301: "Smudge the Ignored (G1)",
    },
    # Round 19 (slice 16ae): the Imbued, their powers and convictions (owner,
    # 2026-10-14: in "every card"; 2026-10-21: "let's do the imbued").
    {
        100007: "Abjure",
        100322: "Champion",
        100551: "Discern",
        100575: "Donate",
        100764: "Foresee",
        100919: "Hide",
        100953: "Illuminate",
        100991: "Inspire",
        101492: "Project",
        101557: "React with Conviction",
        101592: "Rejuvenate",
        101611: "Respire",
        101705: "Second Sight",
        101752: "Shame",
        101886: "Strike with Conviction",
        101908: "Surge",
        102120: "Vigilance",
        200102: "Anna \"Dictatrix11\" Suljic (G4)",
        200181: "Béatrice \"Oracle171\" Tremblay (G4)",
        200397: "Earl \"Shaka74\" Deams (G4)",
        200432: "Erick \"Shophet125\" Franco (G4)",
        200479: "François \"Warden\" Loehr (G4)",
        200632: "Inez \"Nurse216\" Villagrande (G4)",
        200656: "Jack \"Hannibal137\" Harmon (G4)",
        200689: "Jennie \"Cassie247\" Orne (G4)",
        200690: "Jennifer \"Flame61\" Vidisania (G4)",
        200715: "John \"Cop90\" O'Malley (G4)",
        200825: "Leaf \"Potter116\" Pankowski (G4)",
        200846: "Liz \"Ticket312\" Thornton (G4)",
        200886: "Lupe \"Cabbie22\" Droin (G4)",
        200907: "Maman Boumba (G4)",
        200934: "Marion \"Teacher193\" Perks (G4)",
        201103: "Paul \"Sixofswords29\" Moreton (G4)",
        201108: "Pedro Cortez (G4)",
        201115: "Peter \"Outback295\" Rophail (G4)",
        201386: "Travis \"Traveler72\" Miller (G4)",
        201481: "Xian \"DziDzat155\" Quan (G4)",
    },
    # Round 20 (slice 16af): the cards players asked for (/admin/unsupported,
    # 2026-10-23; owner: "tell Z-add what cards players have tried to play and
    # couldn't so they can be added").
    {
        100111: "Aura Absorption",
        100306: "Catacombs",
        100326: "Chanjelin Ward",
        100689: "Fae Contortion",
        100841: "Goblinism",
        100853: "Great Symposium",
        100856: "Gremlins",
        100865: "Guarded Rubrics",
        100889: "Harvest Rites",
        100943: "Humanitas",
        101293: "Nocturn",
        101548: "Ravager",
        101826: "Song of Pan",
        101875: "Stone Travel",
        102156: "Watchtower: Four Ride Forth",
        # The two cards of VEKN's "List of allowed cards in 2-player VTES"
        # (2026-10-03) outside the pool (Z-main, 2026-10-23).
        100713: "Fee Stake: Corte",
        100714: "Fee Stake: Los Angeles",
        200085: "Andrew Emory (G4)",
        200127: "The Arcadian (G5)",
        200165: "Banjoko (G3)",
        200311: "Dame Hollerton (G4)",
        200644: "Isanwayen (G4)",
        201081: "Omme Enberbenight (G4)",
        201120: "Pherydima (G4)",
        201144: "Quincy, The Trapper (G3)",
        201161: "Randel, The Coward (G4)",
        201195: "Roderick Phillips March (G4)",
    },
    # Round 21 (slice 16ag): the last step paying in recent TWDA decks (Faerie
    # Wards and four G2 vampires) and Nights of Reckoning completed, the first
    # set-by-set round (owner, 2026-10-26: "Step 1 + Nights of Reckoning").
    {
        100067: "Angel of Berlin",
        100075: "Antithesis",
        100110: "Augur",
        100198: "Blood Cult Awareness Network",
        100234: "Bond",
        100248: "Break the Code",
        100348: "The Church of Vindicated Faith",
        100358: "Cleave",
        100474: "The Crusader Sword",
        100534: "Determine",
        100674: "Expiate",
        100690: "Faerie Wards",
        100963: "Imprison",
        100982: "Inflict",
        101115: "Living Wood Staff",
        101118: "Lock",
        101231: "Moise Kasavubu",
        101257: "Muse of Flame",
        101518: "Punish",
        101653: "The Rose Foundation",
        101812: "Smite",
        102074: "Unity",
        102119: "Vigil: The Thin Line",
        200172: "Bartholomew (G2)",
        200734: "Julia Prima (G2)",
        200760: "Kassiym Malikhair (G2)",
        200914: "Marconius (G2)",
    },
    # Round 22 (slice 16ah): Anthology and Anarch Unbound completed, the
    # decks metric at nine cards or more a recent deck (owner, 2026-10-28:
    # "Switch to sets").
    {
        100725: "Field Training",
        100952: "Illegalism",
        101275: "Netwar",
        101575: "The Red Question",
        102205: "Zephyr",
        200213: "Boss Callihan (G3 ADV)",
        200241: "Cao Nguyen (G6)",
        200308: "Daliah (G4)",
        200314: "Dancin' Dana (G1 ADV)",
        200338: "Desiree Narayan, Anarch Historian (G6)",
        200341: "Dhita Choudhair (G6)",
        200356: "Domenic Giovanni (G6)",
        200380: "Dr. Lawrence Mayhew (G5)",
        200469: "Flavia, Avenging Angel (G6)",
        200503: "Gengis (G3 ADV)",
        200640: "Irena (G5)",
        200681: "Javier Montoya (G2 ADV)",
        200688: "Jenna Cross (G4)",
        200863: "Louis Fortier (G4 ADV)",
        200921: "Marguerite Foccart (G4 ADV)",
        201121: "Philip van Vermeer IV (G2)",
        201226: "Salvador Garcia (G4)",
        201243: "Sarrasine (G2 ADV)",
        201272: "Shahara al-Rashwa (G6)",
        201326: "Sundown (G3 ADV)",
        201439: "Victor Gerard (G2)",
        201460: "Vulture (G2)",
        201499: "Zack North (G1 ADV)",
    },
    # Round 23 (slice 16ai): The Unaligned completed and the decks metric's
    # last paying step (owner, 2026-10-29: "Unaligned + 4 deck cards").
    {
        100187: "Blight",
        100407: "Conniver",
        100793: "From a Sinking Ship",
        100797: "Funeral Wake",
        100827: "Gift of Proteus",
        100879: "Hall of Mirrors",
        100967: "In Memory of the Two Lands",
        101264: "Nar-Sheptha",
        101325: "Ophidian Gaze",
        101727: "Serpent's Numbing Kiss",
        101789: "Skaald",
        101869: "Steely Tenacity",
        101911: "Suspension of Disbelief",
        102100: "Velvet Tongue",
        102170: "Weirding Stone",
        200007: "Abid bin Haji, Scholar of Mirrors (G6)",
        200015: "Adri, The Seeker (G6)",
        200036: "Al-Ashrad, Amr of Alamut (G2 ADV)",
        200142: "Art Morgan (G5)",
        200195: "Bez Dunsirn (G6)",
        200230: "C.J. (G5)",
        200294: "Count Jocalo (G6)",
        200332: "Delizbieta of the Dark Eyes (G3)",
        200384: "Dr. Sutton Gassaway (G4)",
        200414: "Elizabeth Dimitros (G3)",
        200422: "Enam (G3)",
        200426: "Enzo Giovanni, Pentex Board of Directors (G2 ADV)",
        200526: "Glycon (G3)",
        200615: "Iago Castille (G6)",
        200630: "Imanna (G6)",
        200649: "Ivan Krenyenko (G2 ADV)",
        200756: "Karmenita Yoryari (G6)",
        200828: "Legba (G6)",
        200840: "Lisandro Giovanni (G3)",
        200888: "Lupo Giovanni (G6)",
        200897: "Madeleine Giovanni (G6)",
        200902: "Malcolm (G5)",
        200997: "Missy Shillingford (G6)",
        201170: "Rebekah (G3)",
        201203: "Rosa Hernández (G4)",
        201233: "Sanjay Chakrabarti (G6)",
        201437: "Vestgeir (G6)",
    },
    # Round 24 (slice 16aj): Lost Kindred, Twilight Rebellion and Sword of
    # Caine completed (owner, 2026-10-30: "we'll skip promos, so let's go
    # with Lost Kindred, Twilight Rebellion and Sword of Caine").
    {
        100017: "Achilles' Heel",
        100072: "Anonymous Freight",
        100137: "Baseball Bat",
        100167: "Black Hand Emissary",
        100174: "Blade Clot",
        100175: "Blade of Bellona",
        100180: "Blessed Blade",
        100186: "Blessings of the Loa",
        100188: "Blind Spot",
        100216: "Blood Turnip",
        100275: "Cadet",
        100314: "Census Taker",
        100340: "Children of Stone",
        100360: "Cloak of Blood",
        100532: "Detect Authority",
        100589: "Drink the Blood of Ahriman",
        100590: "Drop Point Network",
        100607: "Eccentric Billionaire",
        100627: "Elixir of Distillation",
        100639: "Empowering the Puppet King",
        100647: "Enrage",
        100655: "Epiphany",
        100671: "Exclusion Principle",
        100691: "Failsafe",
        100730: "Final Loosening",
        100757: "Follow the Blood",
        100786: "The Framing",
        100787: "Framing an Ancient Grudge",
        100858: "Grey Thorne",
        100896: "Haven Hunting",
        100974: "Inevitability of the Void",
        101024: "Joseph Pander",
        101041: "Keystone Kine",
        101111: "Liquefy the Mortal Coil",
        101258: "Mustajib",
        101268: "Necrosis",
        101290: "No Confidence",
        101304: "Notorious Brutality",
        101428: "Power of All",
        101446: "Praetorian Backer",
        101489: "Prison of the Mind",
        101595: "Relentless Reaper",
        101678: "Sanguinary Wind",
        101724: "Seraph's Second",
        101759: "Shattering",
        101771: "Shoulder Drop",
        101866: "Stealing Years",
        101967: "Their Master's Voice",
        101999: "Touch of Pain",
        102022: "Tribunal Judgment",
        102027: "Trophy: Chosen",
        102043: "Truth in Ink",
        102070: "Unexpected Coalition",
        102077: "Unleash the Hounds",
        102107: "Vengeful Spirit",
        102112: "Vermin Channel",
        102158: "Watchtower: The Wolves Feed",
        102193: "The World's a Canvas",
        102209: "Zip Line",
        200087: "Andy (G4)",
        200093: "Angelique (G6)",
        200109: "Antoine, The Lost (G4)",
        200133: "Arianne, The Conqueror (G5)",
        200146: "Ash Harrison (G5)",
        200238: "Calvin Cleaver (G4)",
        200246: "Carmen (G5)",
        200255: "Castiel (G3)",
        200264: "Chalcedony (G6)",
        200283: "Clifton Derrik (G4)",
        200347: "Dirk (G4)",
        200454: "Fairuza (G5)",
        200496: "Gatjil Munyarryun (G5)",
        200560: "Hagar Stone (G5)",
        200563: "Handsome Dan (G6)",
        200580: "Hecate (G5)",
        200655: "Jack Drake (G4)",
        200685: "Jean Lisle (G6)",
        200781: "Khurshid (G6)",
        200806: "Laecanus (G4)",
        200808: "Lailah (G6)",
        200837: "Lin Jun (G4)",
        200862: "Louis Fortier (G4)",
        200908: "Mambo Jeanne (G6)",
        200919: "Marge Khan (G5)",
        200920: "Marguerite Foccart (G4)",
        200928: "Marie-Pierre (G6)",
        200983: "Michaela (G5)",
        200998: "Mister Schwartz (G6)",
        201032: "Nails (G4)",
        201062: "Nizzam al-Latif (G5)",
        201069: "Nuriel (G6)",
        201083: "Ondine \"Boudicca\" Sinclair (G5)",
        201133: "Preston Varrick (G4)",
        201166: "Rashid Stockton (G5)",
        201181: "Reverend Adams (G4 ADV)",
        201255: "Sean Rycek (G4)",
        201271: "Shaggydog (G5)",
        201279: "Sheela Na Gig (G5)",
        201315: "Stephen Bateson (G5)",
        201342: "Tarautas (G4)",
        201376: "Toby (G4)",
        201382: "Topaz (G4)",
        201510: "Zubeida (G5)",
    },
    # Round 25 (slice 16ak): Ebony Kingdom and Blood Shadowed Court
    # completed (owner, 2026-11-03: "Ebony Kingdom + BSC").
    {
        100120: "Aye",
        100129: "Bamba",
        100153: "Bestial Vengeance",
        100209: "Blood Shield",
        100263: "Brutal Influence",
        100530: "Despiral",
        100538: "Devil-Channel: Feet",
        100595: "Dusk Work",
        100613: "Edge of the World",
        100673: "Exile",
        100699: "Familial Bond",
        100870: "Guinea-Bissau Carnival",
        100922: "Hiding in the Open",
        100956: "Ilomba",
        100966: "Impundulu",
        101005: "Invoke Poison Glands",
        101012: "Ishtarri Warlord",
        101027: "Jua Vema",
        101069: "Kuta",
        101146: "Make an Example",
        101163: "Mapatano Utando",
        101253: "Mundane",
        101269: "Neebi",
        101289: "Nkishi",
        101331: "Orun",
        101347: "Pallid",
        101599: "Reliquary: Trinket",
        101600: "Remnant of the Endless Storm",
        101683: "Savannah Runner",
        101719: "Sense Vibrations",
        101906: "Supernatural Resistance",
        101929: "Taking the Skin: Vulture",
        101934: "Taming the Beast",
        102009: "Transcendent Laibon",
        102047: "Tunnel Runner",
        102073: "Unholy Radiance",
        102172: "Well-Marked",
        200009: "Abu Nuwasi (G4)",
        200012: "Adelaide Davis (G3)",
        200028: "Aisata Swanou (G5)",
        200040: "Alan Sovereign (G3)",
        200044: "Alejandro Aguirre (G3)",
        200051: "Alfred Benezri (G3)",
        200056: "Allison Maller (G3)",
        200061: "Alonzo Guillen (G3)",
        200072: "Amelia, The Blood Red Tears (G3)",
        200078: "Anastasz di Zagreb (G3)",
        200103: "Annabelle Triabell (G3)",
        200110: "Antoinette, She Who Watches (G3)",
        200118: "Antonio Veradas (G3)",
        200139: "Arnold Simpson (G3)",
        200141: "Arriette Sylla (G4)",
        200171: "Barth (G3)",
        200177: "Batsheva (G4 ADV)",
        200252: "Casino Reeds (G3)",
        200263: "Cesewayo (G4 ADV)",
        200284: "Cock Robin (G3)",
        200285: "Cohn Rose (G3)",
        200299: "Creamy Jade (G3)",
        200324: "Darva Felispa (G3)",
        200326: "David Morgan, The Scourge (G3)",
        200342: "Diana Vick (G3)",
        200374: "Dr. Douglas Netchurch (G3)",
        200401: "Edith Blount (G3)",
        200402: "Edward Neally (G3)",
        200406: "Ehrich Weiss (G3)",
        200407: "Elena Gutierrez (G3)",
        200413: "Elizabeth Conde (G4)",
        200417: "Ellison Humboldt (G3)",
        200420: "Emerson Wilkershire III (G3)",
        200423: "Enid Blount (G3)",
        200431: "Erichtho (G3)",
        200436: "Esau (G3)",
        200444: "Evan Klein (G3)",
        200470: "Fleurdumal (G3)",
        200472: "Fode Kourouma (G4)",
        200499: "Gemini (G3)",
        200502: "Gengis (G3)",
        200510: "Gerard Rafin (G3)",
        200532: "Gracis Nostinus (G3)",
        200604: "Horatio Ballard (G3)",
        200608: "Hrothulf (G3)",
        200629: "Ilyana Ravidovich (G3)",
        200651: "J. Oswald \"Ozzy\" Hyde-White (G3)",
        200677: "Jara Drory (G3)",
        200678: "Jaroslav Pascek (G3)",
        200679: "Jason, The World's Voice (G3)",
        200693: "Jeremy MacNeil (G3)",
        200704: "Jibade el-Bahrawi (G4 ADV)",
        200726: "Joshua Tarnopolski (G3)",
        200736: "Julio Martinez (G3)",
        200750: "Kamiri wa Itherero (G4 ADV)",
        200762: "Katarina Kornfeld (G3)",
        200776: "Khalid (G3)",
        200794: "Krid (G3)",
        200795: "Kurt Densch (G3)",
        200798: "Kyoko Shinsegawa (G3)",
        200835: "Lille Haake (G3)",
        200864: "Luanda Magere (G4)",
        200868: "Lucas Halton (G3)",
        200871: "Lucian, the Perfect (G4)",
        200872: "Lucina (G3)",
        200896: "Madame Guil (G3)",
        200906: "Mamadou Keita (G4)",
        200939: "Marlena (G3)",
        200966: "Maxwell (G3)",
        200967: "Mazz (G3)",
        200975: "Menele (G3)",
        200990: "Milo, The Invisible Horror (G3)",
        201014: "Mouse (G3)",
        201035: "Nana Buruku (G4)",
        201050: "Nestor Kaba (G4)",
        201053: "Ngozi Ekwensu (G5)",
        201076: "Oliver Thrace (G3)",
        201125: "Pieter van Dorn (G3)",
        201135: "Pug Jackson (G3)",
        201141: "Quentin King III (G3)",
        201148: "Rachel Brandywine (G3)",
        201205: "The Rose (G3)",
        201212: "Ruth McGinley (G3)",
        201257: "Sébastien Goulet (G3)",
        201289: "Sir Henry Johnson (G3)",
        201291: "Sir Ralph Hamilton (G3)",
        201303: "Socrate Cidibe (G4)",
        201317: "Steve Booth (G3)",
        201328: "Suzanne Kadim (G3)",
        201338: "Tammy Walenski (G3)",
        201350: "Tayshawn Kearns (G3)",
        201375: "Tobias Smith (G3)",
        201381: "Tony (G3)",
        201397: "Tyler (G3)",
        201406: "Umdava (G4)",
        201409: "Undele (G5)",
        201445: "Victorine Lafourcade (G3)",
        201454: "Virstania, The Great Mother (G3)",
        201469: "William Biltmore (G3)",
        201474: "Wolfgang (G3)",
    },
    # Round 26 (slice 16al): Gehenna and Anarchs completed (owner,
    # 2026-11-06: "Gehenna + Anarchs").
    {
        100027: "Ahriman's Demesne",
        100036: "Alacrity",
        100039: "Alia, God's Messenger",
        100043: "Amaranth",
        100112: "Aura of Invincibility",
        100144: "Bear-Baiting",
        100148: "Becoming of Ennoia",
        100168: "Black Hand Ritual",
        100226: "Blow Torch",
        100229: "Body Bag",
        100240: "Border Skirmish",
        100244: "Boxed In",
        100247: "Bravo",
        100251: "Bribes",
        100255: "Brinksmanship",
        100293: "Car Bomb",
        100299: "Carnivale",
        100304: "Caseless Rounds",
        100316: "Chain of Command",
        100343: "Chiram's Hold",
        100347: "Church of the Order of St. Blaise",
        100375: "Cold Amber's Hold",
        100422: "Cooler",
        100451: "Crusade: Amsterdam",
        100455: "Crusade: Berlin",
        100459: "Crusade: Dublin",
        100460: "Crusade: Frankfurt",
        100461: "Crusade: Geneva",
        100468: "Crusade: Paris",
        100473: "Crusade: Washington, D.C.",
        100475: "Cry Wolf",
        100482: "Curmudgeon",
        100504: "Dead-End Alley",
        100520: "Delivery Truck",
        100543: "The Diamond Thunderbolt",
        100564: "Diversity",
        100586: "Dread Gaze",
        100591: "Drum of Xipe Totec",
        100611: "Ecstatic Agony",
        100621: "Elder Michaelis's Hold",
        100630: "Elysium: Sforzesco Castle",
        100635: "Emergency Powers",
        100636: "Emergency Preparations",
        100638: "Emissary",
        100653: "Entrenching",
        100662: "Esgrima",
        100732: "Finding the Path",
        100736: "Firebrand",
        100745: "Flash Grenade",
        100759: "Force of Will",
        100772: "Form of the Ghost",
        100791: "Frenzy",
        100792: "Friend of Mine",
        100798: "Galaric's Legacy",
        100799: "Gambit Accepted",
        100801: "Games of Instinct",
        100812: "Gather",
        100820: "Ghouled",
        100826: "Gift of Experience",
        100840: "Go Anarch",
        100930: "Highway Haven: RV",
        100965: "Improvised Tactics",
        101000: "Into the Fire",
        101010: "Irregular Protocol",
        101016: "Jack of Both Sides",
        101101: "Liberty Club Intrigue",
        101121: "Loner",
        101122: "Loose Cannon",
        101142: "Magazine",
        101191: "Meat Hook",
        101230: "Mob Rule",
        101233: "The Mole",
        101237: "Momentum's Edge",
        101247: "Movement of the Slow Body",
        101295: "Nod",
        101313: "Occlusion",
        101336: "Out of the Frying Pan",
        101343: "Pack Tactics",
        101351: "Paragon",
        101380: "Peace Treaty",
        101394: "Personal Involvement",
        101480: "Presence",
        101486: "Principia Discordia",
        101524: "Pushing the Limit",
        101541: "Rant!",
        101543: "Rapid Healing",
        101580: "Redline",
        101582: "Reformation",
        101584: "Regarhagan's Hold",
        101605: "Repo Man",
        101622: "Revelation of Desire",
        101652: "Root of Vitality",
        101669: "Sacrament of Carnage",
        101676: "San Lorenzo de El Escorial, Spain",
        101689: "Scattershot",
        101693: "Scorpion Sting",
        101704: "Seattle Committee",
        101707: "Secret Horde",
        101725: "Serenading the Kami",
        101729: "Servitor of Irad",
        101784: "Siphon",
        101790: "Skin of Night",
        101796: "Skullduggery",
        101803: "Sleep of Reason",
        101804: "Sleep Unseen",
        101813: "Smoke and Mirrors",
        101815: "Snipe Hunt",
        101828: "Songs of the Distant Vitae",
        101838: "Sowing Dissension",
        101861: "Starvation of Marena",
        101915: "Sweeper",
        101920: "Sword of Nuln",
        101922: "Sword of Troile",
        101936: "Tapestry of Blood",
        102005: "Traditionalist",
        102058: "Uncontrollable Rage",
        102060: "Undead Persistence",
        102061: "Undead Strength",
        102095: "Veil of Darkness",
        102123: "Virolax Facility",
        102200: "Year of Fortune",
        200017: "Aeron (G3)",
        200018: "Afifa, The Herald (G4)",
        200052: "Alfred Benezri (G3 ADV)",
        200053: "Ali Kar (G3)",
        200054: "Alicia Barrows (G4)",
        200073: "Amenophobis (G4)",
        200108: "Antara (G4)",
        200115: "Antonino (G3)",
        200128: "Ardan Lane (G3)",
        200147: "Assad Salhoum (G4)",
        200183: "Beckett (G3)",
        200184: "Beckett (G3 ADV)",
        200212: "Boss Callihan (G3)",
        200214: "Bothwell (G3)",
        200223: "Brunhilde (G3)",
        200274: "Christopher Houghton (G3)",
        200276: "Clarissa Steinburgen (G3)",
        200292: "Count Germaine (G4)",
        200302: "Cristos Mantigo (G3)",
        200307: "Cyscek (G4)",
        200316: "Danielle Diron (G3)",
        200408: "Elena Mendoza Vasquez (G3)",
        200415: "Elizabeth Westcott (G3)",
        200439: "Étienne Fauberge (G3)",
        200453: "Fahd al-Zawba'a (G4)",
        200457: "Faruq (G2)",
        200465: "Ferox, The Rock Lord (G2 ADV)",
        200511: "Gharston Roland (G4)",
        200514: "Giangaleazzo, The Traitor (G4)",
        200546: "Guillermo Arsuaga (G4)",
        200552: "Gustavo Morales (G3)",
        200564: "Hanna Redmonds (G3)",
        200574: "Harry Reese (G3)",
        200578: "Hazimel (G4)",
        200582: "Hector Trelane (G4)",
        200596: "Hezekiah Rutledge (G4)",
        200606: "Horrock (G3)",
        200626: "Iliana (G2)",
        200661: "Jacob, The Glitch (G3)",
        200670: "Jan Pieterzoon (G2 ADV)",
        200692: "Jeremiah Noble (G3)",
        200694: "Jeremy MacNeil (G3 ADV)",
        200695: "Jeremy Talbot (G3)",
        200698: "Jesse Menks (G3)",
        200702: "Jezebelle (G4)",
        200716: "John Paleologus (G3)",
        200724: "Joseph O'Grady (G3)",
        200733: "Juggler (G3)",
        200742: "Kahina the Sorceress (G3)",
        200757: "Karsh (G3)",
        200761: "Katarina (G2)",
        200821: "Lazarus (G1 ADV)",
        200822: "Lazarus James (G4)",
        200834: "Lia Milliner (G3)",
        200839: "Lisa Noble (G3)",
        200853: "Lord Ashton (G3)",
        200875: "Lucita (G2 ADV)",
        200903: "Maldavis (G3)",
        200909: "Marcel de Breau (G3)",
        200916: "Marcus Vitel (G3 ADV)",
        200942: "Marthe Dizier (G3)",
        200962: "Matthew Romans (G4)",
        200982: "Michael Luther (G3)",
        201019: "Muhandis (G4)",
        201021: "Mukhtar Bey (G4)",
        201040: "Nedal, The Careless (G3)",
        201090: "Panagos Levidis (G2)",
        201142: "Quentin King III (G3 ADV)",
        201158: "Ramona (G2)",
        201159: "Ramona (G2 ADV)",
        201193: "Robin Withers (G4)",
        201215: "Ryder (G4)",
        201232: "Sandra White (G3)",
        201239: "Sarah Raines (G3)",
        201245: "Sascha Vykos, The Angel of Caine (G2 ADV)",
        201251: "Scarlet Carson O'Toole (G3)",
        201265: "Seren Sukardi (G4)",
        201273: "Shahid (G4)",
        201278: "Shawnda Dorrit (G4)",
        201299: "Smallpox Griet (G3)",
        201307: "Sophia Watson (G3)",
        201308: "Spider (G4)",
        201325: "Sundown (G3)",
        201353: "Tegyrius, Vizier (G2 ADV)",
        201363: "Theo Bell (G2 ADV)",
        201384: "Torvus Bloodbeard (G3)",
        201396: "Tusk, The Talebearer (G1 ADV)",
        201434: "Velya, The Flayer (G3)",
        201502: "Zayyat, The Sandstorm (G3)",
    },
    # Round 27 (slice 16am): Secrets of New York and Noites Brasileiras,
    # the 2026-10-31 bundles (owner, 2026-11-08).
    {
        102366: "Porto de Santos",
        102367: "Aaron Fletcher, Silent Investor",
        102368: "Academia de Campeões, São Paulo",
        102369: "The Art Hole, New York",
        102370: "Backroom Deal",
        102371: "Bar Último Trago, Campina Grande",
        102372: "Cãoera",
        102373: "Chave de Braço",
        102374: "Circulatory System Outlet",
        102375: "Convento da Penha, Vila Velha",
        102376: "Deadly Secrets",
        102377: "Destilaria Estrela da Paraíba",
        102378: "Double Spiral Corporation",
        102379: "Emboldened Councilor",
        102380: "Father Anthony",
        102381: "Hálito Negro",
        102382: "Iara",
        102383: "Information Exchange",
        102384: "Lambreta",
        102385: "Meia Lua",
        102386: "Mulher de Branco",
        102387: "Palavra Sagrada, Rio de Janeiro",
        102388: "Respected Representative",
        102389: "Ron Zertnert, S.A.D. Agent",
        102390: "St. Patrick's Cathedral, New York",
        102391: "Vial of Zapathasura's Vitae",
        201787: "Addison Payne (G6)",
        201788: "Aisling Sturbridge (G6)",
        201789: "Aríete (G6)",
        201790: "Bruxa Branca (G7)",
        201791: "Caçador de Raposas (G7)",
        201792: "Caique (G7)",
        201793: "Carter Vanderweyden (G6)",
        201794: "Dorinha, a Confusa (G7)",
        201795: "Durok (G6)",
        201796: "Eduardo, o Oráculo Mascarado (G7)",
        201797: "Elie Sansaricq (G6)",
        201798: "Gianni D'Angelo (G7)",
        201799: "Hellene Panhard (G7)",
        201800: "Hope (G7)",
        201801: "Julia Sowinski (G6)",
        201802: "Kaiser (G6)",
        201803: "Kali (G6)",
        201804: "Katherine Wiese (G7)",
        201805: "Luana do Lago (G7)",
        201806: "Luíza (G6)",
        201807: "Marcela (G6)",
        201808: "Maria Quitéria (G6)",
        201809: "Mel, a Guerreira Vibrante (G6)",
        201810: "O Encourado (G7)",
        201811: "O Grande Arquiteto (G7)",
        201812: "Padraic Conroy (G7)",
        201813: "Qadir al-Asmai (G7)",
        201814: "Rey, o Mentor (G6)",
        201815: "Samira (G7)",
        201816: "Sydnelson (G7)",
        201817: "Tereza Rossi (G6)",
        201818: "Tharmas (G6)",
        201819: "Thomas Arturo (G6)",
        201820: "Torque (G7)",
        201821: "Tubarão (G7)",
        201822: "Valerie Duval (G7)",
        201823: "Vritra (G6)",
    },
    # Round 28: Heirs to the Blood and its 2018 reprint completed (owner,
    # 2026-11-09: "We aim to add all the cards ultimately. Let's work set
    # by set."; the cards board's lines).
    {
        100013: "Absorb the Mind",
        100025: "Agent of Power",
        100030: "The Ailing Spirit",
        100048: "Amulet of Temporal Perception",
        100117: "Autonomic Mastery",
        100134: "Barrenness",
        100151: "Benefit Performance",
        100164: "The Black Beast",
        100181: "Blessed Resilience",
        100184: "Blessing of the Beast",
        100185: "Blessing of the Name",
        100189: "Bliss",
        100190: "Blissful Agony",
        100213: "Blood Tempering",
        100221: "Blooding by the Code",
        100256: "Brother in Arms",
        100292: "Capitalist",
        100310: "Cavalier",
        100330: "Charge of the Buffalo",
        100336: "Cheat the Fates",
        100363: "Clockwerx",
        100372: "Code of Samiel",
        100383: "Command Performance",
        100389: "Compress",
        100424: "Coroner's Contact",
        100437: "Covincraft",
        100440: "Create Gargoyle",
        100487: "Dagger",
        100497: "Darkness Within",
        100512: "Decompose",
        100517: "Defender of the Haven",
        100535: "Development",
        100544: "Din of the Damned",
        100579: "Draeven Softfoot",
        100599: "Ears of the Hare",
        100648: "Ensconced",
        100668: "Evil Eye",
        100679: "Eye of Unforgiving Heaven",
        100700: "Fanfare for Elysium",
        100741: "Flames of the Netherworld",
        100754: "Foldable Machine Gun",
        100755: "Folderol",
        100783: "Fractura",
        100828: "Gift of Sleep",
        100854: "Greater Curse",
        100859: "Groaning Corpse",
        100888: "Harmony",
        100892: "Hatchling",
        100899: "Hay Ride",
        100915: "Hexe",
        100920: "Hide the Heart",
        100937: "Horseshoes",
        100940: "Houngan",
        100944: "Hunger Moon",
        101020: "Jar of Skin Eaters",
        101025: "Joumlon's Axe",
        101026: "Journal of Hrorsh",
        101064: "Knotted Cord",
        101077: "Lead Fist",
        101109: "Lily Prelude",
        101113: "Little Mountain Cemetery",
        101123: "Lord of Serenity",
        101128: "Loving Agony",
        101149: "Maleficia",
        101177: "Masca",
        101197: "Member of the Entourage",
        101220: "Minor Curse",
        101234: "Mole's Tunneling",
        101236: "Momentary Delay",
        101244: "Morphean Blow",
        101281: "New Moon Sigil",
        101312: "Obtenebration",
        101316: "Off Kilter",
        101319: "Olid Loa",
        101362: "The Path of Harmony",
        101367: "The Path of Retribution",
        101368: "The Path of Service",
        101374: "Patrol",
        101382: "Penitent Resilience",
        101425: "Potio Martyrium",
        101426: "Pounce",
        101481: "Pressing Flesh",
        101505: "Psalm of the Damned",
        101556: "Razor Bat",
        101563: "Reanimated Corpse",
        101607: "Research",
        101640: "The Rising",
        101643: "Ritual Scalpel",
        101645: "Rock Cat",
        101673: "Safe Passage",
        101688: "Scarlet Lore",
        101691: "Scobax",
        101696: "Scourge of Alecto",
        101701: "Scry the Hearthstone",
        101732: "Shaal Fragment",
        101747: "Shadowed Eyes",
        101757: "Shatter the Gate",
        101761: "Shattering Crescendo",
        101833: "Soul of the Earth",
        101847: "Spirit Claws",
        101864: "Steadfastness",
        101870: "Stiff Contempt",
        101884: "Striga",
        101887: "Strix",
        101921: "Sword of the Righteous",
        101956: "Tend the Flock",
        101965: "Thaumaturgy",
        101969: "Thicker than Blood",
        101977: "Threading the Path of Orpheus",
        101985: "Tinglestripe",
        102016: "Treat the Sick Mind",
        102106: "Vengeance of Samiel",
        102124: "Virtuosa",
        102125: "Visionquest",
        102130: "Voices of the Castle",
        102146: "Warding the Beast",
        200006: "Abebe (G4)",
        200038: "Al-Muntathir, God's Witness (G5)",
        200042: "Alcoan (G6)",
        200089: "Angel Chavarria (G6)",
        200104: "Annazir (G5)",
        200129: "Aredhel (G4)",
        200134: "Ariel (G6)",
        200156: "Azrael (G6)",
        200169: "Baroque (G5)",
        200203: "Black Wallace (G4)",
        200219: "Brigitte Mandisa (G6)",
        200229: "Byzar (G6)",
        200257: "Cedric (G4)",
        200269: "Chaundice (G4)",
        200322: "Darlene Killian (G5)",
        200371: "Dovey Ebfwe (G6)",
        200447: "Evil Jensen (G4)",
        200460: "Federico di Padua (G5)",
        200481: "Frank Weisshadel (G4)",
        200486: "Fustuk (G5)",
        200537: "Greer Worder (G6)",
        200541: "Grotesque (G6)",
        200570: "Harlan Graves (G6)",
        200585: "Helen Fairchild (G6)",
        200598: "Hillanvale (G6)",
        200605: "The Horde (G6)",
        200672: "Janet Langer (G5)",
        200725: "Josette (G6)",
        200728: "Jozz (G6)",
        200801: "La Viuda Blanca (G6)",
        200813: "Langa (G4)",
        200854: "Lord Ephraim Wainwright (G5)",
        200857: "Lord Vauxhall (G4)",
        200891: "Lydia, Grand Praetor (G6)",
        200895: "Macoute (G4)",
        200901: "Malachai (G6)",
        200930: "Mariel St. John (G4)",
        200961: "Matthew (G6)",
        200988: "Mikael Birkholm (G6)",
        201009: "Mordechai Ben-Nun (G5)",
        201011: "Morlock (G4)",
        201028: "Myrna Goldman (G6)",
        201031: "Nahum Enosh (G6)",
        201061: "Nitidas (G6)",
        201063: "Nkechi (G4)",
        201070: "Obaluaye (G5)",
        201167: "Rashiel (G4)",
        201194: "Rocia (G4)",
        201211: "Rusticus (G5)",
        201266: "Serenna the White (G6)",
        201287: "Silas (G6)",
        201298: "Slag (G5)",
        201330: "Sylvie Helgon (G6)",
        201339: "Tanginé (G4)",
        201346: "Tarrence Moore (G5)",
        201374: "Titus Camille (G4)",
        201385: "Toy (G5)",
        201387: "Troglodytia (G4)",
        201416: "Uriel (G4)",
        201433: "Veejay Vinod (G6)",
        201452: "Virginie, Prodigy (G4)",
        201480: "Xeper, Sultan of Lepers (G6)",
    },
    # Round 29: Keepers of Tradition and its 2018 reprint completed (owner,
    # 2026-11-09: "We aim to add all the cards ultimately. Let's work set
    # by set."; the cards board's lines).
    {
        100010: "Abomination",
        100024: "Agate Talisman",
        100035: "Al's Army Apparatus",
        100068: "Anima Gathering",
        100080: "Arcane Appraiser",
        100089: "Armor of Vitality",
        100090: "Arms Dealer",
        100125: "Backstep",
        100130: "Bang Nakh — Tiger's Claws",
        100147: "The Becoming",
        100152: "Beretta 9mm",
        100176: "Blade of Enoch",
        100202: "Blood Hunt",
        100204: "Blood of Sandman",
        100233: "Bomb",
        100242: "Bounty",
        100253: "Brick Laying",
        100262: "Brujah Justicar",
        100264: "Brute Force",
        100267: "Bundi",
        100273: "Burst of Sunlight",
        100289: "Can't Take it with You",
        100334: "Charming Lobby",
        100338: "Childling Muse",
        100419: "Contingency Planning",
        100432: "Coterie Tactics",
        100494: "Dark Mirror of the Mind",
        100502: "Day Operation",
        100580: "Dragon's Breath Rounds",
        100619: "Elder Kindred Network",
        100631: "Elysium: The Arboretum",
        100683: "Eyes of the Beast",
        100737: "First Tradition: The Masquerade",
        100758: "Force of Personality",
        100768: "Forgery",
        100819: "Ghoul Retainer",
        100894: "The Haunting",
        100900: "Haymaker",
        100925: "High Ground",
        100926: "High Museum of Art, Atlanta",
        100933: "Honor the Elders",
        100939: "Hostile Takeover",
        100993: "Instability",
        101038: "Keep it Simple",
        101059: "King's Rising",
        101083: "Leech",
        101129: "Loyal Street Gang",
        101155: "Malkavian Prank",
        101168: "Mark V",
        101174: "Martinelli's Ring",
        101219: "Minor Boon",
        101221: "Minor Irritation",
        101245: "Mouthpiece",
        101250: "Muddled Vampire Hunter",
        101266: "National Guard Support",
        101280: "New Management",
        101294: "Nocturn Theater",
        101307: "The Oath",
        101393: "Persona Non Grata",
        101398: "Phased Motion Detector",
        101400: "Pier 13, Port of Baltimore",
        101434: "Powerbase: Chicago",
        101449: "Praxis Seizure: Atlanta",
        101455: "Praxis Seizure: Chicago",
        101462: "Praxis Seizure: Houston",
        101465: "Praxis Seizure: Miami",
        101472: "Praxis Seizure: Washington, D.C.",
        101510: "Psychic Veil",
        101558: "Read Intentions",
        101594: "Relentless Pursuit",
        101604: "Repair the Undead Flesh",
        101655: "Rowan Ring",
        101695: "Scourge",
        101700: "Scrounging",
        101715: "Sengir Dagger",
        101754: "Shape Mastery",
        101758: "Shattered Mirror",
        101788: "Sixth Tradition: Destruction",
        101820: "Social Ladder",
        101824: "Sonar",
        101835: "Soul Scan",
        101841: "Special Report",
        101842: "Specialization",
        101871: "Still the Mortal Flesh",
        101878: "Storm Sewers",
        101889: "Stutter-Step",
        101902: "The Summoning",
        101907: "Suppressing Fire",
        101941: "Target Retainer",
        102021: "Tremere Justicar",
        102068: "Undying Tenacity",
        102099: "Veles' Hunt",
        102102: "Veneficorum Artum Sanguis",
        102111: "Ventrue Justicar",
        102118: "Victim of Habit",
        102129: "Voice of Madness",
        102173: "Wendell Delburton",
        102178: "White Nights Massacre",
        102182: "Will of the Council",
        102184: "Winchester Mansion",
        102192: "Wooden Stake",
        102198: "XTC-Laced Blood",
        102211: "Zoning Board",
        200043: "Aleister Crowley (G5)",
        200048: "Alex Wilkins (G5)",
        200055: "Allanyan Serata (G5)",
        200101: "The Ankou (G5)",
        200168: "Baron Dieudonne (G4)",
        200191: "Bernard, the Scourge (G4)",
        200193: "Beth Malcolm (G5)",
        200194: "Bethany Ray (G5)",
        200209: "Bloody Mary (G5)",
        200226: "Bulscu (G5)",
        200244: "Carlak (G5)",
        200297: "Count Zaroff (G5)",
        200360: "Don Caravelli (G5)",
        200361: "Don Cerro (G5)",
        200377: "Dr. John Dee (G4)",
        200451: "Ezra Hawthorne (G4)",
        200471: "Florentina Lengauer (G4)",
        200473: "Foureyes (G4)",
        200482: "Freddy Gage (G5)",
        200494: "Garret (G4)",
        200530: "Gotsdam, The Tired Warrior (G4)",
        200531: "Gracetius (G5)",
        200550: "Gustav Breidenstein (G4)",
        200556: "Gwyedd (G4)",
        200591: "Herbert Westin (G4)",
        200601: "Honest Abe (G5)",
        200657: "Jack Tredegar (G5)",
        200675: "Jann Berger (G4)",
        200711: "Johan Wrede (G4)",
        200713: "Johannes Castelein (G4)",
        200722: "Joseph DiGiaccomo (G5)",
        200744: "Kalila (G5)",
        200768: "Keller Thiel (G5)",
        200838: "Lindsay Yates (G5)",
        200841: "Lisé (G5)",
        200855: "Lord Fianna (G5)",
        200893: "Lynn Thompson (G5)",
        200894: "MacAlister Marshall (G5)",
        200949: "Masdela (G5)",
        200984: "Mictlantecuhtli (G5)",
        200986: "Miguel Cordovera (G5)",
        201007: "Montecalme (G5)",
        201099: "Paul Calderone (G5)",
        201107: "Pedrag Hasek (G5)",
        201122: "Philippe de Marseilles (G5)",
        201131: "Portia (G5)",
        201168: "Rathmere (G5)",
        201177: "Reiner Stoschka (G5)",
        201214: "Ruxandra (G5)",
        201235: "Santaleous (G5)",
        201254: "Sean Andrews (G4)",
        201267: "Sergei Voshkov, The Eye (G5)",
        201283: "Sheva Carr (G5)",
        201284: "Shiloh Marie, Vengeance (G5)",
        201336: "Talbot (G5)",
        201361: "Themistocles (G5)",
        201379: "Tomaine (G4)",
        201390: "Tryphosa (G4)",
        201446: "Vidal Jarbeaux (G5)",
        201447: "Viktor, The Night General (G5)",
        201470: "William Thorbecke (G5)",
        201504: "Zelios (G5)",
        200599: 'Hiram "Hide" DeVries (G5)',
    },
    # Round 30: Lords of the Night completed (owner, 2026-11-09: "We aim
    # to add all the cards ultimately. Let's work set by set."; the cards
    # board's lines).
    {
        100045: "Ambulance",
        100047: "Amria",
        100076: "Apparition",
        100092: "Army of Apparitions",
        100121: "Baal's Bloody Talons",
        100155: "Bestow Vigor",
        100191: "Blithe Acceptance",
        100193: "Blood Awakening",
        100222: "Bloodlust",
        100268: "Burden the Mind",
        100318: "Chair of Hades",
        100331: "Charigger, The Axe",
        100341: "Chill of Oblivion",
        100342: "Chimerstry",
        100370: "Cobra Fangs",
        100384: "Command the Legion",
        100387: "Community Justice",
        100388: "Compel the Spirit",
        100396: "Condemn the Sins of the Father",
        100449: "The Crocodile Temple",
        100565: "Divine Image",
        100587: "Dream World",
        100610: "Ecstasy",
        100615: "Edged Illusion",
        100650: "Enticement",
        100665: "The Eternal Mask",
        100667: "The Eternals of Sirius",
        100669: "Ex Nihilo",
        100681: "Eyes of Blades",
        100697: "False Resonance",
        100701: "Fantasy World",
        100706: "Fata Amria",
        100707: "Fata Morgana",
        100708: "Fatuus Mastery",
        100729: "Fillip",
        100748: "Flesh Bond",
        100763: "Forearm Block",
        100767: "Forger's Hammer",
        100778: "Foul Blood",
        100810: "Garrote",
        100814: "Gemini's Mirror",
        100846: "Gramle",
        100884: "Haqim's Law: Judgment",
        100885: "Haqim's Law: Leadership",
        100887: "Hard Case",
        100905: "Heart's Desire",
        100923: "Hierophant",
        100948: "Ignis Fatuus",
        100992: "Inspire Greatness",
        101022: "Jericho Founding",
        101023: "The Jones",
        101033: "Karavalanisha Vrana",
        101044: "The Khabar: Honor",
        101046: "Khazar's Diary (Endless Night)",
        101065: "Kpist m/45",
        101095: "Lesser Boon",
        101126: "Lost in Translation",
        101148: "Malajit Chandramouli",
        101162: "Mantle of the Moon",
        101188: "Mayaparisatya",
        101202: "Mercury's Arrow",
        101222: "Mirror Image",
        101224: "Mirror's Visage",
        101232: "Mokolé Blood",
        101267: "Necromancy",
        101273: "Nephren-Ka",
        101274: "Nest of Eagles",
        101284: "Nightmare Curse",
        101287: "Nightstick",
        101320: "Omael Kuman",
        101322: "Open Dossier",
        101358: "The Path of Blood",
        101366: "The Path of Paradox",
        101407: "Poison the Well of Life",
        101423: "Possession",
        101445: "Powerbase: Zürich",
        101474: "Precision",
        101504: "Proxy Kissed",
        101506: "Pseudo-Blindness",
        101529: "Quick Jab",
        101534: "Quietus",
        101616: "Resume the Coil",
        101618: "Retain the Quick Blood",
        101623: "Revelation of Despair",
        101624: "Revelation of Ecstasy",
        101626: "Revelation of Wrath",
        101651: "Rooftop Shadow",
        101694: "Scorpion's Touch",
        101714: "Selective Silence",
        101728: "Serpentis",
        101731: "Set's Curse",
        101738: "Shadow Feint",
        101751: "Shambling Hordes",
        101762: "Sheepdog",
        101763: "Shell Break",
        101776: "Shroudsight",
        101831: "Soul Feasting",
        101845: "Spell of Life",
        101885: "Strike at the True Flesh",
        101912: "Svadharma",
        101917: "Swiss Cut",
        101923: "Sympathetic Agony",
        101927: "Tajdid",
        101932: "Talith",
        101938: "Target Hand",
        101939: "Target Head",
        101940: "Target Leg",
        101954: "Temptation",
        102033: "Trophy: Library",
        102034: "Trophy: No Questions",
        102037: "Trophy: Revered",
        102044: "Truth of a Thousand Lies",
        102045: "Truth of Blood",
        102048: "Tutu the Doubly Evil One",
        102072: "Unholy Penance",
        102085: "Urban Jungle",
        102148: "Warrant",
        102175: "Whispers from the Dead",
        102183: "Will-o'-the-Wisp",
        102204: "Zapaderin",
        200024: "Ahmose Chambers (G5)",
        200097: "Anjalika Underwood (G5)",
        200140: "Arnulf Jormungandrsson (G5)",
        200163: "Bakr (G5)",
        200188: "Belle Equitone (G4)",
        200192: "Bertrand d'Anjou (G4)",
        200218: "Brian Thompson (G4)",
        200270: "Chavi Oraczko (G5)",
        200282: "Clea Auguste d'Holford (G5)",
        200350: "Dmitri Borodin (G4)",
        200393: "Durga Syn (G4)",
        200445: "Evan Rogers (G4)",
        200475: "Francis Milliner (G4)",
        200492: "Ganesh (G5)",
        200553: "Gwen Brand (G5)",
        200559: "Hafsa, The Watcher (G5)",
        200561: "Halim Bey (G4)",
        200650: "Izhim abd Azrael (G5)",
        200668: "Jamal (G5)",
        200682: "Jayakumar (G5)",
        200712: "Johann Matheson (G5)",
        200741: "Kabede Maru (G5)",
        200759: "Kashan (G4)",
        200765: "Kay Polerno (G5)",
        200784: "Kiradin (G4)",
        200804: "Lady Constancia (G4)",
        200818: "Layla bint-Nadr (G4)",
        200847: "Lizette (G4)",
        200858: "Lorenzo Detuono (G5)",
        200885: "Luna Giovanni (G5)",
        200912: "Marcia Felicia Licinia (G5)",
        200917: "Margaret Milliner (G4)",
        200938: "Marla Kenyon (G4)",
        201008: "Monty Coven (G5)",
        201033: "Nakhthorheb (G4)",
        201041: "Neel Ramanathan (G4)",
        201072: "Ogwon (G5)",
        201092: "Paolo Sardenzo (G5)",
        201093: "Papa Legba (G5)",
        201109: "Pentweret (G5)",
        201118: "Phaedyme (G5)",
        201129: "Porphyrion (G5)",
        201179: "Renenet (G4)",
        201223: "Sajid al Misbah (G5)",
        201269: "Seterpenre (G5)",
        201282: "Shemti (G5)",
        201293: "Sisocharis (G5)",
        201316: "Stephen Milliner (G5)",
        201323: "Sukainah (G5)",
        201324: "Sundervere, The Devil Brahmin (G4)",
        201429: "Vasiliy Vasilevich (G5)",
        201431: "Vassily Taltos (G4)",
    },
    # Round 31: Third Edition completed (owner, 2026-11-09: "We aim to add
    # all the cards ultimately. Let's work set by set."; the cards board's
    # lines).
    {
        100005: "Abandoning the Flesh",
        100008: "Ablative Skin",
        100020: "Acrobatics",
        100049: "Amusement Park Hunting Ground",
        100096: "The Art of Love",
        100116: "Auto-da-fé",
        100124: "Backstab",
        100142: "Bauble",
        100166: "Black Gloves",
        100170: "Black Spiral Buddy",
        100182: "Blessing of Chaos",
        100200: "Blood Feast",
        100206: "Blood of the Sabbat",
        100218: "Bloodbath",
        100228: "Body Arsenal",
        100238: "Bonecraft",
        100246: "Brass Knuckles",
        100271: "Burning Wrath",
        100278: "The Call",
        100296: "Cardinal Sin: Insubordination",
        100337: "Cheval de Bataille",
        100367: "Club Zombie",
        100373: "Codex of the Damned",
        100379: "Combat Shotgun",
        100380: "Command",
        100429: "Corrupt Construction",
        100434: "Courier",
        100438: "Cracking the Wall",
        100458: "Crusade: Detroit",
        100467: "Crusade: New York",
        100469: "Crusade: Philadelphia",
        100470: "Crusade: Pittsburgh",
        100471: "Crusade: Rome",
        100472: "Crusade: Toronto",
        100479: "Cull the Herd",
        100490: "The Damned",
        100491: "Danse Macabre",
        100498: "Darksight",
        100500: "Dauntain Black Magician",
        100525: "Deploy the Hand",
        100554: "Disengage",
        100558: "Dissolution",
        100569: "Dogs of War",
        100582: "Drain Essence",
        100641: "Enchanted Marionette",
        100660: "Esbat",
        100661: "Escaped Mental Patient",
        100672: "Excommunication",
        100802: "Gang Tactics",
        100803: "Gang Territory",
        100805: "Gangrel Conspiracy",
        100881: "Hand Intervention",
        100895: "Haven Affinity",
        100914: "Hexaped",
        100917: "Hidden Pathways",
        100970: "Inconnu Tutelage",
        100978: "Infernal Familiar",
        100979: "Infernal Pact",
        100985: "Information Network",
        101051: "Kindred Manipulation",
        101066: "Kraken's Kiss",
        101081: "Leather Jacket",
        101084: "Left for Dead",
        101086: "Legacy of Caine",
        101093: "Legwork",
        101116: "Lobotomy",
        101117: "Local 1111",
        101132: "Lunatic Eruption",
        101133: "Lupine Assault",
        101153: "Malkavian Game",
        101164: "Marijava Ghoul",
        101173: "Martial Ritus",
        101193: "Media Influence",
        101196: "Meld with the Land",
        101209: "Mighty Grapple",
        101241: "Monomancy",
        101288: "Nimble Feet",
        101364: "The Path of Metamorphosis",
        101365: "The Path of Night",
        101370: "The Path of the Feral Heart",
        101375: "Patronage",
        101378: "Patterns in the Chaos",
        101383: "Pentex™ Loves You!",
        101391: "Perpetual Care",
        101412: "Political Antagonist",
        101418: "Political Struggle",
        101438: "Powerbase: Mexico City",
        101517: "Pulse of the Canaille",
        101520: "Purchase Pact",
        101522: "Purity of the Beast",
        101535: "Rabble Razing",
        101537: "Rafastio Ghoul",
        101549: "Rave",
        101561: "Reality Mirror",
        101568: "Recruiting Party",
        101569: "Recruitment",
        101571: "Recure of the Homeland",
        101608: "Resilience",
        101615: "Restructure",
        101666: "Sabbat Inquisitor",
        101667: "Sabbat Priest",
        101677: "San Nicolás de los Servitas",
        101682: "Saturday-Night Special",
        101726: "Sermon of Caine",
        101737: "Shadow Court Satyr",
        101740: "Shadow of the Beast",
        101744: "Shadow Step",
        101745: "Shadow Strike",
        101777: "Sibyl's Tongue",
        101785: "Sire's Index Finger",
        101795: "Skin Trap",
        101802: "Slave Auction",
        101827: "Song of Serenity",
        101849: "Spirit Summoning Chamber",
        101860: "Starshell Grenade Launcher",
        101867: "Stealth Ritus",
        101879: "Strained Vitae Supply",
        101892: "Succubus",
        101903: "Sunrise Service",
        101910: "Survivalist",
        101919: "Sword of Judgment",
        101930: "Talaq, The Immortal",
        101933: "Talons of the Dead",
        101952: "Templar",
        101986: "Tithings",
        101997: "Total Insanity",
        102002: "Toy Chest Test",
        102010: "Transfer of Power",
        102086: "Using the Advantage",
        102101: "Vendetta",
        102110: "Ventrue Investment",
        102115: "Vial of Garou Blood",
        102133: "Vox Senis",
        102145: "War Party",
        102153: "Waste Management Operation",
        102179: "White Phosphorus Grenade",
        102191: "Wolf Companion",
        102202: "Young Bloods",
        200138: "Armin Brenner (G4)",
        200151: "Axel Von Anders (G4)",
        200182: "Beauregard Krueller (G4)",
        200187: "Bela Kardoza (G4)",
        200197: "Bill Butler (G4)",
        200250: "Caroline Bishops (G4)",
        200267: "Charlie Tyne (G4)",
        200288: "Conrad Adoula (G4)",
        200296: "Count Vladimir Rustovitch (G4)",
        200325: "Darvag, The Butcher of Rus (G4)",
        200337: "DeSalle (G4)",
        200359: "Dominique Santo Paulo (G4)",
        200379: "Dr. Julius Sutphen (G3 ADV)",
        200388: "Droescher One-Eye (G4)",
        200390: "Drusilla Euphemia (G4)",
        200429: "Eric Kressida (G4)",
        200437: "Esoara (G4)",
        200452: "Fabrizia Contreraz (G4)",
        200458: "Father Juan Carlos (G4)",
        200484: "Frère Marc (G4)",
        200488: "Gabriel de Cambrai (G4)",
        200535: "Gravitnir (G4)",
        200543: "Guedado (G4)",
        200572: "Harold Zettler, Pentex Director (G4)",
        200584: "Hektor (G4)",
        200612: "Hukros (G4)",
        200613: "Humo (G4)",
        200619: "Icarus, The Manchurian (G4)",
        200646: "Isouda de Blaise (G4)",
        200664: "Jacques Molay (G4)",
        200665: "Jaggedy Andy (G4)",
        200671: "Jane Sims (G4)",
        200721: "Joseph Cambridge (G4)",
        200743: "Kai Simmons (G4)",
        200790: "Konrad Fleischer (G4)",
        200805: "Lady Zara Slatikov (G4)",
        200815: "Laszlo Mirac (G4)",
        200831: "Leo Washington (G4)",
        200833: "Lernean (G4)",
        200845: "Little Willie (G4)",
        200852: "Lord Aaron Wesley Wilkshire (G4)",
        200861: "Louis de Maisonneuve (G4)",
        200877: "Lucubratio (G4)",
        200878: "Lucy Markowitz (G4)",
        200881: "Luke Fellows (G4)",
        200882: "Lula Burch (G4)",
        200918: "Margarite (G4)",
        200947: "Mary Johnson (G4)",
        200970: "Melinda Galbraith (G4)",
        200971: "Melinda Galbraith (G4 ADV)",
        201012: "Morrow the Sage (G4)",
        201015: "Mowgli (G4)",
        201018: "Mugur Sabau (G4)",
        201056: "Nickolai, The Survivor (G4)",
        201066: "Nostoket (G4)",
        201097: "Patrick (G4)",
        201150: "Radu Bistri (G4)",
        201154: "Rain (G4)",
        201164: "Raphael Catarari (G4)",
        201172: "Redbone McCray (G4)",
        201189: "Rico Loco (G4)",
        201197: "Rodrigo (G4)",
        201216: "Ryszard (G4)",
        201252: "Schuyler (G4)",
        201268: "Servius Marius Pustula (G4)",
        201270: "Sha-Ennu (G4)",
        201300: "Smash (G4)",
        201351: "Tears, The Dark Pierrot (G4)",
        201358: "Terrifisto (G4)",
        201403: "Ulrike Rothbart (G4)",
        201417: "Urraca (G4)",
        201418: "Uta Kovacs (G4)",
        201479: "Xendil Charmer (G4)",
        201491: "Ysador the Foul (G4)",
    },
    # Round 32: Legacies of Blood completed (owner, 2026-11-09: "We aim to add
    # all the cards ultimately. Let's work set by set."; the cards board's
    # lines).
    {
        100028: "Ahrimane Protectorate",
        100034: "Akunanse Kholo",
        100051: "Ananasi Vampirephile",
        100061: "Ancestor Spirit",
        100062: "Ancestor's Insight",
        100087: "Armor of Caine's Fury",
        100088: "Armor of Terra",
        100100: "Art's Traumatic Essence",
        100103: "Asanbonsam Ghoul",
        100105: "Ashes to Ashes",
        100118: "Aversion",
        100138: "Basilisk's Touch",
        100140: "Bastille Opera House",
        100141: "Battle Frenzy",
        100150: "Belonging Grants Protection",
        100160: "Bima",
        100161: "Bind the Night-Walker",
        100179: "Blessed Audience",
        100257: "Brother's Blood",
        100270: "Burning Touch",
        100321: "Chameleon's Colors",
        100345: "Choir",
        100349: "Circle",
        100357: "Cleansing Ritual",
        100359: "Clio's Kiss",
        100365: "Clotho's Gift",
        100377: "Collapse the Arches",
        100393: "Concert Tour",
        100402: "Conductor",
        100412: "Conscripted Statue",
        100417: "Consume the Dead",
        100439: "Crawling Chamber",
        100443: "Creeping Infection",
        100496: "Darkling Trickery",
        100507: "The Death of My Conscience",
        100508: "Death of the Drum",
        100537: "Devil-Channel: Back",
        100539: "Devil-Channel: Hands",
        100540: "Devil-Channel: Throat",
        100546: "Dirty Contract",
        100593: "Duma Rafiki",
        100596: "Dust to Dust",
        100602: "Earth Swords",
        100603: "Earth-Feeder",
        100608: "Echo of Harmonies",
        100623: "The Eldest Command Undeath",
        100626: "Elephant Guardian",
        100637: "Emergency Rations",
        100642: "Enforcer",
        100670: "Excellent Thirst",
        100694: "Falcon's Eye",
        100721: "Feral Spirit",
        100735: "Fire on the Mountain",
        100751: "Flow Within the Mountain",
        100780: "Founders of the Ebony Kingdom",
        100784: "Fractured Armament",
        100789: "Free Fight",
        100795: "Frozen Object",
        100816: "Ghost-Eater",
        100818: "Ghoul Messenger",
        100825: "Gift of Bellona",
        100835: "Glare of Lies",
        100839: "Glutton",
        100849: "The Grandest Trick",
        100873: "The Guruhi Are the Land",
        100874: "Guruhi Kholo",
        100878: "Hall of Hades' Court",
        100883: "Hanging Fermata",
        100901: "Healing Touch",
        100907: "Heaven's Gate",
        100924: "High Aye",
        100927: "High Orun",
        100929: "High Top",
        100941: "Hourglass of the Mind",
        100950: "Igoli's Loyalty",
        100977: "Infection",
        100983: "Informant",
        100998: "Internal Recursion",
        101006: "Invoking the Beast",
        101011: "Ishtarri Kholo",
        101030: "Jungle Hunting Ground",
        101037: "Kduva's Mask",
        101039: "Kerrie",
        101058: "King's Favor",
        101061: "Kiss of Lachesis",
        101076: "Lazarene Inquisitor",
        101079: "Leapfrog",
        101090: "Legend of the Leopard",
        101141: "Madrigal",
        101160: "Manstopper Rounds",
        101176: "Masai Blood Milk",
        101186: "Massassi's Honor",
        101226: "The Missing Voice",
        101255: "Muricia's Call",
        101276: "Neutral Guard",
        101278: "New in Town",
        101291: "No Secrets From the Magaji",
        101317: "Ohoyo Hopoksia (Bastet)",
        101339: "Overseer",
        101344: "Palace Hunting Ground",
        101348: "Panacea",
        101349: "Pandora's Whisper",
        101352: "Paris Opera House",
        101369: "The Path of Tears",
        101381: "Peacemaker",
        101392: "Persistent Echo",
        101397: "Phantom Speaker",
        101408: "Poker",
        101414: "Political Flux",
        101421: "Port Hunting Ground",
        101443: "Powerbase: Tshwane",
        101477: "Predator's Communion",
        101478: "Predator's Mastery",
        101493: "Projectile",
        101509: "Psychic Projection",
        101526: "Putrescent Servitude",
        101533: "Quicksilver Contemplation",
        101555: "Rayzeel's Song",
        101572: "Recurring Contemplation",
        101576: "Redcap Wilder",
        101586: "Regenerative Blood",
        101589: "Reindoctrination",
        101596: "Reliquary: Akunanse Remains",
        101597: "Reliquary: Biague",
        101617: "Resurrection",
        101634: "Riddle Phantastique",
        101636: "Rigor Mortis",
        101637: "Riposte",
        101648: "Roll",
        101709: "The Secret Must Be Kept",
        101710: "Secret Passage",
        101720: "Sense Vitality",
        101739: "Shadow of Taint",
        101750: "Shaman",
        101786: "Siren's Lure",
        101787: "Sires Command, Childer Inherit",
        101794: "Skin of the Chameleon",
        101800: "The Slaughterhouse",
        101818: "Soar",
        101840: "Speak with Spirits",
        101846: "Spike-Thrower",
        101848: "Spirit Marionette",
        101858: "Squirrel Balance",
        101862: "Static Virtue",
        101865: "Steal the Mind",
        101873: "Stone Dog",
        101874: "Stone Quills",
        101880: "Strange Day",
        101883: "Strength of the Bear",
        101899: "Summon Spirit Beast",
        101914: "Swarm",
        101916: "Swiftness of the Stag",
        101918: "Swoop",
        101959: "Terra Incognita",
        101975: "Those Who Endure Judge",
        101991: "Toreador's Bane",
        102000: "Tourette's Voice",
        102003: "Tracker's Mark",
        102011: "Transfusion",
        102013: "Travelers Obey the Tenets",
        102056: "Unburdening the Bestial Soul",
        102059: "Uncontrolled Impulse",
        102081: "Unseen Hibernation",
        102089: "Vampiric Speed",
        102127: "Vitae Block",
        102132: "Vox Domini",
        102135: "Vulture's Buffet",
        102143: "Wanderer's Counsel",
        102177: "Whistling Up the Beast",
        102181: "The Wildebeest",
        102186: "Winged Second",
        102187: "Wise Spider",
        102188: "Withering",
        200023: "Agru Kabera (G4)",
        200026: "Aimee Laroux (G4)",
        200032: "Aiyana, The Wolfcatcher (G4)",
        200037: "Al-Muntaquim, The Avenger (G4)",
        200066: "Amavi (G4)",
        200111: "Antoinette Dubois (G4)",
        200130: "Aren, Priest of Eshu (G4)",
        200153: "Ayo Igoli (G4)",
        200176: "Batsheva (G4)",
        200228: "Bupe Kuila (G4)",
        200329: "Dela Eden (G3)",
        200333: "Demdemeh (G4)",
        200353: "Dolie (G4)",
        200448: "Eze, The Demon Prince (G3)",
        200456: "Falhu Shibaba (G3)",
        200493: "Ganhuru (G4)",
        200505: "Gentha Shale (G4)",
        200542: "Gualtiero Ghiberti (G4)",
        200576: "Hasani (G4)",
        200602: "Honorine Ateba (G4)",
        200618: "Ibn Khaldun, Scholar (G4)",
        200636: "Iniko, The Black Lion (G4)",
        200645: "Ismitta (G4)",
        200654: "Jack Dawson (G2 ADV)",
        200703: "Jibade el-Bahrawi (G4)",
        200731: "Jubal (G4)",
        200747: "Kamaria (G4)",
        200748: "Kamau Jafari (G4)",
        200749: "Kamiri wa Itherero (G4)",
        200772: "Kenyatta (G4)",
        200783: "The Kikiyaon (G4)",
        200884: "Lumumba (G4)",
        200923: "Maria Stone (G3 ADV)",
        200953: "Maskini (G4)",
        200957: "Matata (G3)",
        200977: "Meno Ngari (G4)",
        200996: "Misrak (G4)",
        201016: "Mr. Noir (G4)",
        201023: "Muricia (G4)",
        201036: "Nangila Were (G4)",
        201064: "Nkule Galadima (G4)",
        201078: "Olugbenga (G4)",
        201086: "Otieno (G4)",
        201091: "Panya, The Wicked (G4)",
        201119: "Phagian (G4)",
        201132: "Prejudice (G4)",
        201206: "Rosemarie (G4)",
        201234: "Sanjo (G3)",
        201250: "Sayshila (G4)",
        201263: "Sennadurek (G4)",
        201277: "Shasa Abu Badr (G4)",
        201302: "Sobayifa (G3)",
        201392: "Tsunda (G4)",
        201399: "Ubende (G4)",
        201401: "Uchenna (G4)",
        201402: "Ugadja (G4)",
        201414: "Urenna Bunu (G4)",
        201463: "Wamukota (G4)",
        201484: "Yavu Matebo (G4)",
        201492: "Yseult (G3)",
        201505: "Zhara (G3)",
        201506: "Zhenga (G4)",
        201511: "Zygodat (G4)",
    },
    # Round 33: Kindred Most Wanted and Tenth Anniversary completed (owner,
    # 2026-11-09: "We aim to add all the cards ultimately. Let's work set
    # by set."; the cards board's lines).
    {
        100057: "Anarch Secession",
        100123: "Backflip",
        100143: "Bay and Howl",
        100156: "Betrayer",
        100194: "Blood Bond",
        100223: "Bloodstone",
        100224: "Bloodstorm of Chorazin",
        100265: "Bujo",
        100276: "Caiaphas Smith",
        100288: "Campground Hunting Ground",
        100351: "City Gangrel Connections",
        100395: "Concordance",
        100413: "Consecration Rites",
        100415: "Consignment to Duat",
        100436: "Covenant of Blood",
        100484: "D'habi Revenant",
        100503: "Dead Hand",
        100506: "Deal with the Devil",
        100536: "Deviki Prasanta",
        100547: "Dirty Little Secrets",
        100559: "Distant Friend",
        100577: "Dr. Marisa Fletcher, CDC",
        100664: "Esteem",
        100711: "Fear of the Void Below",
        100731: "The Final Nights",
        100733: "Fire Dance",
        100756: "Follow the Alpha",
        100806: "Gangrel Justicar",
        100817: "Ghoul Escort",
        100829: "Giovanni Acceptance",
        100837: "Glass Walker Pact",
        100838: "Gleam of Red Eyes",
        100850: "Grasp of the Python",
        100890: "Harzomatuili",
        100902: "Heart of Darkness",
        100911: "Hellhound",
        100949: "Ignore the Searing Flames",
        100958: "Immense Size",
        100972: "Incriminating Videotape",
        100988: "Inquisition",
        100997: "Insurance Scam",
        101004: "Invitation Accepted",
        101060: "Kingston Penitentiary, Ontario",
        101082: "Leathery Hide",
        101088: "Legacy of Power",
        101105: "Lifeless Tongues",
        101203: "Mercy for Seth",
        101208: "Might of the Camarilla",
        101213: "Mind of a Killer",
        101263: "The Name Forgotten",
        101286: "Nightshades",
        101303: "Not to Be",
        101328: "Orb of Ulain",
        101379: "The Peace of Khetamon",
        101389: "Permanency",
        101409: "Polaris Coach",
        101427: "Powder of Rigidity",
        101450: "Praxis Seizure: Barcelona",
        101468: "Praxis Seizure: Rome",
        101488: "Priority Shift",
        101511: "Psychomachia",
        101521: "Purification",
        101544: "Rapid Thought",
        101546: "Rastacourere",
        101551: "Ravnos Acceptance",
        101565: "Rebirth",
        101574: "Red List",
        101625: "Revelation of the Sire",
        101630: "Revocation of Tyre",
        101641: "Ritual Challenge",
        101659: "Ruins of Charizel",
        101668: "Sabbat Threat",
        101671: "Sacrifice",
        101675: "Samuel Haight",
        101755: "Shared Nightmare",
        101756: "Shared Strength",
        101765: "Shepherd's Innocence",
        101782: "Silence of Death",
        101783: "Sins of the Cauchemar",
        101793: "Skin of the Adder",
        101830: "Soul Decoration",
        101853: "Spiritual Protector",
        101891: "Submachine Gun",
        101926: "Tainted Vitae",
        101962: "The Textbook Damnation",
        101980: "Threestar Cab Company",
        102008: "Tranquility",
        102017: "The Treatment",
        102028: "Trophy: Clan Respect",
        102029: "Trophy: Diablerie",
        102030: "Trophy: Discipline",
        102031: "Trophy: Domain",
        102032: "Trophy: Hunting Ground",
        102035: "Trophy: Progeny",
        102036: "Trophy: Retainers",
        102038: "Trophy: Safe Passage",
        102039: "Trophy: Wealth",
        102042: "Trumped-Up Charges",
        102046: "Tumnimos",
        102067: "Undue Influence",
        102069: "Undying Thirst",
        102090: "Vanish from the Mind's Eye",
        102098: "Veiled Sight",
        102147: "Warning Sirens",
        102168: "Weigh the Heart",
        102196: "Writ of Acceptance",
        200016: "Adrianne (G1)",
        200022: "Agrippina (G1)",
        200045: "Aleph (G1)",
        200050: "Alexis Sorokin (G4)",
        200057: "Allonzo Montoya (G3)",
        200065: "Amaravati (G4)",
        200069: "Ambrogino Giovanni (G2 ADV)",
        200077: "Anastasia Grey (G1)",
        200083: "Andreas, The Bard of Crete (G1)",
        200088: "Angel (G1)",
        200095: "Angus the Unruled (G1)",
        200099: "Ankh-sen-Sutekh (G4)",
        200120: "Anvil (G1)",
        200148: "Astrid Thomas (G1)",
        200154: "Azaneal (G4)",
        200159: "Badger (G1)",
        200174: "Basilia (G1)",
        200175: "Basir (G4)",
        200178: "Bear Paw (G1)",
        200189: "Benedict Giovanni, Agent (G4)",
        200196: "Bianca (G1)",
        200201: "Black Cat (G1)",
        200202: "Black Lotus (G4)",
        200217: "Brazil (G1)",
        200231: "Cagliostro, The Grand Copht (G4)",
        200237: "Callirus (G4)",
        200240: "Camille Devereux, The Raven (G1)",
        200243: "Cardano (G1)",
        200247: "Carmine Giovanni (G4)",
        200253: "Cassandra, Magus Prime (G1)",
        200271: "Chester DuBois (G1)",
        200286: "Colin Flynn (G1)",
        200293: "Count Germaine (G4 ADV)",
        200298: "Courtland Leighton (G1)",
        200300: "Cristobal Ghiberti (G4)",
        200301: "Cristofero Giovanni (G2)",
        200304: "Crusher (G1)",
        200313: "Dancin' Dana (G1)",
        200334: "Demetrius Slater (G1)",
        200335: "Democritus (G1)",
        200340: "Devyn (G4)",
        200343: "Didi Meyers (G1)",
        200345: "Dieter Kleist (G1)",
        200354: "Dollface (G1)",
        200362: "Don Cruez, The Idealist (G1)",
        200368: "Dorian Strack (G1)",
        200372: "Doyle Fincher (G4)",
        200375: "Dr. Jest (G1)",
        200376: "Dr. John Casey (G1)",
        200387: "Dre, Leader of the Cold Dawn (G1)",
        200392: "Duck (G1)",
        200395: "Dylan (G2 ADV)",
        200398: "Ebanezer Roush (G1)",
        200409: "Elihu (G4)",
        200411: "Eliott Sinclair, Virtuoso Thespian (G1)",
        200419: "Emerson Bridges (G1)",
        200438: "Ethan Locke (G2)",
        200461: "Felicia Mostrom (G1)",
        200516: "Gideon Fontaine (G1)",
        200517: "Gilbert Duane (G1)",
        200519: "Gillian Krader (G2)",
        200523: "Gitane St. Claire (G1)",
        200524: "Giuliano Vincenzi (G1)",
        200539: "Grendel the Worm-Eaten (G1)",
        200548: "Gunther, Beast Lord (G1)",
        200577: "Hasina Kesi (G1)",
        200579: "Heather Florent, The Opportunist (G1)",
        200581: "Hector Sosa (G1)",
        200588: "Helena Casimir (G1)",
        200623: "Ignatius (G1)",
        200625: "Igo the Hungry (G1)",
        200673: "Janey Pickman (G4)",
        200676: "Janni (G4)",
        200683: "Jayne Jonestown (G4)",
        200684: "Jazz Wentworth (G1)",
        200687: "Jeffrey Mullins (G4)",
        200701: "Jesús Alcalá (G3)",
        200706: "Jing Wei (G1)",
        200739: "Justine, Elder of Dallas (G1)",
        200746: "Kallista, Master Sculptor (G1)",
        200770: "Kemintiri (G2 ADV)",
        200789: "KoKo (G1)",
        200820: "Lazarus (G1)",
        200830: "Lena Rowe (G2)",
        200832: "Leon (G2)",
        200859: "Lorrie Dunsirn (G4)",
        200860: "Louhi (G4)",
        200869: "Luccia Paciola (G1)",
        200870: "Lucian (G1)",
        200876: "Lucretia, Cess Queen (G1)",
        200887: "Lupo (G1)",
        200892: "Lydia Van Cuelen (G1)",
        200898: "Magdelena Schaefer (G2)",
        200922: "Maria Stone (G3)",
        200929: "Mariel, Lady Thunder (G1)",
        200933: "Marion French (G4)",
        200945: "Marty Lechtansi (G1)",
        200950: "Masika (G1)",
        200955: "Mata Hari (G4)",
        200964: "Maureen, Dark Priestess (G4)",
        200973: "Melissa Barton (G1)",
        200979: "Merrill Molitor (G1)",
        200981: "Michael diCarlo (G4)",
        200993: "Miranda Sanova (G1)",
        201002: "Mitru the Hunter (G4)",
        201004: "Monique (G2)",
        201038: "Natasha Volfchek (G1)",
        201043: "Neferu (G4)",
        201059: "Nik (G1)",
        201065: "Normal (G1)",
        201094: "Pariah (G2)",
        201098: "Patrizia Giovanni, Collector of Secrets (G2)",
        201102: "Paul Forrest, False Prophet (G4)",
        201112: "Petaniqua (G4)",
        201113: "Petaniqua (G4 ADV)",
        201117: "Petru Sipos (G2)",
        201123: "Phillipe Rigaud (G4)",
        201124: "Pieter (G2)",
        201145: "Quinton McDonnell (G1)",
        201147: "Rabbat, The Sewer Goddess (G4)",
        201152: "Rafaele Giovanni (G2)",
        201155: "Rake (G1)",
        201156: "Ramiel DuPre (G1)",
        201163: "Ransam, The Old Man (G4)",
        201183: "Reverend Djoser Jones (G4)",
        201184: "Rex, The Necronomist (G2)",
        201188: "Ricki Van Demsi (G1)",
        201199: "Roland Bishop (G1)",
        201200: "Roland Loussarian (G1)",
        201201: "Roman Alexander (G1)",
        201202: "Roreca Quaid (G1)",
        201207: "Roxanne, Rectrix of the 13th Floor (G1)",
        201210: "Rufina Soledad (G1)",
        201217: "Sabine Lafitte (G1)",
        201221: "Sahira Siraj (G4)",
        201230: "Sammy (G1)",
        201238: "Sarah Cobbler (G1)",
        201256: "Sebastian Marley (G1)",
        201275: "Shane Grimald (G2)",
        201292: "Sir Walter Nash (G1)",
        201296: "Skryta Zyleta (G3)",
        201306: "Sonja Blue (G4)",
        201329: "Sylvester Simms (G1)",
        201345: "Tariq, The Silent (G2 ADV)",
        201348: "Tatiana Stepanova, Alastor (G4)",
        201355: "Tereza Rostas (G2)",
        201368: "Thomas Thorne (G1)",
        201371: "Tiberius, The Scandalmonger (G1)",
        201372: "Timothy Crowley (G1)",
        201394: "Tura Vaughn (G1)",
        201395: "Tusk, The Talebearer (G1)",
        201404: "Ulugh Beg, The Watcher (G1)",
        201405: "Uma Hatch (G1)",
        201415: "Uriah Winter (G1)",
        201421: "Valerius Maior, Hell's Fool (G4)",
        201422: "Valerius Maior, Hell's Fool (G4 ADV)",
        201426: "Vardar Vardarian (G4)",
        201450: "Violette Prentiss (G1)",
        201457: "Vliam Andor (G1)",
        201476: "Wynn (G1)",
        201493: "Yuri, The Talon (G1)",
        201498: "Zack North (G1)",
        201503: "Zebulon (G1)",
    },
    # Round 34: Black Hand and Camarilla Edition completed (owner,
    # 2026-11-09: "We aim to add all the cards ultimately. Let's work set
    # by set."; the cards board's lines).
    {
        100012: "Absolution of the Diabolist",
        100014: "Abyssal Hunter",
        100069: "Animal Magnetism",
        100097: "The Art of Memory",
        100104: "Ascendance",
        100122: "Babble",
        100214: "Blood to Water",
        100220: "Blooding",
        100225: "Bloodwork",
        100245: "Brainwash",
        100258: "Brothers Grimm",
        100269: "Bureaucratic Overload",
        100277: "Cairo Int'l Airport",
        100284: "Camarilla Exemplary",
        100302: "Carthage Remembered",
        100305: "Cat Burglary",
        100309: "Cauldron of Blood",
        100313: "Celestial Harmony",
        100319: "Chalice of Kinship",
        100346: "Chronicle of the Lost Tribe",
        100350: "Circumspect Revelation",
        100353: "Clan Impersonation",
        100382: "Command of the Harpies",
        100386: "Communal Haven: Temple",
        100427: "Corpse Balloon",
        100433: "Council of Seraphim",
        100442: "Creep Show",
        100454: "Crusade: Barcelona",
        100456: "Crusade: Brussels",
        100466: "Crusade: Miami",
        100477: "Cryptic Mission",
        100514: "Deep Cover Agent",
        100550: "Disarming Presence",
        100560: "Distraction",
        100574: "Dominion",
        100606: "Ebony Fox Hunt",
        100624: "Eldritch Glimmer",
        100676: "Extremis Boon",
        100738: "Fists of Death",
        100766: "Forest of Shadows",
        100811: "Gas-Powered Chainsaw",
        100836: "Glaser Rounds",
        100857: "Grenade",
        100872: "Guru",
        100898: "Hawg",
        100951: "Illegal Search and Seizure",
        100989: "Inscription",
        101034: "Kaymakli Barrier",
        101049: "Kindred Coercion",
        101054: "Kindred Society Games",
        101063: "The Knights",
        101074: "Last Stand",
        101096: "Lessons in the Steel",
        101099: "Lextalionis",
        101130: "Loyalist",
        101165: "Marijava Thuggee",
        101178: "Mask Empathy",
        101181: "Masque of Judas",
        101183: "Masquerade Endangered",
        101184: "Masquerade Enforcement",
        101190: "Meat Cleaver",
        101200: "Memory's Fading Glimpse",
        101205: "Mesmerize",
        101212: "Mind of a Child",
        101218: "Ministry",
        101228: "Mistrust",
        101301: "Nosferatu Performance Art",
        101315: "Of Noble Blood",
        101360: "Path of Death and the Soul",
        101361: "Path of Evil Revelations",
        101395: "Personal Scourge",
        101413: "Political Backlash",
        101422: "The Portrait",
        101432: "Powerbase: Berlin",
        101444: "Powerbase: Washington, D.C.",
        101452: "Praxis Seizure: Boston",
        101456: "Praxis Seizure: Cleveland",
        101457: "Praxis Seizure: Dallas",
        101469: "Praxis Seizure: Seattle",
        101476: "Precognizant Mobility",
        101502: "Protracted Investment",
        101516: "Pulling Strings",
        101540: "Random Patterns",
        101601: "Remover",
        101609: "Resilient Mind",
        101612: "Resplendent Protector",
        101621: "Reunion Kamut",
        101629: "Reversal of Fortunes",
        101642: "Ritual of the Bitter Rose",
        101702: "Scrying of Secrets",
        101723: "Seraph",
        101760: "Shattering Blow",
        101769: "Short-Term Investment",
        101770: "Shotgun Ritual",
        101799: "The Slashers",
        101823: "Sociopath",
        101825: "Song in the Dark",
        101839: "The Spawning Pool",
        101844: "Speed of Thought",
        101868: "Steam Tunnels",
        101909: "Surprise Influence",
        101971: "Thin-Blooded Seer",
        102007: "Trainer",
        102014: "Treachery",
        102064: "Under the Skin",
        102080: "Unnatural Disaster",
        102108: "Ventrue Directorate Assembly",
        102114: "Vial of Elder Vitae",
        102154: "Watch Commander",
        102155: "Watchtower: Chosen are Called",
        102157: "Watchtower: Greatest Fall",
        102167: "Weeping Stone",
        200033: "Akram (G2)",
        200064: "Amadeo (G2)",
        200075: "Ana Rita Montaña (G3)",
        200079: "Anatole, Prophet of Gehenna (G2)",
        200137: "Aristotle de Laurent (G3)",
        200143: "Artemis (G2)",
        200199: "Bindusara, Historian of the Kindred (G2)",
        200215: "Boy Toy (G2)",
        200216: "Brachah (G2)",
        200221: "Brooke (G2)",
        200225: "Bryan Van Duesen (G2)",
        200232: "Cailean (G2)",
        200234: "Calebos (G2)",
        200266: "Charice Fontaigne (G3)",
        200272: "Christanius Lionel, The Mad Chronicler (G2)",
        200287: "The Colonel (G3)",
        200289: "Constanza Vinti (G2)",
        200309: "Daliyah (G2)",
        200310: "Damaskenos, Herald of Leandro (G2)",
        200315: "Dani (G2)",
        200351: "Doctor Streck (G3)",
        200358: "Dominique (G2 ADV)",
        200364: "Dónal O'Connor (G2)",
        200367: "Donatien (G2)",
        200412: "Elisabetta Romano (G2)",
        200440: "Etrius (G2)",
        200449: "Ezekiel, Lord of Montreal (G3)",
        200529: "Goratrix (G2 ADV)",
        200538: "Greger Anderssen (G2)",
        200540: "Greta Kircher (G2)",
        200554: "Gwendolyn (G2)",
        200565: "Hannibal (G2)",
        200573: "Harrod (G2)",
        200583: "Heinrick Schlempt (G2)",
        200589: "Henri Lavenant (G3)",
        200590: "Henry Taylor (G3)",
        200616: "Ian Forestal (G2)",
        200617: "Ian Wallingford (G2)",
        200624: "Ignazio Giovanni (G3)",
        200631: "Imogen (G2)",
        200633: "Ingram Frizer (G3)",
        200647: "Itzahk Levine (G2)",
        200667: "Jalan-Aajav (G3)",
        200669: "Jan Pieterzoon (G2)",
        200680: "Javier Montoya (G2)",
        200719: "Josef (G2)",
        200732: "Judah (G2)",
        200766: "Kazimir Savostin (G3)",
        200771: "Kendrick (G2)",
        200782: "Kij Dansky (G2)",
        200788: "Klaus van der Veken (G2)",
        200796: "Kurt Strauss (G2)",
        200817: "Laurent de Valois (G2)",
        200899: "Makarios, The Seducer (G2)",
        200926: "Mariano Pomposo (G3)",
        200931: "Marino Reymundo Vásquez (G3)",
        200958: "Mateusz Gryzbowsky (G3)",
        200960: "Matteus, Flesh Sculptor (G2)",
        200987: "Miguel Santo Domingo (G3)",
        200989: "Miller Delmardigan, Teacher of Bahari (G2)",
        201017: "Muaziz, Archon of Ulugh Beg (G2)",
        201030: "Nahir (G3)",
        201034: "Nakova, Advocate of Golconda (G2)",
        201075: "Olga Triminov (G3)",
        201077: "Olivia (G2)",
        201126: "Piotr Andreikov (G3)",
        201149: "Radeyah (G2)",
        201162: "Ranjan Rishi, Camarilla Scholar (G2)",
        201169: "Raziya Samater (G2)",
        201171: "Rebekka, Chantry Elder of Munich (G2)",
        201174: "Regilio, The Seeker of Akhenaten (G2)",
        201182: "Reverend Blackwood (G2)",
        201198: "Roger Farnsworth (G3)",
        201204: "Rosa Martínez (G3)",
        201218: "Sabrina (G3)",
        201258: "Sébastien Goulet (G3 ADV)",
        201260: "Sela (G2 ADV)",
        201286: "Sigrid Bekker (G2)",
        201294: "Sister Evelyn (G3)",
        201304: "Soldat (G3)",
        201310: "Spiridonas (G2)",
        201333: "Szechenyi Jolán, Mother of Horrors (G3)",
        201354: "Teresita, The Godmother (G3)",
        201356: "Terrell Harding (G3)",
        201360: "Thelonius (G2)",
        201377: "Tock (G3)",
        201428: "Vasilis, The Traitor of Don Cruez (G2)",
        201443: "Victoria (G2)",
        201451: "Virgil (G2)",
        201458: "Volker, The Puppet Prince (G2)",
        201461: "Wah Chun-Yuen (G3)",
        201464: "Watenda (G2)",
        201468: "Wilhelm Waldburg (G2)",
        201483: "Yasmin the Black (G3)",
        201485: "Yazid Tamari (G3)",
        201487: "Yitzak (G3)",
        201489: "Yong-Sun, Harmonist (G2 ADV)",
    },
    # Round 35: Bloodlines and Final Nights completed (owner, 2026-11-09:
    # "We aim to add all the cards ultimately. Let's work set by set."; the
    # cards board's lines).
    {
        100033: "Akhenaten, The Sun Pharaoh",
        100042: "Amam the Devourer",
        100044: "Ambrosius, The Ferryman",
        100063: "The Ancestor's Talisman",
        100109: "Atonement",
        100127: "Baleful Doll",
        100132: "Banner of Neutrality",
        100173: "Blackmail",
        100183: "Blessing of Durga Syn",
        100192: "Blood Agony",
        100197: "Blood Clots",
        100205: "Blood of the Cobra",
        100211: "Blood Sweat",
        100235: "Bond with the Mountain",
        100254: "Brigitte Gebauer",
        100291: "Canopic Jar",
        100354: "Clan Loyalty",
        100368: "Coagulate Blood",
        100397: "Condemnation: Betrayed",
        100398: "Condemnation: Doomed",
        100399: "Condemnation: Languid",
        100420: "Contract",
        100430: "Corruption",
        100447: "Crimson Fury",
        100486: "Daemonic Possession",
        100488: "Dagon's Call",
        100523: "Denial of Aphrodite's Favor",
        100553: "Disease",
        100556: "Dismemberment of Osiris",
        100578: "Draba",
        100584: "Draught of the Soul",
        100614: "Edge Vitiation",
        100625: "Elemental Stoicism",
        100658: "Erosion",
        100659: "Eruption of Vitae",
        100675: "Extortion",
        100677: "Exuding Blood",
        100702: "Far Fatuus",
        100718: "Feline Saboteur",
        100719: "Felix \"Fix\" Hessian",
        100746: "Fleecing the Gaje",
        100773: "Form of the Serpent",
        100822: "The Ghouls of Plaza Morería",
        100833: "Giuseppe, Gravedigger",
        100848: "Grand Temple of Set",
        100851: "Grasp the Ghostly",
        100864: "Guard Duty",
        100893: "Haunt",
        100906: "Heartblood of the Clan",
        100935: "Horrid Reality",
        100954: "Illusions of the Kindred",
        100962: "Imposing Phantasm",
        100968: "Inbase Discotek, Frankfurt",
        101009: "Iron Heart",
        101021: "Jar the Soul",
        101045: "Khabar: Loyalty",
        101057: "King of the Mountain",
        101094: "Leonardo, Mortician",
        101134: "Lure of the Serpent",
        101138: "Madman's Quill",
        101159: "Malleable Visage",
        101166: "Mark of Damnation",
        101182: "Masquer",
        101185: "Mass Reality",
        101187: "The Mausoleum, Venice",
        101252: "Mummy's Tongue",
        101373: "The Path of Typhon",
        101376: "Patshiv",
        101399: "Phobia",
        101441: "Powerbase: Rome",
        101484: "Priestess of Sekhmet",
        101519: "Puppeteer",
        101525: "Putrefaction",
        101527: "Qetu the Evil Doer",
        101560: "Reality",
        101562: "The Realm of the Black Sun",
        101573: "Red Herring",
        101684: "Save Face",
        101721: "Sensory Deprivation",
        101722: "Sensory Overload",
        101730: "Set's Call",
        101742: "Shadow Parasite",
        101766: "Shilmulo Deception",
        101773: "Shroud Mastery",
        101806: "Sleight of Hand",
        101814: "Smoke Grenade",
        101834: "Soul Painting",
        101836: "Soul Stealing",
        101881: "The Stranger Among Us",
        101894: "Succulent Vitae",
        101898: "Summon Soul",
        101901: "Summon the Serpent",
        101981: "Thrones Crumble",
        101987: "Tomb of Rameses III",
        101988: "Tongue of the Serpent",
        101992: "Torment the Soul",
        101996: "Tortured Confession",
        102001: "Tower of London",
        102024: "The Trick of the Danya",
        102040: "True Faith",
        102091: "Vascular Explosion",
        102096: "Veil of Silence",
        102104: "Venenation",
        102105: "Venetian Conference",
        102122: "Violation of Trust",
        102161: "Wave of Lethargy",
        102210: "Zombie",
        200001: "Aabbt Kindred (G2)",
        200004: "Abd al-Rashid (G2)",
        200019: "Agaitas, The Scholar of Antiquities (G2)",
        200029: "Aisha (G2)",
        200035: "Al-Ashrad, Amr of Alamut (G2)",
        200068: "Ambrogino Giovanni (G2)",
        200074: "Amisa (G2)",
        200082: "Andrea Giovanni (G2)",
        200091: "Angela Preston (G2)",
        200096: "Anisa Marianna Lopez (G2)",
        200100: "Ankla Hotep (G2)",
        200121: "Anwar (G2)",
        200149: "Augustus Giovanni (G2)",
        200155: "Aziz, Dammar of Istanbul (G2)",
        200161: "Badr al-Budur (G2)",
        200162: "Bajazet al-Nasir (G2)",
        200180: "Béatrice L'Angou (G2)",
        200206: "Blanche Hill (G2)",
        200245: "Carlotta Giovanni (G2)",
        200258: "Céleste, The Voice of a Secret (G2)",
        200260: "Celine Chevalier (G2)",
        200268: "Chas Giovanni Tello (G2)",
        200295: "Count Ormonde (G2)",
        200306: "Cynthia Ingold (G2)",
        200318: "Danny Larkshill (G2)",
        200320: "Darius Styx (G2)",
        200328: "Dedefra (G2)",
        200331: "Delilah Monroe (G2)",
        200369: "Doris McMillon (G2)",
        200405: "Egothha (G2)",
        200425: "Enzo Giovanni, Pentex Board of Directors (G2)",
        200433: "Erinyi (G2)",
        200450: "Ezmerelda (G2)",
        200464: "Ferox, The Rock Lord (G2)",
        200466: "Fida'i (G2)",
        200467: "Fidus, The Shrunken Beast (G2)",
        200474: "Francesca Giovanni (G2)",
        200490: "Gabrin (G2)",
        200491: "Gaël Pilet (G2)",
        200495: "Gaspare Giovanni (G2)",
        200506: "George Frederick (G2)",
        200518: "Gillespi Giovanni (G2)",
        200522: "Gisela Harden, The Winnower (G2)",
        200525: "Gloria Giovanni (G2)",
        200551: "Gustav Mallenhous (G2)",
        200557: "Haakon Mortensen (G2)",
        200558: "Hadrian Garrick (G2)",
        200569: "Harika Guljan (G2)",
        200597: "High Priest Angra Mainyu (G2)",
        200607: "Howler (G2)",
        200611: "Huitzilopochtli (G2)",
        200614: "Husamettin (G2)",
        200637: "Intisar (G2)",
        200643: "Isabel Giovanni (G2)",
        200648: "Ivan Krenyenko (G2)",
        200653: "Jack Dawson (G2)",
        200666: "Jalal Sayad (G2)",
        200709: "Joaquina Amaya (G2)",
        200718: "Jorge De La Muerte, The Agent (G2)",
        200730: "Juanita Santiago (G2)",
        200735: "Julian Sanders (G2)",
        200745: "Kalinda (G2)",
        200751: "Kanimana Belghazi (G2)",
        200752: "Kanya Akhtar (G2)",
        200769: "Kemintiri (G2)",
        200773: "Kephamos, High Priest of Marrakech (G2)",
        200774: "Kervos, The Lieutenant (G2)",
        200777: "Khalil Anvari (G2)",
        200780: "Khay'tall, Snake of Eden (G2)",
        200792: "Kostantin, Baro of the Caravan (G2)",
        200793: "Krassimir (G2)",
        200809: "Lalitha (G2)",
        200843: "Lithrac (G2)",
        200883: "Luma, Stone Beauty (G2)",
        200925: "Marianna (G2)",
        200932: "Mario Giovanni (G2)",
        200948: "Mary the Black (G2)",
        200963: "Matthias (G2)",
        200965: "Max Lowell (G2)",
        200969: "Melek (G2)",
        200972: "Melisande (G2)",
        200995: "Miriam Benyona (G2)",
        201025: "Muse (G2)",
        201037: "Natalia (G2)",
        201042: "Nefertiti (G2)",
        201045: "Nehsi (G2)",
        201047: "Nepata (G2)",
        201051: "Nettie Hale (G2)",
        201071: "Obsidian (G2)",
        201095: "Parmenides (G2)",
        201096: "Parnassus (G2)",
        201101: "Paul DiCarlo, The Alpha (G2)",
        201127: "Pochtli (G2)",
        201136: "Pugfar (G2)",
        201137: "Qadir ul-Ghani (G2)",
        201143: "Qufur am-Heru (G2)",
        201153: "Raful al-Zarqa (G2)",
        201173: "Reg Driscoll (G2)",
        201175: "Regina Giovanni, The Right Hand of Augustus (G2)",
        201209: "Rudolfo Giovanni (G2)",
        201229: "Samat Ramal-Ra, Archon (G2)",
        201236: "Saqqaf, Keeper of the Grand Temple of Set (G2)",
        201240: "Sargon (G2)",
        201241: "Sarisha Veliku (G2)",
        201242: "Sarrasine (G2)",
        201249: "Saxum, Master of Slaves (G2)",
        201285: "The Siamese (G2)",
        201288: "Silvia Giovanni (G2)",
        201290: "Sir Marriot D'Urban (G2)",
        201309: "Spider-Killer (G2)",
        201314: "Stefano Giovanni (G2)",
        201327: "Sutekh, The Dark God (G2)",
        201331: "Synesios (G2)",
        201340: "Tansu Bekir (G2)",
        201344: "Tariq, The Silent (G2)",
        201352: "Tegyrius, Vizier (G2)",
        201364: "Theron (G2)",
        201365: "Thetmes, Caliph of Alamut (G2)",
        201367: "Thomas Steed, The Angry (G2)",
        201400: "Ublo-Satha (G2)",
        201412: "Unre, Keeper of Golgotha (G2)",
        201413: "Ur-Shulgi, The Shepherd (G2)",
        201420: "Vaclav Petalengro (G2)",
        201455: "Vittorio Giovanni (G2)",
        201473: "Wolf Valentine (G2)",
        201495: "Yusuf, Scribe of Alamut (G2)",
        201500: "Zahir, Hand of the Silsila (G2)",
    },
]
ADDED_CARDS = {k: v for batch in ADDED_BATCHES for k, v in batch.items()}


# Bundles KRCG types "Precon" that are card packs, no deck (54 cards, two of
# each new card): in the pool, never a precon table's deck.
PACK_BUNDLES = {("V5", "SoNY"), ("V5", "NBr")}


def pool_sets(card):
    """`krcg.sets`, the PACK_BUNDLES no precon."""
    return krcg.sets(card, PACK_BUNDLES)


def in_pool(card):
    """A card of the engine pool: printed in a V5-era set, or an
    owner-added extra or addition."""
    return (
        bool(set(V5_SETS.values()) & krcg.set_codes(card))
        or card["id"] in EXTRA_CARDS
        or card["id"] in ADDED_CARDS
    )


# Anniversary decks (owner, 2026-10-05: each decklist reviewed before it
# trains; not every card of the bundle belongs to the deck). A deck is its
# set's cards minus the extras Black Chantry lists apart (25th: the
# 2019-06-04 announcement; 30th: the product page, as the VTES ONE blog), by
# id with the copies left out. Emitted once every card of the deck is in the
# pool. No Fall of London / Shadows of Berlin deck ever trains (owner): they
# are not real decks.
ANNIVERSARY_DECKS = {
    "Twenty-Fifth Anniversary": ("Reign of Stanislava", {
        100037: 1,  # Alamut
        100071: 1,  # The Ankara Citadel, Turkey
        100169: 1,  # Black Metamorphosis
        100286: 1,  # Camarilla Vitae Slave
        100651: 1,  # Entombment
        100720: 1,  # Femur of Toomler
        100770: 1,  # Form of Corruption
        100882: 1,  # Hand of Conrad
        100908: 1,  # Heidelberg Castle, Germany
        100932: 1,  # Homunculus
        101048: 1,  # Khobar Towers, Al-Khubar
        101091: 1,  # Legendary Vampire
        101103: 1,  # Life Boon
        101664: 1,  # Rutor's Hand
        101681: 1,  # The Sargon Fragment
        101781: 1,  # The Signet of King Saul
        101931: 1,  # Talbot's Chainsaw
        201407: 1,  # Una (G3)
        201522: 2,  # Grimgroth (G6)
    }),
    "Thirtieth Anniversary": ("The Endless Dance", {
        100038: 1,  # Alastor
        100060: 1,  # Anathema
        101500: 1,  # Protected Resources
        101515: 1,  # Pulled Fangs
        101973: 1,  # Third Tradition: Progeny
        102268: 1,  # Banu Haqim Justicar
        102272: 1,  # Lasombra Justicar
        201693: 2,  # Annabelle Triabell (G6)
        201695: 1,  # Dónal O'Connor (G6)
        201697: 2,  # Ian Carfax (G7)
        201698: 2,  # Juliet Parr (G7)
        201700: 2,  # Lucinde, Alastor (G7)
        201704: 2,  # Molly MacDonald (G7)
        201705: 2,  # Nikolaus Vermeulen (G6)
    }),
}


def anniversary_decks(cards):
    """The anniversary decks fully in the pool, in ANNIVERSARY_DECKS order,
    as (deck name, set, {card id: copies}): the set's copies minus its
    extras. Both lists are 20 cards and the decks 12 crypt / 88 library."""
    out = []
    for s, (name, extras) in ANNIVERSARY_DECKS.items():
        assert sum(extras.values()) == 20, s
        deck = {}
        for c in cards:
            n = sum(e.get("copies", 0) for e in pool_sets(c).get(s, []))
            n -= extras.get(c["id"], 0)
            assert n >= 0, (s, krcg.full_name(c))
            if n:
                deck[c["id"]] = (c, n)
        if not all(in_pool(c) for c, _ in deck.values()):
            continue
        crypt = sum(n for c, n in deck.values() if "Vampire" in c["types"])
        assert (crypt, sum(n for _, n in deck.values()) - crypt) == (12, 88), (s, crypt)
        out.append((name, s, {i: n for i, (_, n) in deck.items()}))
    return out


# The Sabbat V5 precons (KRCG's bundles, `krcg.sets` `precon`), each
# emitted once every card of it is in the pool; drawn by path for training
# (owner, 2026-10-05).
SABBAT_SET = "Sabbat V5"


def sabbat_decks(cards):
    """The Sabbat V5 precons fully in the pool, by deck name, as (deck name,
    set, {card id: copies}); 12 crypt / 77 library each."""
    decks = {}
    for c in cards:
        for e in pool_sets(c).get(SABBAT_SET, []):
            if "precon" in e:
                decks.setdefault(e["precon"], {})[c["id"]] = (c, e["copies"])
    # In the order they were completed (the batch of their last card), so
    # a deck added never moves one already emitted.
    batch = {k: i for i, b in enumerate(ADDED_BATCHES) for k in b}
    order = sorted(decks, key=lambda n: (max(batch.get(i, -1) for i in decks[n]), n))
    out = []
    for name in order:
        deck = decks[name]
        if not all(in_pool(c) for c, _ in deck.values()):
            continue
        crypt = sum(n for c, n in deck.values() if "Vampire" in c["types"])
        assert (crypt, sum(n for _, n in deck.values()) - crypt) == (12, 77), (name, crypt)
        out.append((name, SABBAT_SET, {i: n for i, (_, n) in deck.items()}))
    return out


def pool_order(cards, key):
    """Table order: the V5 pool (and EXTRA_CARDS) sorted by `key`, then the
    ADDED_CARDS batch by batch, by id within a batch."""
    batch = {k: i for i, b in enumerate(ADDED_BATCHES) for k in b}
    base = sorted((c for c in cards if c["id"] not in ADDED_CARDS), key=key)
    return base + sorted((c for c in cards if c["id"] in ADDED_CARDS),
                         key=lambda c: (batch[c["id"]], c["id"]))

# Phrases removed before window matching: riders and granted abilities that
# fire after the card's own play step (review convention: "riders firing after
# the card's own play step add no play window"; in-play granted abilities are
# the effects framework's concern).
RIDERS = [
    r"\d (optional )?(maneuver|press)s?\b[^.;]*",     # "with 1 optional press", "gets 1 maneuver…"
    # "As above, with an optional press" (Thrown Sewer Lid [POT]).
    r"with an optional (maneuver|press)\b[^.;]*",
    # A strike's own rider for the opposing minion if dodged (Pounce).
    r"gets an optional (maneuver|press)\b[^.;]*",
    r"with \d additional strikes?\b[^.]*",
    # "Strike: dodge, with an additional strike (limited)" (Acrobatics
    # [CEL]), as "with 1".
    r"(?<=dodge, )with an additional strike\b[^.]*",
    # A bar on its own play, no window (Stutter-Step [CEL]).
    r"not usable as an additional strike",
    r"\d maneuver or press",
    # Damage in the press step, no play there (Necrosis [THN], Dagon's Call).
    r"during the press step (this|each) round[^.]*",
    r"(cannot|can't) [^.]*",                              # "cannot maneuver…, press…"
    r"can (also )?strike(, ranged)?:[^.]*",                          # granted strike ability
    r"(can|could|may) prevent \d[^.]*",                   # granted prevention
    # A strike's own prevention of the next opposing hand strike (Forearm
    # Block): resolved with it, no window of its own.
    r"prevent \d damage from the opposing minion's next hand strike[^.]*",
    r"can burn this card to prevent[^.]*",                # granted prevention (Wall of Filth)
    # A bar the card in play grants, no prevention (Creeping Infection).
    r"burn this card to prevent this minion from blocking",
    # A granted ability, playable before any damage (Brother's Blood;
    # advanced rules 4.5.1).
    r"can burn his or her own blood to heal or prevent[^.]*",
    # A strike's own prevention, resolved with it, and the opposing
    # minion's burn against it (Stunt Cycle; Far Fatuus [CHI]'s "all
    # damage from opponent's strikes"): no window of their own.
    r"\band prevent (\d|all) damage from (the opposing minion's|opponent's) strike[^.]*",
    r"can burn \d blood to prevent all damage from this strike",
    # A card in play burned for votes (Propaganda of the Deed): no play
    # window.
    r"can burn this card during the polling step[^.]*",
    r"but to prevent \d[^.]*",                            # its [PRO] "as above, but to prevent 2"
    # A granted intercept ability ("can burn 1 life to get +1 intercept"),
    # not a reaction's own "this vampire burns 1 blood to get +1 intercept"
    # (Aura Absorption [aus]).
    r"\b(can|may) [^.]*?to get \+\d intercept",
    r"skip the determine range step",
    # A weapon's own maneuver once in play (Zip Gun): no play window.
    r"with an optional maneuver each combat",
    # A card in play pressing for its vampire each round (Trap, Conquer the
    # Beast): a rider, no play window.
    r"during the press step of each round this combat, this \w+ automatically burns 1 "
    r"(of those )?counters?[^.]*",
]

# Timing windows: an explicit printed play point. Generic windows: legality
# derived from the effect (stealth/intercept, bleed modifier, bleed reduction).
COMBAT_TIMING = [
    # "Before range is chosen" (Disguised Weapon) is the same moment.
    (r"before range is (determined|chosen)", "COMBAT_BEFORE_RANGE"),
    # After range, before strikes (Haymaker), as Immortal Grapple.
    (r"play after range is determined", "COMBAT_BEFORE_STRIKES"),
    # A gain read as it lands (The Oath), as Slake the Thirst's: a steal
    # resolving with the strikes, a Taste of Vitae at the end of the round.
    (r"gains blood from a taste of vitae or steals blood", "COMBAT_UNTIMED"),
    (r"gains blood from a taste of vitae or steals blood", "COMBAT_END_OF_ROUND"),
    # An aim, "only usable as this minion chooses a strike" (Target Vitals);
    # "usable when choosing a strike" (Disguised Weapon [OBF]).
    (r"as this minion chooses a strike|when choosing a strike", "COMBAT_STRIKE_DECLARATION"),
    # The strike order swapped before the first is chosen (Rapid Thought
    # [CEL]).
    (r"only usable during the choose-strike step", "COMBAT_STRIKE_DECLARATION"),
    # "Only usable if the opposing minion's strike inflicts aggravated damage
    # on this vampire" (Ignore the Searing Flames [ser]): the strike
    # resolved, its damage awaiting prevention.
    (r"strike inflicts aggravated damage on this vampire", "COMBAT_DAMAGE_PREVENTION"),
    (r"\bmaneuver\b", "COMBAT_DETERMINE_RANGE"),
    # "Grapple." is a label, no timing (Chave de Braço's strike): Immortal
    # Grapple's timing is its "before strikes are chosen".
    (r"before strikes are chosen", "COMBAT_BEFORE_STRIKES"),
    # A combat card's replacement of an ally's burn (Heaven's Gate [obe]):
    # Left for Dead's moment.
    (r"only usable (?:when|if) an ally would be burned in combat", "ALLY_WOULD_BURN_IN_COMBAT"),
    # "Usable after strikes have been chosen" (Sanguinary Wind [CEL]).
    (r"usable after strikes have been chosen", "COMBAT_BEFORE_RESOLUTION"),
    (r"additional strike", "COMBAT_ADDITIONAL_STRIKE"),
    (r"\bstrike(, ranged)?:", "COMBAT_STRIKE_DECLARATION"),
    # Played as a strike ("put this card on the opposing vampire as a hand
    # strike": Punish [COMBAT]).
    (r"\bas a (hand )?strike\b", "COMBAT_STRIKE_DECLARATION"),
    (r"\bprevent\b", "COMBAT_DAMAGE_PREVENTION"),
    (r"\bpress\b", "COMBAT_PRESS"),
    (r"end of (the |a |this )?round", "COMBAT_END_OF_ROUND"),
    # "Only usable if … the opposing minion is not [ready]": after the
    # round's damage, in the end-of-round window (Hell-for-Leather [obt]).
    (r"the opposing minion is not\.", "COMBAT_END_OF_ROUND"),
    (r"combat would end", "COMBAT_WOULD_END"),
    # "Only usable if combat is about to end" (Hunting the Quarry [cel]):
    # the same moment, before it ends.
    (r"combat is about to end", "COMBAT_WOULD_END"),
    # "Only usable when this vampire burns a minion … in combat"
    # (Predator's Transformation [ABO]); "if this vampire burns a vampire"
    # (Soul Stealing [NEC]): after the round's damage.
    (r"(when|if) this vampire burns a (minion|vampire)", "COMBAT_END_OF_ROUND"),
    # "Only usable if the opposing ally is being burned" (Orgy of Blood):
    # the round's damage done, the ally burning.
    (r"opposing ally is being burned", "COMBAT_END_OF_ROUND"),
    # "Only usable when the opposing vampire is going into torpor"
    # (Decapitate [POT]); "when the opposing vampire goes into torpor"
    # (Tortured Confession): after the preventions, the damage about to land.
    (r"opposing vampire (is going|goes) into torpor", "COMBAT_GOING_TO_TORPOR"),
    # "Only usable when the opposing vampire should go to torpor"
    # (Amaranth); "when this vampire should go into torpor" (Undead
    # Persistence [for]): the same moment.
    (r"vampire should go (in)?to torpor", "COMBAT_GOING_TO_TORPOR"),
    # "Only usable by a vampire being burned" (Ashes to Ashes [for]): the
    # same moment, its own burn about to land.
    (r"only usable by a vampire being burned", "COMBAT_GOING_TO_TORPOR"),
    # Ammo, "Only usable before resolution of a gun's strike" (Scattershot,
    # Caseless Rounds), a melee weapon's (Baal's Bloody Talons): the strike
    # chosen, before it resolves.
    (r"before resolution of a (gun|melee weapon)'s strike", "COMBAT_BEFORE_RESOLUTION"),
    # "Only usable if the opposing minion strikes with a weapon" (Darkling
    # Trickery [MYT]): the strike chosen, before it resolves.
    (r"only usable if the opposing minion strikes with a weapon", "COMBAT_BEFORE_RESOLUTION"),
    # "Only usable after this Blood Brother gains blood" (Slake the
    # Thirst): any step after the gain, the round's end too (Taste of
    # Vitae's gain; LSJ 20100206-2, rules-qa 94.2).
    (r"after this [^.]{0,30}gains blood", "COMBAT_UNTIMED"),
    (r"after this [^.]{0,30}gains blood", "COMBAT_END_OF_ROUND"),
]
COMBAT_DEFAULT = "COMBAT_UNTIMED"   # rules-qa 2.2: no printed step
# Spiritual Guidance, Reality Mirror: timed by the card they play, every
# combat step a combat card is played in, never another card's as-played
# window (owner, 2026-11-17: rules-qa 122.215).
ASH_PLAY = r"play a combat card from [^.]*ash heap as if from your hand"
ASH_PLAY_WINDOWS = ["COMBAT_BEFORE_RANGE", "COMBAT_DETERMINE_RANGE", "COMBAT_BEFORE_STRIKES",
                    "COMBAT_STRIKE_DECLARATION", "COMBAT_STRIKE_ANNOUNCED",
                    "COMBAT_BEFORE_RESOLUTION", "COMBAT_DAMAGE_PREVENTION",
                    "COMBAT_GOING_TO_TORPOR", "COMBAT_ADDITIONAL_STRIKE", "COMBAT_PRESS",
                    "COMBAT_END_OF_ROUND", "COMBAT_WOULD_END", "COMBAT_UNTIMED"]

MODIFIER_TIMING = [
    (r"as (an?|the|this)\b[^.]{0,60}?\bis announced", "ACTION_AS_ANNOUNCED"),
    # "Only usable when the action is announced" (Shell Game); "when the
    # action to recruit a mortal ally is announced" (Ghouled).
    # "Only usable when an action is announced" (The Grandest Trick).
    (r"when (an?|the) action\b[^.]{0,40}?\bis announced", "ACTION_AS_ANNOUNCED"),
    # "Only usable when an older non-Anarch vampire successfully blocks"
    # (Bear-Baiting): the block made, before its resolution.
    (r"when an? [^.]{0,40}?successfully blocks", "ACTION_BLOCKED"),
    # "Only usable at the end of a successful action (after resolving the
    # action)" (Sleep Unseen); not an effect's moment ("the acting vampire
    # goes to torpor after resolving the action", Day Operation).
    (r"(?<!goes to torpor )after resolving the action", "ACTION_AFTER_RESOLUTION"),
    # "… another one who has just been blocked, before block resolution"
    # (Claiming the Body).
    (r"has just been blocked", "ACTION_BLOCKED"),
    # "Only usable when an action is blocked" (Conscripted Statue); "when
    # an action to equip with a weapon from your hand is blocked" (Gift of
    # Bellona [VAL]).
    (r"only usable when an action\b[^.]{0,60}?\bis blocked", "ACTION_BLOCKED"),
    # "As above, but usable as an action modifier" (Slake the Thirst
    # [SAN]); "As above, but usable any time before the action is
    # resolved" (Walk of Caine [SAN]).
    (r"usable as an action modifier|any time before the action is resolved", "ACTION_DURING"),
    # "Only usable after this Blood Brother gains blood" (Slake the Thirst
    # [SAN]): whenever the gain comes, after resolution too (LSJ 20100325).
    (r"after this [^.]{0,30}gains blood", "ACTION_AFTER_RESOLUTION"),
    # … and after a diablerie, before its blood hunt (ANK 20201228).
    (r"after this [^.]{0,30}gains blood", "ACTION_AFTER_DIABLERIE"),
    (r"if a minion attempts to block", "ACTION_BLOCK_ATTEMPT"),
    # "As above, but usable if a minion is attempting to block" (Day
    # Operation [FOR]); "Only usable when a minion is attempting to block"
    # (Council of Seraphim).
    (r"usable (if|when) an? [^.]{0,40}?is attempting to block", "ACTION_BLOCK_ATTEMPT"),
    # "Only usable if an ally or younger vampire attempts to block"
    # (Crocodile's Tongue).
    (r"if an? [^.]{0,40}? attempts to block", "ACTION_BLOCK_ATTEMPT"),
    # "Only usable when a vampire who does not have Fortitude attempts to
    # block" (The Kiss of Ra).
    (r"when an? [^.]{0,60}? attempts to block", "ACTION_BLOCK_ATTEMPT"),
    # "… burns a minion … as a Ⓓ action" (Predator's Transformation [ABO]);
    # "if this vampire burns a vampire" (Soul Stealing [nec]).
    (r"(when|if) this vampire burns a (minion|vampire)", "ACTION_AFTER_RESOLUTION"),
    # … and a skin played on a diablerie, before its blood hunt (Taking
    # the Skin: Minion, RTR 19991206).
    (r"skin\. play when this vampire burns a minion", "ACTION_AFTER_DIABLERIE"),
    # … and Soul Stealing's vampire burned by a diablerie (RTR 19991206).
    (r"if this vampire burns a vampire", "ACTION_AFTER_DIABLERIE"),
    (r"if (this|the acting) minion is blocked", "ACTION_BLOCKED"),
    # "when an acting Laibon you control is blocked (before combat)" (Make
    # an Example); "if an ally you control is blocked" (Malleable Visage).
    (r"(when an acting|if an?) \w+ you control is blocked", "ACTION_BLOCKED"),
    # "Only usable if this Anarch is blocked, before block resolution"
    # (Power of One [obt]); "Only usable when this vampire is blocked"
    # (Horrific Countenance); "… is successfully blocked by …" (Clan
    # Loyalty).
    (r"only usable (if|when) this \w+ is (successfully )?blocked", "ACTION_BLOCKED"),
    # "Only usable after a combat resulting from a block" (Shadow Boxing
    # [OBF][POT]).
    (r"after a combat resulting from a block", "ACTION_AFTER_COMBAT"),
    (r"only usable after a combat involving", "ACTION_AFTER_COMBAT"),
    # "Only usable after a combat between the acting minion you control and
    # a blocking minion" (Hidden Lurker).
    (r"only usable after a combat between", "ACTION_AFTER_COMBAT"),
    # "Usable after combat" (Pocket Out of Time [TEM]).
    (r"\busable after combat\b", "ACTION_AFTER_COMBAT"),
    (r"would be successful", "ACTION_WOULD_BE_SUCCESSFUL"),
    (r"only usable after (the )?(action )?resol", "ACTION_AFTER_RESOLUTION"),
    # "Play after resolving a successful action" (No Confidence).
    (r"play after resolving a successful action", "ACTION_AFTER_RESOLUTION"),
    # Dusk Work: "Play after resolving an action (successfully or not)".
    (r"play after resolving an action", "ACTION_AFTER_RESOLUTION"),
    # Edge of the World: the victory point is gained as the action resolves.
    (r"gained a victory point during this action", "ACTION_AFTER_RESOLUTION"),
    # "… is acting, after resolution" (Seraph's Second).
    (r"is acting, after resolution", "ACTION_AFTER_RESOLUTION"),
    # "Only usable when this anarch diablerizes" (Blade Clot).
    (r"when this anarch diablerizes", "ACTION_AFTER_DIABLERIE"),
    # "Only usable when this vampire successfully bleeds a Methuselah"
    # (Scrying of Secrets): the bleed resolved (its KRCG ruling).
    (r"only usable when this vampire successfully bleeds", "ACTION_AFTER_RESOLUTION"),
    # "Only usable after a successful directed action" (Marked Path).
    (r"only usable after a successful (directed )?action", "ACTION_AFTER_RESOLUTION"),
    # "Only usable on a successful referendum" (Cryptic Rider): after the
    # political action's resolution, its referendum passed.
    (r"only usable on a successful referendum", "ACTION_AFTER_RESOLUTION"),
    (r"polling step|during (a|the|this) referendum", "REFERENDUM"),
    # "Only usable if this vampire is about to enter combat with an ally"
    # (Gift of Sleep [obe]): Obedience's moments, for the acting vampire too.
    (r"about to enter combat with", "ACTION_BLOCKED"),
    (r"about to enter combat with", "ACTION_BLOCKS_DECLINED"),
    (r"about to enter combat with", "ACTION_QUEUED_COMBAT"),
    # Taking over the action: also after it is blocked, before the block
    # resolves (Mask of a Thousand Faces; LSJ 20010803).
    (r"the action continues with this vampire as the acting minion", "ACTION_BLOCKED"),
    (r"the action continues with this vampire as the acting minion", "ACTION_DURING"),
    # "Only usable at the end of a successful action directed at …"
    # (Esteem).
    (r"at the end of a successful action", "ACTION_AFTER_RESOLUTION"),
    # "… when combat involving another Gangrel antitribu you control ends"
    # (Follow the Alpha): the combat would end, as Coordinate Attacks.
    (r"when combat involving [^.]{0,60}?ends", "COMBAT_WOULD_END"),
    # "Only usable if an ally or younger vampire blocks, before block
    # resolution" (Psychomachia).
    (r"only usable if an ally or younger vampire blocks", "ACTION_BLOCKED"),
    # "Only usable when a diablerie action is successful" (Rebirth): before
    # its blood hunt.
    (r"when a diablerie action is successful", "ACTION_AFTER_DIABLERIE"),
    # "Only usable if an action card requiring Chimerstry is put in play"
    # (Permanency [CHI]): the action resolved.
    (r"only usable if an action card [^.]{0,40}?is put in play", "ACTION_AFTER_RESOLUTION"),
]
MODIFIER_GENERIC = [
    (r"\+\d+ stealth", "ACTION_STEALTH_INTERCEPT"),
    # "The blocking vampire gets -1 intercept" (Smoke and Mirrors [obt]);
    # "the blocking minion" (Suppressing Fire).
    (r"blocking (vampire|minion) gets -\d+ intercept", "ACTION_STEALTH_INTERCEPT"),
    (r"\+\d+ bleed|\+\d+ votes?", "ACTION_DURING"),
]
MODIFIER_DEFAULT = "ACTION_DURING"

REACTION_TIMING = [
    # Out-of-turn masters before a moment happens (crate::control): a
    # control change (The Diamond Thunderbolt), a passed blood hunt's burn
    # (Redline); "about to enter combat": as a block succeeds, blocks
    # declined, or the combat Psyche! queued (Obedience's windows).
    (r"control of a vampire you control is about to change", "CONTROL_ABOUT_TO_CHANGE"),
    (r"a vampire you control is about to enter combat", "ACTION_BLOCKED"),
    (r"a vampire you control is about to enter combat", "ACTION_BLOCKS_DECLINED"),
    (r"a vampire you control is about to enter combat", "ACTION_QUEUED_COMBAT"),
    (r"blood hunt referendum passes and would burn", "BLOOD_HUNT_WOULD_BURN"),
    # "Cancel a blood hunt called on a Gangrel antitribu" (Gangrel
    # Conspiracy): once it passed, before the burn (LSJ 20081203-2).
    (r"cancel a blood hunt", "BLOOD_HUNT_WOULD_BURN"),
    # "Only usable when a vampire is about to be burned by a blood hunt"
    # (Absolution of the Diabolist).
    (r"about to be burned by a blood hunt", "BLOOD_HUNT_WOULD_BURN"),
    # An ally about to be burned in combat, before it is (Left for Dead).
    (r"only usable (?:when|if) an ally would be burned in combat", "ALLY_WOULD_BURN_IN_COMBAT"),
    # A minion or retainer about to be burned, but by a diablerie (Consume
    # the Dead).
    (r"only usable when a minion or retainer is being burned", "MINION_WOULD_BURN"),
    # A vampire about to go to torpor, before it does (Minor Boon).
    (r"controlled by another methuselah is going into torpor", "GOING_TO_TORPOR"),
    # A block about to succeed, before the blocker locks (Lesser Boon).
    (r"would successfully block", "BLOCK_WOULD_SUCCEED"),
    # The influence phase, after a vampire moved to the ready region (Proxy
    # Kissed).
    (r"after you move an? \w+ from your uncontrolled region to your ready region",
     "INFLUENCED_OUT"),
    # In its player's own influence phase (Hanging Fermata).
    (r"only usable during your influence phase", "INFLUENCE_PHASE_OWN"),
    # "Under the control of another Methuselah during his or her unlock
    # phase" (Regarhagan's Hold).
    (r"another Methuselah during his or her unlock phase", "UNLOCK_PHASE_ANY"),
    (r"\bwakes?\b", "WAKE"),                                   # rules-qa 2.4
    # "Can play reaction cards and attempt to block as though unlocked"
    # (Fillip): a wake of the chosen vampire (advanced rules §5.2.3).
    (r"as though unlocked", "WAKE"),
    (r"unlocks?( and| to)? attempts? to block|unlock[^.]*; that \w+ attempts to block",
     "REACTION_UNLOCK_ATTEMPT"),
    (r"only usable[^.]*after block resolution", "ACTION_AFTER_COMBAT"),  # rules-qa 2.1
    # "Only usable after a combat between a blocking minion you control …"
    # (Fast Reaction).
    (r"only usable after a combat between", "ACTION_AFTER_COMBAT"),
    # "Only usable after a combat involving this Assamite and a minion with
    # a contract" (Provision of the Silsila).
    (r"only usable after a combat involving", "ACTION_AFTER_COMBAT"),
    # "… or is the target of a Camarilla vampire's directed action" (Promise
    # of 1528): any time in that action.
    (r"is the target of an? [^.]{0,40}?directed action", "REACTION_DURING_ACTION"),
    # An out-of-turn master as a ready titled vampire is burned or sent to
    # torpor (Leadership Vacuum).
    (r"when a ready titled vampire [^.]*is burned or sent to torpor", "READY_TITLED_LEFT"),
    (r"attempts? to block the current action", "ACTION_STEALTH_INTERCEPT"),  # block-loop play
    # A ready minion made to attempt (Guardian Vigil): until the block
    # succeeds (ANK 20180926).
    (r"\bthis \w+ attempts to block\.", "ACTION_STEALTH_INTERCEPT"),
    # "These two vampires attempt to block" (Coterie Tactics).
    (r"these two \w+ attempt to block", "ACTION_STEALTH_INTERCEPT"),
    (r"unlock this (vampire|minion)|burns? \d blood to unlock\.", "REACTION_UNLOCK"),
    # "Only usable by a locked vampire … This reacting vampire unlocks"
    # (Predator's Communion [ABO]).
    (r"\bthis (reacting )?(vampire|minion) unlocks\.", "REACTION_UNLOCK"),
    # "Only usable when another minion you control enters combat with a
    # minion without flight" (Winged Second): as the combat begins.
    (r"\benters combat with a minion\b", "COMBAT_BEFORE_RANGE"),
    # "Only usable when another vampire you control enters combat" (Legacy
    # of Power; ANK 20200109: played during combat).
    (r"only usable when another vampire you control enters combat", "COMBAT_BEFORE_RANGE"),
    # "… is about to successfully hunt" (Tainted Vitae): the hunt
    # unblocked, as Foul Blood.
    (r"about to successfully hunt", "ACTION_BLOCKS_DECLINED"),
    # "[AUS] Put this card on the acting vampire. You still control this
    # card." (Sins of the Cauchemar): no block needed (LSJ 19990611).
    (r"put this card on the acting vampire\. you still control this card", "REACTION_DURING_ACTION"),
    (r"after blocks are declined", "ACTION_BLOCKS_DECLINED"),
    # Another Methuselah's, after the acting one's own (Car Bomb; LSJ
    # 20081105).
    # Not "cancel an action modifier" (Shape Mastery [PRO]).
    (r"as (an?|the|this)\b[^.]{0,60}?\bis announced|cancel (an?|the|that) action(?! modifier)",
     "REACTION_AS_ANNOUNCED"),
    (r"polling step|during (a|the|this) referendum", "REFERENDUM"),
    (r"reduce (a|the) bleed", "REACTION_DURING_ACTION"),   # legality: a bleed against you
    # "Only usable if a minion controlled by your predator successfully
    # bleeds you" (Burnt Offerings [AUS]): the bleed resolved.
    (r"successfully bleeds you", "ACTION_AFTER_RESOLUTION"),
    # "Only usable when a referendum fails" (Political Backlash): after the
    # political action's resolution, its referendum failed.
    (r"only usable when a referendum fails", "ACTION_AFTER_RESOLUTION"),
    # "Only usable when a vampire successfully hunts" (XTC-Laced Blood): the
    # hunt resolved, its blood gained.
    (r"only usable when a vampire successfully hunts", "ACTION_AFTER_RESOLUTION"),
    # "Put this card on a vampire who successfully hunts" (Masquerade
    # Endangered): after the hunt resolved, like XTC-Laced Blood.
    (r"put this card on a vampire who successfully hunts", "ACTION_AFTER_RESOLUTION"),
    # "Only usable when a bleed against you is about to be successful"
    # (Strix [STR]): the bleed would succeed.
    (r"is about to be successful", "ACTION_WOULD_BE_SUCCESSFUL"),
    # "Only usable if a vampire would successfully hunt" (Foul Blood): the
    # hunt unblocked, before its blood is gained.
    (r"would successfully hunt", "ACTION_BLOCKS_DECLINED"),
    # "Play after resolution" (Touch of Pain); "… is successful, after
    # resolution" (Follow the Blood).
    (r"play after resolution|successful, after resolution", "ACTION_AFTER_RESOLUTION"),
    # "After resolution of a successful action directed at you" (Fata
    # Amria [CHI]).
    (r"after resolution of a successful (directed )?action", "ACTION_AFTER_RESOLUTION"),
    # A steal by an action against this vampire, resolved (The Oath).
    (r"gains blood from a taste of vitae or steals blood", "ACTION_AFTER_RESOLUTION"),
    # "Only usable if this vampire is about to enter combat with an acting
    # … vampire" (Obedience): the block made, before its resolution; or the
    # blocks declined, before the combat the action begins (owner,
    # 2026-10-08: rules-qa 55.7).
    (r"about to enter combat with", "ACTION_BLOCKED"),
    (r"about to enter combat with", "ACTION_BLOCKS_DECLINED"),
    # … or before a combat queued after a preceding one (LSJ 19991025).
    (r"about to enter combat with", "ACTION_QUEUED_COMBAT"),
    # "Only usable if this vampire blocks an ally or younger vampire,
    # before block resolution" (Mental Maze): the block made.
    # "Only usable if this vampire blocks, before block resolution"
    # (Diabolic Lure [dai]).
    # "… blocks an ally or younger vampire bleeding you, before block
    # resolution" (Shattered Mirror); "if a minion you control blocks,
    # before block resolution" (Shilmulo Deception).
    (r"if (this|an?) \w+ (you control )?blocks\b[^.]{0,60}?before block resolution",
     "ACTION_BLOCKED"),
    # "Only usable when this vampire successfully blocks …" (Minor
    # Irritation); "by a vampire who successfully blocks" (Veil of Silence).
    (r"(when this \w+|by a vampire who) successfully blocks", "ACTION_BLOCKED"),
    # "Not usable during combat" with no other timing (Angel of Berlin):
    # any time its player holds the impulse, outside combat (owner,
    # rules-qa 103.6).
    (r"\bnot usable during combat\.", "ANY_IMPULSE"),
    # An out-of-turn master's alternative in its player's own minion phase
    # (Emergency Preparations).
    (r"play this card during your minion phase", "ANY_IMPULSE"),
    # An out-of-turn master before range (Frenzy).
    (r"only usable before range is (determined|chosen)", "COMBAT_BEFORE_RANGE"),
    # An out-of-turn master on a combatant, any time in the combat (Battle
    # Frenzy); not Frenzy's, timed before range.
    (r"(?<!chosen\. )choose an? \w+ in combat", "COMBAT_UNTIMED"),
    # Warning Sirens: its intercept mode and its combat one, each timed.
    (r"play on an? \w+ targeted by a directed action", "ACTION_STEALTH_INTERCEPT"),
    (r"play on an? \w+ in combat", "COMBAT_UNTIMED"),
    # An out-of-turn master triggered by the opposing strike (Rötschreck),
    # before or after the vampire's own strike (LSJ 19990217).
    (r"attempts to inflict aggravated damage", "COMBAT_STRIKE_ANNOUNCED"),
    (r"attempts to inflict aggravated damage", "COMBAT_BEFORE_RESOLUTION"),
    # An out-of-turn master saving a Methuselah from an oust (Life Boon).
    (r"methuselah with no pool to keep them in the game", "OUSTING"),
    # "Only usable when you are being ousted" (Extremis Boon).
    (r"only usable when you are being ousted", "OUSTING"),
]
REACTION_GENERIC = [
    # "+X intercept" (Gestalt) too.
    (r"\+(\d+|X) intercept", "ACTION_STEALTH_INTERCEPT"),
]
REACTION_DEFAULT = "REACTION_DURING_ACTION"

# Cancel-as-played as the card's *own* play (not a rider letting the opponent
# cancel "this card", not an in-play lock ability): opens CARD_AS_PLAYED.
# "Cancel a combat card played by the opposing minion that would restrict
# this anarch's choice of strikes this round as it is played"
# (Groundfighting): the clause runs long.
# "Only usable as the opposing minion plays a card … the played card is
# canceled" (Botched Move): the as-played window too.
CANCEL_AS_PLAYED = (r"cancel (?!this card)[^.]{0,160}?as it is (being )?played"
                    r"|as the opposing minion plays a card[\s\S]{0,300}?the played card is canceled")

FLAGS = [
    # Not a quoted directive another card ends (Black Hand Ritual).
    (r'(?<!")do not replace', "delayed_replacement"),
    # Not "a non-out-of-turn master card" (Bleeding the Vine).
    (r"(?<!non-)out-of-turn", "out_of_turn"),
    (r"\btrifle\b", "trifle"),
    (r"has cast (a )?(votes?|ballots?)", "refers_votes_cast"),
    (r"as if (the action were |it were )?unblocked|continue the action", "continues_action"),
    (r"at random", "randomness"),
    # "To represent a fledgling" (Legion), "to turn this card into a
    # Gargoyle" (Raw Recruit).
    (r"(becomes|represents) an? \d-capacity|becomes an? [^.]{0,30}?\bally\b|ally[^.]*to (your|the) ready region"
     r"|to represent a fledgling|to turn this card into an? \w+"
     # "… to represent a wraith ally" (Khazar's Diary).
     r"|to represent an? [^.]{0,30}?\bally\b",
     "token_entity"),
    (r"cancel [^.]{0,160}?as it is (being )?played"
     r"|as the opposing minion plays a card[\s\S]{0,300}?the played card is canceled", "cancel_as_played"),
    (r"change the target of the bleed", "bounce"),
    # "The monster is now bleeding your predator's predator" (Determine).
    (r"\bis now bleeding\b", "bounce"),
    (r"would (draw|replace) a card[^.]*instead", "draw_replacement"),
    (r"play cards from this (location|card) as if from (your )?hand", "alt_hand_source"),
]
# Card in play grants a minion action (Ⓓ actions, referendum-to-burn, granted
# hunt/equip variants); MPA/DPA-consuming abilities excluded; "as an action …
# is announced" timing clauses are not grants.
# "… as a vampire until the end of this action" (Mouthpiece) grants none.
# "Treated as a mortal ally for the duration of the action" (The Grandest
# Trick) is no action.
# "… as a +1 stealth action to get +1 strength" (Masai Blood Milk) grants one;
# "… action to recruit or employ" and "… action to give" name another's.
IN_PLAY_ACTION = (r"\bas an? (?!vampire\b)(?!mortal ally\b)[^.;]{0,40}?\baction\b"
                  r"(?! to (?:recruit|employ|give)\b)(?! modifier)"
                  r"(?![^.]* is announced)"
                  # "… as a +1 stealth action to turn this card into a
                  # Gargoyle" (Raw Recruit).
                  r"|\baction to turn this card into\b"
                  # "… as a +1 stealth action to represent a wraith ally"
                  # (Khazar's Diary).
                  r"|\baction to represent\b")

# "[cel] or [for]" (the Sabbat V5 layout): either Discipline, one mode
# (rules-qa 52.1); side by side, a combo (rules-qa 25.2).
MODE_LINE = re.compile(r"^(\[[^\]]+\]:?\s*(or\s+(?=\[))?)+")
TYPE_TAGS = {"COMBAT": "Combat", "ACTION MODIFIER": "Action Modifier",
             "REACTION": "Reaction", "ACTION": "Action",
             "POLITICAL ACTION": "Political Action"}
MODE_TABLES = {
    "Combat": (COMBAT_TIMING, [], COMBAT_DEFAULT),
    "Action Modifier": (MODIFIER_TIMING, MODIFIER_GENERIC, MODIFIER_DEFAULT),
    "Reaction": (REACTION_TIMING, REACTION_GENERIC, REACTION_DEFAULT),
}
ACTION_MODE = re.compile(r"^(\+\d stealth )?(Ⓓ )?action\b|^Ⓓ", re.I)


def split_modes(text):
    """Return (preamble, [(type_tags, raw_tags, mode_text), …]). `raw_tags`
    are the bracket tokens as printed (`dom`, `DOM`, `COMBAT`), case kept."""
    preamble, modes = [], []
    for line in text.split("\n"):
        m = MODE_LINE.match(line)
        if not m:
            (modes[-1][2] if modes else preamble).append(line)
            continue
        raw = [t.strip("[]") for t in re.findall(r"\[[^\]]+\]", m.group(0))]
        types = {TYPE_TAGS[t.upper()] for t in raw if t.upper() in TYPE_TAGS}
        modes.append((types, raw, [line[m.end():]]))
    if not modes:                       # no mode lines: the whole text is one mode
        modes, preamble = [(set(), [], preamble)], []
    pre = " ".join(preamble)
    out = []
    for t, raw, ls in modes:
        # "Or play when …": a second use, its own timing (Determine).
        for part in re.split(r"(?<=\.)\s+(?=Also usable |Or play when )", " ".join(ls)):
            out.append((t, raw, part))
    return pre, out


def strip_riders(text):
    for pat in RIDERS:
        text = re.sub(pat, " ", text, flags=re.I)
    return text


# Windows a combat card's effect words suggest, not its own timing when one
# is printed ("Only usable before range is determined").
GRANTED_LATER = {"COMBAT_ADDITIONAL_STRIKE", "COMBAT_PRESS", "COMBAT_DETERMINE_RANGE",
                 "COMBAT_DAMAGE_PREVENTION"}


def classify_mode(mtype, text, as_above=False):
    timing, generic, default = MODE_TABLES[mtype]
    hits = [w for pat, w in timing if re.search(pat, text, re.I)]
    # "Only usable before range is determined/chosen": no window from what
    # the effect grants for later (Octopod's strikes each round, Three's a
    # Crowd's press); an explicit timing stays (Disguised Weapon).
    if mtype == "Combat" and re.search(r"only usable before range is (determined|chosen)", text, re.I):
        hits = [w for w in hits if w not in GRANTED_LATER]
    # "Only usable at the end of a round if … combat is about to end"
    # (Coordinate Attacks): the would-end moment alone.
    if mtype == "Combat" and re.search(r"combat is about to end", text, re.I):
        hits = ["COMBAT_WOULD_END"]
    # A strike's "As above, and at the end of the round, if …" is its rider,
    # no play window (Chave de Braço [POT]).
    if mtype == "Combat" and re.search(r"and at the end of the round, if", text, re.I):
        hits = [w for w in hits if w != "COMBAT_END_OF_ROUND"]
    # "Play when you successfully inflict damage from a hand strike"
    # (Shoulder Drop), a grapple timed after the strike.
    if mtype == "Combat" and re.search(r"inflict damage from a hand strike", text, re.I):
        hits = ["COMBAT_ADDITIONAL_STRIKE"]
    # A printed moment alone: the gun's strike about to resolve (Caseless
    # Rounds' later strike), the torpor about to land (Undead Persistence's
    # later press).
    for only in ("COMBAT_BEFORE_RESOLUTION", "COMBAT_GOING_TO_TORPOR"):
        if mtype == "Combat" and only in hits:
            hits = [only]
    # "Play after range is determined" (Haymaker): that moment alone, the
    # strike it commits no play of its own.
    if mtype == "Combat" and re.search(r"play after range is determined", text, re.I):
        hits = ["COMBAT_BEFORE_STRIKES"]
    # "Only usable at close range, before strike resolution" (Blood to
    # Water): from range set to the strikes' resolution.
    if mtype == "Combat" and re.search(r"before strike resolution", text, re.I):
        hits = ["COMBAT_BEFORE_STRIKES", "COMBAT_BEFORE_RESOLUTION"]
    # "… a vampire being burned either by diablerie or while in combat with
    # this vampire" (Ritual of the Bitter Rose); "after this acting vampire
    # burns another vampire" (Draught of the Soul): the diablerie, or the
    # round's end as the opposing vampire burns.
    if re.search(r"being burned either by diablerie or while in combat"
                 r"|after this acting vampire burns another vampire", text, re.I):
        hits = ["ACTION_AFTER_DIABLERIE", "COMBAT_END_OF_ROUND"]
    # Blade Clot's combat half: timed by the diablerie, no combat step.
    if mtype == "Combat" and re.search(r"when this anarch diablerizes", text, re.I):
        hits = ["ACTION_AFTER_DIABLERIE"]
    if mtype == "Combat" and re.search(ASH_PLAY, text, re.I):
        hits = list(ASH_PLAY_WINDOWS)
    if "ACTION_AFTER_COMBAT" in hits:
        hits = [w for w in hits if w != "REACTION_UNLOCK"]
    if hits:
        return hits
    hits = [w for pat, w in generic if re.search(pat, text, re.I)]
    if as_above:
        return hits              # inherits the previous mode; no default of its own
    return hits or [default]


def windows_for(card):
    """Per-card union of windows and flags (the matrix rows)."""
    wins, flags = [], []
    for _, mwins, mflags in modes_for(card):
        wins += mwins
        flags += mflags
    return list(dict.fromkeys(wins)), list(dict.fromkeys(flags))


def modes_for(card):
    """Per-mode classification: [(raw tags, windows, flags)]. Card-level
    (type) windows go on the modes of that type — the action mode of an
    Action/Combat card, every mode of a single-type card; card-text flags go
    on every mode."""
    types = card.get("types", [])
    text = krcg.text(card)
    wins = []
    flags = [f for pat, f in FLAGS if re.search(pat, text, re.I)]
    # A condition naming an action ("sends a vampire to torpor in combat or
    # as a Ⓓ action": Code of Samiel) grants none.
    if re.search(
        IN_PLAY_ACTION,
        re.sub(r"[^.]*(announces?\b|burns a minion|in combat or as an? )[^.]*\.", "", text),
        re.I,
    ):
        flags.append("in_play_action")

    # A power is gained as a +1 stealth action, a conviction played onto an
    # imbued in the unlock phase (rulebook "Appendix: Imbued Rules" 2, 3):
    # the icons in their text time their abilities, not their play.
    if "Power" in types:
        return [(krcg.disciplines(card), ["ACTION_DECLARE_POWER"], [])]
    if "Conviction" in types:
        return [([], ["UNLOCK_CONVICTION"], [])]
    out_of_turn = "out_of_turn" in flags
    # "Alternatively, … as an out-of-turn master" (Mundane): a plain master
    # too.
    alternative = re.search(r"alternatively, [^.]*as an out-of-turn master", text, re.I)
    if "Master" in types and (not out_of_turn or alternative):
        wins.append("MASTER_PHASE")
    if "Event" in types:
        wins.append("DISCARD_PHASE")
    if "Political Action" in types:
        wins.append("ACTION_DECLARE_POLITICAL")
    if "Action" in types:
        wins.append("ACTION_DECLARE")
    if "Equipment" in types:
        wins.append("ACTION_DECLARE_EQUIP")
    if "Ally" in types:
        wins.append("ACTION_DECLARE_RECRUIT")
    if "Retainer" in types:
        wins.append("ACTION_DECLARE_EMPLOY")

    mode_types = [t for t in types if t in MODE_TABLES]
    if out_of_turn:
        mode_types.append("Reaction")   # out-of-turn masters are reaction-timed plays
    preamble, modes = split_modes(text)
    if not mode_types:
        return [(raw, list(dict.fromkeys(wins)), flags) for _, raw, _ in modes]

    prev = []
    prev_tags = None
    result = []
    for tags, raw, mtext in modes:
        raw_text = preamble + " " + mtext
        stripped = strip_riders(raw_text)
        # "As above, with an additional strike (limited)" over a strike
        # (Projectile [CEL]): played as the strike; over a non-strike
        # (Backstab [CEL]) the additional strike keeps its window.
        if "COMBAT_STRIKE_DECLARATION" in prev and \
                re.match(r"\s*as above, with an additional strike\b", mtext, re.I):
            stripped = re.sub(r"with an additional strike\b[^.]*", " ", stripped, flags=re.I)
        # "As above", or "As [dai][obf] above" (I am Legion).
        as_above = bool(re.match(r"\s*as (?:\[\w+\])*\s*above", mtext, re.I))
        # "As above" over a [COMBAT] mode, a [REACTION] with its own printed
        # timing (Fata Amria [CHI]): that timing alone.
        reaction_after_combat = as_above and prev_tags == {"Combat"} and \
            {t for t in tags if t in MODE_TABLES} == {"Reaction"}
        mwins = list(prev) if as_above else []
        # "As above, with +1 stealth" over a mode with no printed timing
        # (Mind of the Wilds [ANI][AUS]): the stealth only when needed
        # (rules-qa 3.2) — its stealth window alone (Mask of a Thousand
        # Faces keeps its take-over timings).
        if as_above and prev == ["ACTION_DURING"] and \
                re.match(r"\s*as above, with \+\d stealth\.?\s*$", mtext, re.I):
            mwins = ["ACTION_STEALTH_INTERCEPT"]
            prev = []
        action_mode = False
        if tags:
            mts = sorted(t for t in tags if t in MODE_TABLES)  # a set: a fixed order
            action_mode = bool(tags & {"Action", "Political Action"})
        elif "Action" in types and ACTION_MODE.match(mtext.strip()):
            mts = []                     # the action mode of an Action/Combat card
            action_mode = True
        elif {"Combat", "Action Modifier"} <= set(mode_types):
            # An untagged mode of an action modifier and combat card (Swift
            # Cover): a strike or combat effect is the combat card's.
            combat = re.search(r"\b(strike|combat|range|maneuver|press|damage)\b", mtext, re.I)
            mts = ["Combat" if combat else "Action Modifier"]
            # "Play when this vampire burns a minion" (Taking the Skin:
            # Minion): in combat or by the action, as Predator's
            # Transformation [ABO] (LSJ 20090922).
            if re.search(r"when this vampire burns a minion", mtext, re.I):
                mts = ["Action Modifier", "Combat"]
        else:
            mts = mode_types
        # Not an in-play lock ability ("Lock this card to cancel …"); a lock
        # paid as the play goes on is fine (Touch of Clarity: "… as it is
        # played … Lock this vampire").
        cancels = re.search(CANCEL_AS_PLAYED, raw_text, re.I) and \
            not re.search(r"lock this [^.]{0,40}?to cancel", raw_text, re.I)
        if cancels:
            mwins.append("CARD_AS_PLAYED")
        # "Maneuver or press or burn 1 blood to cancel … as it is played"
        # (Groundfighting): the alternatives keep their own timings.
        # "Alternatively, … play this card during your minion phase"
        # (Emergency Preparations), "Press, … Alternatively, burn 1 blood
        # to cancel" (Disengage), "Reduce a bleed … or burn a blood to cancel"
        # (Terra Incognita): its own timing too.
        if not cancels or re.search(r"\b(maneuver|press) or [^.]*\bcancel\b", raw_text, re.I) \
                or re.search(r"alternatively, [^.]*during your minion phase", raw_text, re.I) \
                or re.search(r"\balternatively, burn \d+ blood to cancel\b", raw_text, re.I) \
                or re.search(r"\bor burn (?:a|\d+) blood to cancel\b", raw_text, re.I):
            # "As above, and/or …" (I am Legion [DAI][OBF]): the added
            # effect alone is a play of its own, timed by its text.
            either = as_above and re.search(r"\band/or\b", mtext, re.I)
            for mt in mts:
                got = classify_mode(mt, stripped, as_above and not either)
                if reaction_after_combat and got:
                    mwins = []
                # "As [san] above, but with two additional strikes" over a
                # mode only usable before range (Octopod [SAN]).
                if as_above and mt == "Combat" and \
                        {"COMBAT_BEFORE_RANGE", "COMBAT_WOULD_END"} & set(prev):
                    got = [w for w in got if w not in GRANTED_LATER]
                mwins += got
        # "Only usable if this vampire would be burned" (Abandoning the
        # Flesh); "when this vampire is being diablerized" (Crimson Fury):
        # that moment alone, whatever its card types.
        if re.search(r"only usable (if this vampire would be burned"
                     r"|when this vampire is being diablerized)", raw_text, re.I):
            mwins = ["VAMPIRE_WOULD_BURN"]
        # "As above, but usable any time during combat" (Clockwerx [SAN]):
        # the inherited step lifted.
        if as_above and re.search(r"but usable any time during combat", mtext, re.I):
            mwins = [w for w in mwins if not w.startswith("COMBAT_")] + [COMBAT_DEFAULT]
        if "WAKE" in mwins:
            mwins.append("CARD_AS_PLAYED")          # rules-qa 2.4
        if "COMBAT_STRIKE_DECLARATION" in mwins and \
                re.search(r"with (?:\d|an) optional maneuver", raw_text, re.I):
            mwins.append("COMBAT_DETERMINE_RANGE")  # maneuver rider commits the strike
            flags.append("maneuver_rider")
            # "As above, with 1 optional maneuver" after a mode without one
            # (Slam [POT]): only at the maneuver — as the strike it is the
            # mode above for more (rules-qa 48.4).
            if as_above and "COMBAT_DETERMINE_RANGE" not in prev and \
                    re.match(r"\s*as above, with \d optional maneuver", mtext, re.I):
                mwins = [w for w in mwins if w != "COMBAT_STRIKE_DECLARATION"]
        mflags = list(flags)
        if "Reaction" in mts and re.search(r"(?<!only )usable by a locked", raw_text, re.I) \
                and not ({"WAKE", "REACTION_UNLOCK", "REACTION_UNLOCK_ATTEMPT"} & set(mwins)):
            mflags.append("usable_locked")
        # Type-level windows: on the action mode of an Action/Combat card,
        # else on every mode (single-type cards; masters with modes).
        multi = len(mode_types) + int("Action" in types) > 1 or (
            "Action" in types and mode_types)
        type_wins = wins if (action_mode or not multi) else []
        result.append((raw, list(dict.fromkeys(type_wins + mwins)), mflags))
        prev = mwins
        prev_tags = {t for t in tags if t in MODE_TABLES}
    return result


def rows_for(cards):
    codes = set(V5_SETS.values())
    for card in sorted(cards, key=krcg.full_name):
        if not in_pool(card):
            continue
        if {"Vampire", "Imbued"} & set(card.get("types", [])):
            continue
        in_precon = krcg.in_bundle(card, codes)
        wins, flags = windows_for(card)
        yield [str(card["id"]), krcg.full_name(card), "/".join(card.get("types", [])),
               "/".join(krcg.disciplines(card)),
               "yes" if in_precon else "no",
               ";".join(wins), ";".join(flags), "yes" if len(wins) > 2 else ""]


def check(rows, reviewed_path):
    reviewed = {r["id"]: r for r in csv.DictReader(open(reviewed_path, newline=""))}
    gen = {r[0]: r for r in rows}
    bad = 0
    for cid in sorted(set(gen) | set(reviewed), key=lambda c: gen.get(c, reviewed.get(c))[1]
                      if c in gen else reviewed[c]["name"]):
        if cid not in gen or cid not in reviewed:
            print(f"only in {'generator' if cid in gen else 'reviewed'}: {cid}")
            bad += 1
            continue
        g, r = gen[cid], reviewed[cid]
        for col, idx in (("windows", 5), ("flags", 6)):
            if set(filter(None, g[idx].split(";"))) != set(filter(None, r[col].split(";"))):
                print(f"{r['name']!r:40} {col}: gen={g[idx]}  reviewed={r[col]}")
                bad += 1
    print(f"# {len(gen)} cards, {bad} divergences from {reviewed_path}", file=sys.stderr)
    return bad


def main():
    args = sys.argv[1:]
    reviewed = None
    if "--check" in args:
        i = args.index("--check")
        reviewed = args[i + 1]
        del args[i:i + 2]
    cards = krcg.load_cards(args[0] if args else None)

    rows = list(rows_for(cards))
    if reviewed:
        sys.exit(1 if check(rows, reviewed) else 0)
    out = csv.writer(sys.stdout)
    out.writerow(["id", "name", "types", "disciplines", "in_precon",
                  "windows", "flags", "needs_review"])
    out.writerows(rows)
    print(f"# {len(rows)} library cards, {sum(r[7] == 'yes' for r in rows)} flagged for review",
          file=sys.stderr)


if __name__ == "__main__":
    main()
