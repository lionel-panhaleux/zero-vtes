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
Downloads https://static.krcg.org/data/vtes.json if no path is given.
`--check` prints every card whose generated windows/flags differ from the
reviewed matrix and exits non-zero if any do (convergence is expected to hold:
the generator encodes the review's conventions).
"""
import csv
import json
import re
import sys
import urllib.request

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
]
ADDED_CARDS = {k: v for batch in ADDED_BATCHES for k, v in batch.items()}


def in_pool(card):
    """A card of the engine pool: printed in a V5-era set, or an owner-added
    extra or addition."""
    return (
        bool(V5_SETS.keys() & set(card.get("ordered_sets", [])))
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
            n = sum(e.get("copies", 0) for e in c.get("sets", {}).get(s, []))
            n -= extras.get(c["id"], 0)
            assert n >= 0, (s, c["name"])
            if n:
                deck[c["id"]] = (c, n)
        if not all(in_pool(c) for c, _ in deck.values()):
            continue
        crypt = sum(n for c, n in deck.values() if "Vampire" in c["types"])
        assert (crypt, sum(n for _, n in deck.values()) - crypt) == (12, 88), (s, crypt)
        out.append((name, s, {i: n for i, (_, n) in deck.items()}))
    return out


# The Sabbat V5 precons (KRCG `precon` tags), each emitted once every card
# of it is in the pool; drawn by path for training (owner, 2026-10-05).
SABBAT_SET = "Sabbat V5"


def sabbat_decks(cards):
    """The Sabbat V5 precons fully in the pool, by deck name, as (deck name,
    set, {card id: copies}); 12 crypt / 77 library each."""
    decks = {}
    for c in cards:
        for e in c.get("sets", {}).get(SABBAT_SET, []):
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
    r"with \d additional strikes?\b[^.]*",
    r"\d maneuver or press",
    r"(cannot|can't) [^.]*",                              # "cannot maneuver…, press…"
    r"can (also )?strike(, ranged)?:[^.]*",                          # granted strike ability
    r"(can|could) prevent \d[^.]*",                       # granted prevention
    r"can burn this card to prevent[^.]*",                # granted prevention (Wall of Filth)
    # A card in play burned for votes (Propaganda of the Deed): no play
    # window.
    r"can burn this card during the polling step[^.]*",
    r"but to prevent \d[^.]*",                            # its [PRO] "as above, but to prevent 2"
    r"to get \+\d intercept",                             # granted intercept ability
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
    # An aim, "only usable as this minion chooses a strike" (Target Vitals);
    # "usable when choosing a strike" (Disguised Weapon [OBF]).
    (r"as this minion chooses a strike|when choosing a strike", "COMBAT_STRIKE_DECLARATION"),
    (r"\bmaneuver\b", "COMBAT_DETERMINE_RANGE"),
    (r"\bgrapple\b|before strikes are chosen", "COMBAT_BEFORE_STRIKES"),
    (r"additional strike", "COMBAT_ADDITIONAL_STRIKE"),
    (r"\bstrike(, ranged)?:", "COMBAT_STRIKE_DECLARATION"),
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
    # (Predator's Transformation [ABO]): after the round's damage.
    (r"when this vampire burns a minion", "COMBAT_END_OF_ROUND"),
    # "Only usable if the opposing ally is being burned" (Orgy of Blood):
    # the round's damage done, the ally burning.
    (r"opposing ally is being burned", "COMBAT_END_OF_ROUND"),
]
COMBAT_DEFAULT = "COMBAT_UNTIMED"   # rules-qa 2.2: no printed step

MODIFIER_TIMING = [
    (r"as (an?|the|this)\b[^.]{0,60}?\bis announced", "ACTION_AS_ANNOUNCED"),
    # "Only usable when the action is announced" (Shell Game).
    (r"when the action is announced", "ACTION_AS_ANNOUNCED"),
    # "… another one who has just been blocked, before block resolution"
    # (Claiming the Body).
    (r"has just been blocked", "ACTION_BLOCKED"),
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
    # "Only usable if an ally or younger vampire attempts to block"
    # (Crocodile's Tongue).
    (r"if an? [^.]{0,40}? attempts to block", "ACTION_BLOCK_ATTEMPT"),
    # "Only usable when a vampire who does not have Fortitude attempts to
    # block" (The Kiss of Ra).
    (r"when an? [^.]{0,60}? attempts to block", "ACTION_BLOCK_ATTEMPT"),
    # "… burns a minion … as a Ⓓ action" (Predator's Transformation [ABO]).
    (r"when this vampire burns a minion", "ACTION_AFTER_RESOLUTION"),
    (r"if (this|the acting) minion is blocked", "ACTION_BLOCKED"),
    # "Only usable if this Anarch is blocked, before block resolution"
    # (Power of One [obt]); "Only usable when this vampire is blocked"
    # (Horrific Countenance).
    (r"only usable (if|when) this \w+ is blocked", "ACTION_BLOCKED"),
    # "Only usable after a combat resulting from a block" (Shadow Boxing
    # [OBF][POT]).
    (r"after a combat resulting from a block", "ACTION_AFTER_COMBAT"),
    (r"would be successful", "ACTION_WOULD_BE_SUCCESSFUL"),
    (r"only usable after (the )?(action )?resol", "ACTION_AFTER_RESOLUTION"),
    # "Only usable after a successful directed action" (Marked Path).
    (r"only usable after a successful (directed )?action", "ACTION_AFTER_RESOLUTION"),
    # "Only usable on a successful referendum" (Cryptic Rider): after the
    # political action's resolution, its referendum passed.
    (r"only usable on a successful referendum", "ACTION_AFTER_RESOLUTION"),
    (r"polling step|during (a|the|this) referendum", "REFERENDUM"),
    # Taking over the action: also after it is blocked, before the block
    # resolves (Mask of a Thousand Faces; LSJ 20010803).
    (r"the action continues with this vampire as the acting minion", "ACTION_BLOCKED"),
    (r"the action continues with this vampire as the acting minion", "ACTION_DURING"),
]
MODIFIER_GENERIC = [
    (r"\+\d+ stealth", "ACTION_STEALTH_INTERCEPT"),
    (r"\+\d+ bleed|\+\d+ votes?", "ACTION_DURING"),
]
MODIFIER_DEFAULT = "ACTION_DURING"

REACTION_TIMING = [
    (r"\bwakes?\b", "WAKE"),                                   # rules-qa 2.4
    (r"unlocks?( and| to)? attempts? to block|unlock[^.]*; that \w+ attempts to block",
     "REACTION_UNLOCK_ATTEMPT"),
    (r"only usable[^.]*after block resolution", "ACTION_AFTER_COMBAT"),  # rules-qa 2.1
    (r"attempts? to block the current action", "ACTION_STEALTH_INTERCEPT"),  # block-loop play
    (r"unlock this (vampire|minion)|burns? \d blood to unlock\.", "REACTION_UNLOCK"),
    (r"after blocks are declined", "ACTION_BLOCKS_DECLINED"),
    (r"as (an?|the|this)\b[^.]{0,60}?\bis announced|cancel (an?|the|that) action",
     "ACTION_AS_ANNOUNCED"),
    (r"polling step|during (a|the|this) referendum", "REFERENDUM"),
    (r"reduce (a|the) bleed", "REACTION_DURING_ACTION"),   # legality: a bleed against you
    # "Only usable if this vampire is about to enter combat with an acting
    # … vampire" (Obedience): the block made, before its resolution; or the
    # blocks declined, before the combat the action begins (owner,
    # 2026-10-08: rules-qa 55.7).
    (r"about to enter combat with", "ACTION_BLOCKED"),
    (r"about to enter combat with", "ACTION_BLOCKS_DECLINED"),
    # "Only usable if this vampire blocks an ally or younger vampire,
    # before block resolution" (Mental Maze): the block made.
    (r"if this \w+ blocks [^.]{0,40}?before block resolution", "ACTION_BLOCKED"),
    # An out-of-turn master triggered by the opposing strike (Rötschreck).
    (r"attempts to inflict aggravated damage", "COMBAT_BEFORE_RESOLUTION"),
    # An out-of-turn master saving a Methuselah from an oust (Life Boon).
    (r"methuselah with no pool to keep them in the game", "OUSTING"),
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
CANCEL_AS_PLAYED = r"cancel (?!this card)[^.]{0,120}?as it is (being )?played"

FLAGS = [
    (r"do not replace", "delayed_replacement"),
    # Not "a non-out-of-turn master card" (Bleeding the Vine).
    (r"(?<!non-)out-of-turn", "out_of_turn"),
    (r"\btrifle\b", "trifle"),
    (r"has cast (a )?(votes?|ballots?)", "refers_votes_cast"),
    (r"as if (the action were |it were )?unblocked|continue the action", "continues_action"),
    (r"at random", "randomness"),
    (r"becomes an? \d-capacity|becomes an? [^.]{0,30}?\bally\b|ally[^.]*to (your|the) ready region",
     "token_entity"),
    (r"cancel [^.]{0,120}?as it is (being )?played", "cancel_as_played"),
    (r"change the target of the bleed", "bounce"),
    (r"would (draw|replace) a card[^.]*instead", "draw_replacement"),
    (r"play cards from this (location|card) as if from (your )?hand", "alt_hand_source"),
]
# Card in play grants a minion action (Ⓓ actions, referendum-to-burn, granted
# hunt/equip variants); MPA/DPA-consuming abilities excluded; "as an action …
# is announced" timing clauses are not grants.
IN_PLAY_ACTION = r"\bas an? [^.;]{0,40}?\baction\b(?! to )(?! modifier)(?![^.]* is announced)"

# "[cel] or [for]" (the Sabbat V5 layout): either Discipline, one mode
# (rules-qa 52.1); side by side, a combo (rules-qa 25.2).
MODE_LINE = re.compile(r"^(\[[^\]]+\]\s*(or\s+(?=\[))?)+")
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
        for part in re.split(r"(?<=\.)\s+(?=Also usable )", " ".join(ls)):
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
    text = card.get("card_text", "")
    wins = []
    flags = [f for pat, f in FLAGS if re.search(pat, text, re.I)]
    if re.search(IN_PLAY_ACTION, re.sub(r"[^.]*(announces?\b|burns a minion)[^.]*\.", "", text), re.I):
        flags.append("in_play_action")

    out_of_turn = "out_of_turn" in flags
    if "Master" in types and not out_of_turn:
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
    result = []
    for tags, raw, mtext in modes:
        raw_text = preamble + " " + mtext
        stripped = strip_riders(raw_text)
        # "As above", or "As [dai][obf] above" (I am Legion).
        as_above = bool(re.match(r"\s*as (?:\[\w+\])*\s*above", mtext, re.I))
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
        if not cancels or re.search(r"\b(maneuver|press) or [^.]*\bcancel\b", raw_text, re.I):
            # "As above, and/or …" (I am Legion [DAI][OBF]): the added
            # effect alone is a play of its own, timed by its text.
            either = as_above and re.search(r"\band/or\b", mtext, re.I)
            for mt in mts:
                got = classify_mode(mt, stripped, as_above and not either)
                # "As [san] above, but with two additional strikes" over a
                # mode only usable before range (Octopod [SAN]).
                if as_above and mt == "Combat" and \
                        {"COMBAT_BEFORE_RANGE", "COMBAT_WOULD_END"} & set(prev):
                    got = [w for w in got if w not in GRANTED_LATER]
                mwins += got
        if "WAKE" in mwins:
            mwins.append("CARD_AS_PLAYED")          # rules-qa 2.4
        if "COMBAT_STRIKE_DECLARATION" in mwins and \
                re.search(r"with \d optional maneuver", raw_text, re.I):
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
    return result


def rows_for(cards):
    codes = set(V5_SETS.values())
    for card in sorted(cards, key=lambda c: c.get("name", "")):
        if not in_pool(card):
            continue
        if "Vampire" in card.get("types", []):
            continue
        in_precon = any(
            p.split(":")[0] in codes and re.match(r"^P", p.split(":", 1)[1])
            for p in card.get("_set", "").split(", ") if ":" in p
        )
        wins, flags = windows_for(card)
        yield [str(card["id"]), card["name"], "/".join(card.get("types", [])),
               "/".join(card.get("disciplines", [])),
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
    if args:
        cards = json.load(open(args[0]))
    else:
        with urllib.request.urlopen("https://static.krcg.org/data/vtes.json") as f:
            cards = json.load(f)

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
