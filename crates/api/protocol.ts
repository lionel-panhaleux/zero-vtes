/* zvtes protocol 1.0.0 (docs/protocol.md): generated from
 * crates/api/schema.json by scripts/gen_ts.py, do not edit. */
export const PROTOCOL_VERSION = '1.0.0'

/**
 * A command as sent: an optional client id, echoed by an `error` it
 * causes.
 */
export type In = {
    id?: number | null
} & (
    | {
        after?: number | null
        token?: string | null
        type: 'join'
    }
    | {
        decision: number
        option: number
        type: 'answer'
    }
    | {
        amounts: number[]
        decision: number
        type: 'allocate'
    }
    | {
        decision: number
        favor: boolean
        type: 'vote_all'
    }
    | {
        decision: number
        type: 'undo'
    }
    | {
        type: 'settings'
    } & SettingsPatch
    | {
        type: 'leave'
    }
)

/**
 * A message as sent: numbered per seat from 1, so a client rejoining
 * `after` the last it received gets the rest.
 */
export type Out = {
    /** The client id of the command it answers (an `error`). */
    reply_to?: number | null
    seq: number
} & (
    | {
        type: 'joined'
    } & Joined
    | {
        events: Event[]
        type: 'events'
        view?: TableView | null
    }
    | {
        type: 'decision'
    } & Decision
    | {
        type: 'over'
    } & Over
    | {
        type: 'error'
    } & Error
)

export type GameSpec = {
    /** The client's self-description ("succubus-club 1.4"), recorded. */
    client?: string | null
    /** In seating order: 2 to 6. */
    seats: SeatSpec[]
    /**
     * The deal's seed; the host picks one when absent (and records it).
     * 32 bits: exact as a JSON number in every language.
     */
    seed?: number | null
    /** In player turns; the engine's default when absent. */
    time_limit?: number | null
}

/** Why a game spec is refused (`POST /games`, a wasm game's creation). */
export type SpecError = {
    error: 'seats'
    n: number
} | {
    cards: string[]
    error: 'unsupported'
    seat: number
} | {
    crypt: number
    error: 'deck_size'
    library: number
    seat: number
} | {
    error: 'unknown_card'
    id: string
    seat: number
} | {
    error: 'profile'
    profile: string
    seat: number
} | {
    error: 'humans'
    n: number
} | {
    error: 'time_limit'
    max: number
}

export type GameRecord = {
    /** Per bot seat, the profile resolved. */
    bots?: BotRecord[]
    ended?: number | null
    /** The engine's commit. */
    engine: string
    entries: Entry[]
    protocol: string
    result?: GameResult | null
    /**
     * The spec as played: the seed always set, human seats' names
     * replaced by "human".
     */
    spec: GameSpec
    /** Unix milliseconds. */
    started: number
}

export type ActionKind = 
    | 'bleed'
    | 'hunt'
    | 'leave_torpor'
    | 'rescue'
    | 'diablerie'
    | 'employ'
    | 'recruit'
    | 'political'
    | 'burn_card'
    | 'steal_card'
    | 'equip'
    | 'equip_from'
    | 'equip_found'
    | 'become_anarch'
    | 'card'
    | 'ability'
    | 'red_list_hunt'
    | 'unknown'

export type ActionOutcome = 'succeeds' | 'blocked' | 'fails' | 'fizzles' | 'ends'

/** Where an action stands. */
export type ActionStep = 
    | 'announced'
    | 'would_succeed'
    | 'blocked'
    | 'after_combat'
    | 'resolved'
    | 'open'
    | 'attempt'
    | 'declined'
    | 'diablerie'
    | 'unknown'

export type ActionView = {
    acting: number
    bleed?: number | null
    /** The Methuselah bled, for a bleed. */
    bleed_target?: number | null
    /** The minion attempting the block, or that blocked, and its intercept. */
    blocker?: number | null
    /** The action card, for a card action. */
    card?: Card | null
    /** The Methuselahs who declined to block. */
    declined?: number[]
    intercept?: number | null
    kind: ActionKind
    /** Its mode (Disciplines, level) and X. */
    mode?: Mode | null
    seat: number
    /** The card in play granting the action (a location's, an equipment's). */
    source?: number | null
    stealth: number
    step: ActionStep
    target?: Target | null
    x?: number | null
}

/** Points allocated to a Methuselah or a card in play. */
export type Allocated = {
    points: number
    to: Target
}

/**
 * An allocation of points under way, answered whole with
 * `Command::Allocate`: all `points` given, at most `cap` to a recipient,
 * at least `min_nonzero` recipients given some.
 */
export type Allocation = {
    min_nonzero: number
    points: number
    /** In the order `amounts` lists them. */
    recipients: Recipient[]
}

export type AllocationError = 'none' | 'length' | 'total' | 'cap' | 'min_nonzero'

export type BotRecord = {
    /** "network" or a scripted style. */
    kind: string
    profile: string
    seat: number
    /**
     * Its sampling seed: replaying uses the recorded options, the seed
     * re-simulates the bot.
     */
    seed: number
    temperature?: number | null
    /** The weights' SHA-256, for a network. */
    weights?: string | null
}

export type By = 'human' | 'bot' | 'order' | 'auto'

/**
 * A physical card: its copy where the viewer may follow it, its KRCG id
 * where the viewer sees its face. A card in another seat's hand, a
 * library or a crypt has neither (a card back).
 */
export type Card = {
    copy?: number | null
    /** A crypt card (else a library card), hidden or not. */
    crypt?: boolean
    id?: number | null
    /**
     * Its label among the copies of the same card from the same deck the
     * viewer sees (1, 2…: "Embrace #2"): the lowest number no other
     * visible copy holds, given as it comes into sight. With `id` only.
     */
    nth?: number | null
}

/**
 * A card an option names: its KRCG id when the viewer may know it, its
 * copy when it is one physical card (on the table; not a card in hand,
 * where copies alike are one option).
 */
export type CardRef = {
    copy?: number | null
    /** Played from cards stored on this card in play, not from the hand. */
    from?: number | null
    id?: number | null
}

/**
 * What identifies an option: two options of one decision never share a
 * key (copies alike in hand are one option). A record keeps each answer
 * by its key, with the engine's copy numbers: a later engine finds the
 * same option whatever its index or however it describes it.
 */
export type ChoiceKey = {
    /** Which of a card's abilities, when it has several. */
    ability?: number | null
    /** The action a minion takes (`take_action`). */
    action?: ActionKind | null
    /** The minion playing the card, using the ability, taking the action. */
    actor?: number | null
    /** An amount: blood, pool, votes, X, points. */
    amount?: number | null
    /** The blood an ability moves onto (+) or off (−) the vampire it is on. */
    blood?: number | null
    /** The card played, used, taken, discarded or named. */
    card?: CardRef | null
    /** For or against a referendum. */
    favor?: boolean | null
    kind: OptionKind
    /**
     * The minion the option is about otherwise (blocker, voter, payer,
     * the vampire an ability is on).
     */
    minion?: number | null
    /** The mode a card is played in (`play`). */
    mode?: Mode | null
    target?: Target | null
    /** A clan, a Discipline, a circle chosen: its snake_case id. */
    value?: string | null
    /** Where a card chosen by name comes from (a search). */
    zone?: ZoneRef | null
}

export type CombatSide = {
    maneuvers: number
    minion: number
    presses: number
    /**
     * Its strike once declared (the blocker chooses knowing the acting
     * minion's), kept shown once resolved until the next pair.
     */
    strike?: Strike | null
}

export type CombatView = {
    /** The acting minion first. */
    minions: number[]
    range?: Range | null
    round: number
    /** Each side, the acting minion's first. */
    sides: CombatSide[]
}

export type Decision = {
    allocation?: Allocation | null
    decision: number
    /**
     * The options shown (the seat's settings may hide some: indices keep
     * the engine's numbering).
     */
    options: OptionView[]
    /**
     * What is chosen, in English ("Camarilla's Iron Fist: choose the
     * Methuselah who gains 1 pool"), when the seat's settings ask for text.
     */
    prompt?: string | null
    seat: number
    /**
     * The card (or minion, by its card) whose terms or effect this
     * decision chooses, when it is one: the options' `effects` say what
     * each choice gets.
     */
    source?: CardRef | null
    /** `Command::Undo` would take back the seat's previous decision. */
    undo: boolean
    view: TableView
    votes?: VoteTotals | null
    window: Window
}

export type DisciplineLevel = {
    /** The KRCG abbreviation, lowercase: `dom`, `aus`… */
    discipline: string
    superior: boolean
}

/** One thing an option does, to whom. */
export type Effect = {
    /** A card named (to hand, discarded). */
    card?: number | null
    /** The minion or card in play it applies to. */
    copy?: number | null
    kind: EffectKind
    /** Signed: gained (+) or lost (−). */
    n?: number | null
    /** What a chosen minion or Methuselah becomes (`role`). */
    role?: Role | null
    /** The Methuselah it applies to (a pool, a hand). */
    seat?: number | null
    to?: Zone | null
}

export type EffectKind = 
    | 'pool'
    | 'blood'
    | 'counters'
    | 'damage'
    | 'burn'
    | 'lock'
    | 'unlock'
    | 'draw'
    | 'discard'
    | 'steal'
    | 'corruption'
    | 'move'
    | 'keep'
    | 'role'
    | 'unknown'

export type EndReason = {
    type: 'last_standing'
} | {
    type: 'time_limit'
} | {
    type: 'humans_ousted'
} | {
    seat: number
    type: 'abandoned'
} | {
    type: 'engine_error'
}

export type Entry = {
    /**
     * Milliseconds since the start, when the host received the
     * answer (a human's think time includes the client's pacing).
     */
    at: number
    by: By
    choice: ChoiceKey
    decision: number
    /** Every option offered was this choice. */
    forced?: boolean
    option: number
    seat: number
    /** The options the seat was shown, when its settings hid some. */
    shown?: number[] | null
    type: 'decision'
} | {
    at: number
    seat: number
    to: number
    type: 'undo'
} | {
    at: number
    seat: number
    settings: Settings
    type: 'settings'
} | {
    at: number
    error: Error
    seat: number
    type: 'refused'
}

export type Error = {
    error: 'stale'
    expected: number
} | {
    error: 'not_your_turn'
} | {
    error: 'option'
    index: number
} | {
    error: 'no_undo'
} | {
    error: 'allocation'
    reason: AllocationError
} | {
    error: 'no_votes'
} | {
    error: 'over'
} | {
    error: 'not_joined'
} | {
    error: 'malformed'
    reason: string
} | {
    error: 'rate_limited'
}

export type Event = {
    /** The event told in English, when the seat's settings ask for text. */
    text?: string | null
} & (
    | {
        choice: OptionView
        decision: number
        kind: 'decided'
        seat: number
        window: Window
    }
    | {
        decision: number
        kind: 'auto_passed'
        seat: number
        window: Window
    }
    | {
        kind: 'undone'
        seat: number
        to: number
    }
    | {
        active: number
        kind: 'turn'
        turn: number
    }
    | {
        crypt: boolean
        kind: 'draw'
        n: number
        seat: number
    }
    | {
        delta: number
        kind: 'pool'
        pool: number
        seat: number
    }
    | {
        blood: number
        delta: number
        kind: 'blood'
        minion: number
    }
    | {
        card: Card
        kind: 'enters_play'
        on?: number | null
        seat: number
    }
    | {
        card: Card
        kind: 'moves'
        on?: number | null
    }
    | {
        card: Card
        kind: 'leaves_play'
        to?: Zone | null
    }
    | {
        /** Goes to torpor (true) or leaves it. */
        enters: boolean
        kind: 'torpor'
        minion: number
    }
    | {
        kind: 'ousted'
        seat: number
    }
    | {
        kind: 'block_fails'
        minion: number
    }
    | {
        acting: number
        blocker?: number | null
        kind: 'action_ends'
        outcome: ActionOutcome
        seat: number
    }
    | {
        kind: 'referendum_ends'
        outcome: ReferendumOutcome
        votes_against: number
        votes_for: number
    }
    | {
        kind: 'combat_begins'
        minions: number[]
    }
    | {
        kind: 'strike_declared'
        minion: number
        strike: Strike
    }
    | {
        kind: 'strikes'
        pair: number
        range: Range
        round: number
        sides: StruckSide[]
    }
    | {
        kind: 'combat_ends'
    }
)

export type GameResult = {
    /** The engine's failure, when it failed: message, location, backtrace. */
    failure?: string | null
    reason: EndReason
    vp_halves: number[]
    winner?: number | null
}

export type InPlayView = {
    card: Card
    contested?: boolean
    controller: number
    counters?: number
    locked: boolean
    /** Cards stored on it (face down ones hidden to others), top last. */
    stored?: Card[]
}

/** The answer to `join`. */
export type Joined = {
    /** The engine's version: its commit. */
    engine: string
    game: string
    /** Seconds without a command before the game ends as abandoned. */
    idle_timeout?: number | null
    /** `PROTOCOL_VERSION` of the host. */
    protocol: string
    seat: number
    seats: SeatInfo[]
    settings: Settings
    view: TableView
}

/**
 * Public state on a minion beyond blood and cards: counters, marks,
 * effects lasting a while.
 */
export type Marker = {
    /** The card it comes from. */
    card?: number | null
    kind: MarkerKind
    n?: number | null
    /** The Methuselah it belongs to (corruption counters). */
    seat?: number | null
    /** English, when the seat's settings ask for text. */
    text?: string | null
}

/**
 * Public state on a minion beyond blood and cards (completed with
 * the host, A3).
 */
export type MarkerKind = 
    | 'hostage'
    | 'nightmare'
    | 'torture'
    | 'disease'
    | 'title_contested'
    | 'expelled'
    | 'corruption'
    | 'effect'
    | 'unknown'

export type MinionKind = 'vampire' | 'ally' | 'token' | 'created' | 'unknown'

/** A minion: named by its card's copy everywhere (options, events). */
export type MinionView = {
    /** Blood (a vampire) or life (an ally). */
    blood: number
    capacity?: number | null
    card: Card
    cards: InPlayView[]
    contested?: boolean
    kind: MinionKind
    locked: boolean
    markers?: Marker[]
    name: string
    /** The Methuselah whose ash heap it goes to, when not its controller. */
    owner?: number | null
    title?: Title | null
}

/**
 * A card's mode: the printed line it is played from, at which
 * Disciplines and levels, as which card type.
 */
export type Mode = {
    /**
     * The card type it is played as, for a card of several types:
     * `action`, `action_modifier`, `reaction`, `combat`, `reflex`…
     */
    card_type?: string | null
    /** Empty for a card without Discipline requirement. */
    disciplines?: DisciplineLevel[]
    index: number
    /**
     * The printed line (one line may split into several modes, told
     * apart by the option's `effects`).
     */
    line: number
}

/** What an option does (the engine's choice kinds). */
export type OptionKind = 
    | 'pass'
    | 'edge_pool'
    | 'pay_contest'
    | 'yield_contest'
    | 'play'
    | 'use_ability'
    | 'resolve_rider'
    | 'resolve_printed'
    | 'burn_in_play'
    | 'burn_pool'
    | 'burn_blood_from'
    | 'set_x'
    | 'discard'
    | 'find_card'
    | 'take_from'
    | 'slot'
    | 'ash_vampire'
    | 'draw_library'
    | 'draw_stored'
    | 'bottom'
    | 'take_equipment'
    | 'take_action'
    | 'declare_block'
    | 'pay_rescue'
    | 'cast_votes'
    | 'cast_edge'
    | 'cast_card_vote'
    | 'burn_for_vote'
    | 'allocate_to'
    | 'choose_seat'
    | 'choose_minion'
    | 'choose_clan'
    | 'choose_circle'
    | 'choose_discipline'
    | 'choose_card'
    | 'allocate_to_card'
    | 'transfer'
    | 'withdraw'
    | 'draw_crypt'
    | 'influence_out'
    | 'merge'
    | 'hand_strike'
    | 'use_maneuver'
    | 'use_press'
    | 'use_prevent'
    | 'pay_block_toll'
    | 'pay_to_cancel'
    | 'strike_combat_ends'
    | 'round_strike'
    | 'take_damage_after_resolution'
    | 'committed_strike'
    | 'burn_weapon_strike'
    | 'employ_from_hand'
    | 'burn_blood_for_intercept'
    | 'unlock_for_blood'
    | 'unlock_infernal'
    | 'continue_action'
    | 'gain_strength'
    | 'burn_blood_to_avoid'
    | 'burn_corruption_to_fail'
    | 'burn_corruption_to_unlock'
    | 'diablerise'
    | 'unknown'

/**
 * An option offered: its index (what an answer names), what it is, and
 * what it does.
 */
export type OptionView = {
    /** Which of a card's abilities, when it has several. */
    ability?: number | null
    /** The action a minion takes (`take_action`). */
    action?: ActionKind | null
    /** The minion playing the card, using the ability, taking the action. */
    actor?: number | null
    /** An amount: blood, pool, votes, X, points. */
    amount?: number | null
    /** The blood an ability moves onto (+) or off (−) the vampire it is on. */
    blood?: number | null
    /** The card played, used, taken, discarded or named. */
    card?: CardRef | null
    /**
     * What taking it does when its key does not say: a cost, what a
     * chosen Methuselah or minion gets, an ability's effects, what a
     * pass lets happen.
     */
    effects?: Effect[]
    /** For or against a referendum. */
    favor?: boolean | null
    /**
     * The engine's index: stable while the decision is pending, even when
     * the seat's settings hide other options.
     */
    index: number
    kind: OptionKind
    /**
     * The minion the option is about otherwise (blocker, voter, payer,
     * the vampire an ability is on).
     */
    minion?: number | null
    /** The mode a card is played in (`play`). */
    mode?: Mode | null
    target?: Target | null
    /** The option in English, when the seat's settings ask for text. */
    text?: string | null
    /** A clan, a Discipline, a circle chosen: its snake_case id. */
    value?: string | null
    /** Where a card chosen by name comes from (a search). */
    zone?: ZoneRef | null
}

export type Over = {
    reason: EndReason
    view: TableView
    /** Per seat, in halves of a victory point. */
    vp_halves: number[]
    /**
     * The game's winner: the most victory points, at least 2 of them,
     * alone.
     */
    winner?: number | null
}

/** The active Methuselah's phase. */
export type Phase = 'unlock' | 'master' | 'minion' | 'influence' | 'discard' | 'over' | 'unknown'

export type Player = {
    settings?: Settings
    type: 'human'
} | {
    profile: string
    type: 'bot'
}

export type Range = 'close' | 'long' | 'unknown'

export type Recipient = {
    cap?: number | null
    /** What one point does to it ("burns 1 pool": `pool`, `n` −1). */
    effects?: Effect[]
    to: Target
}

export type ReferendumOutcome = 'passes' | 'fails' | 'canceled'

export type ReferendumStep = 'terms' | 'polling' | 'shortfall' | 'choose' | 'unknown'

export type ReferendumView = {
    caller: number
    /** The calling minion, if any (a blood hunt's has none). */
    calling?: number | null
    /** The political card (or what calls it). */
    card?: number | null
    step: ReferendumStep
    terms: Terms
    /** Every vote cast so far, in order. */
    votes: Vote[]
    votes_against: number
    votes_for: number
}

/**
 * What a chosen minion or Methuselah becomes for the card choosing it
 * (completed with the host, A3).
 */
export type Role = 'does_not_unlock' | 'unblockable_non_bleed' | 'chosen' | 'unknown'

export type SeatInfo = {
    /** The bot profile; absent for a human. */
    bot?: string | null
    name: string
}

export type SeatSpec = {
    deck: Record<string, number>
    /**
     * Shown to the table; a human seat's is not recorded (records say
     * "human").
     */
    name: string
    player: Player
}

export type SeatView = {
    ash_heap: Card[]
    /** Top last. */
    crypt: Card[]
    edge: boolean
    hand: Card[]
    /** Cards in play the seat controls on no minion. */
    in_play: InPlayView[]
    /** Top last. */
    library: Card[]
    /** Cards the seat is playing, not resolved yet. */
    limbo: Card[]
    name: string
    ousted: boolean
    pool: number
    predator: number
    prey: number
    ready: MinionView[]
    removed: Card[]
    seat: number
    torpor: MinionView[]
    uncontrolled: SlotView[]
    /**
     * Victory points, in halves of a point (a survivor at the time limit
     * scores 1 half).
     */
    vp_halves: number
}

/**
 * A human seat's preferences (fields left out take their default);
 * changed during the game by `Command::Settings`.
 */
export type Settings = {
    /**
     * Asked about every opportunity (true), or only in its moment
     * (false): a wake outside an action that concerns the seat, a block
     * denial short of the action's stealth, an any-time ability where
     * nothing else is on offer are passed over; a decision left with a
     * pass alone passes by itself.
     */
    opportunities?: boolean
    /**
     * English renderings beside the data (`text` on options, events,
     * markers, strikes; a decision's `prompt`): for a client that does
     * not compose its own.
     */
    text?: boolean
    /**
     * Table views sent on every step (bots' included: a client
     * animating each move) or only with the seat's decisions and the
     * game's end.
     */
    views?: Views
}

/** Some settings to change; the others stay. */
export type SettingsPatch = {
    opportunities?: boolean | null
    text?: boolean | null
    views?: Views | null
}

export type SlotView = {
    /** A banished vampire, waiting to be influenced out again. */
    banished?: boolean
    card: Card
    /** The cards it carries there, out of play until it returns. */
    cards?: Card[]
    counters: number
}

export type Strike = {
    aggravated?: boolean
    /** The card it comes from (a weapon, a strike card). */
    card?: number | null
    kind: StrikeKind
    /** Damage or blood stolen. */
    n?: number | null
    ranged?: boolean
    /** Resolved this pair: the damage it dealt. */
    resolved?: number | null
    /** English, when the seat's settings ask for text. */
    text?: string | null
}

export type StrikeKind = 
    | 'hand'
    | 'melee'
    | 'dodge'
    | 'combat_ends'
    | 'damage'
    | 'steal_blood'
    | 'torpor_or_burn'
    | 'torpor'
    | 'burn_weapon'
    | 'burn_equipment'
    | 'ranged_burn_equipment'
    | 'corrupt'
    | 'card_effect'
    | 'unknown'

export type StruckSide = {
    damage: number
    minion: number
    strike?: Strike | null
}

export type TableView = {
    action?: ActionView | null
    active: number
    combat?: CombatView | null
    /** The next decision's number. */
    decision: number
    phase: Phase
    referendum?: ReferendumView | null
    seats: SeatView[]
    /** Game turn, from 1. */
    turn: number
    /** The viewing seat; absent: every card shown (records, tests). */
    viewer?: number | null
}

/**
 * What an action or an option is aimed at: a Methuselah, or a card on
 * the table (a minion, an uncontrolled vampire, a card in play).
 */
export type Target = {
    kind: 'seat'
    seat: number
} | {
    copy: number
    kind: 'card'
}

/** A referendum's terms chosen so far. */
export type Terms = {
    allocated?: Allocated[]
    cards?: number[]
    /** Snake_case clan id. */
    clan?: string | null
    seats?: number[]
}

/** A vampire's title. */
export type Title = {
    /**
     * `primogen`, `prince`, `baron`, `justicar`, `inner_circle`,
     * `archbishop`, `bishop`, `cardinal`, `priscus`, `regent`, `magaji`,
     * `independent` (named in `of`), `votes` (a title by its votes only).
     */
    id: string
    /**
     * The city (a prince, a baron, an archbishop), the clan (a justicar,
     * the Inner Circle) or an independent title's name.
     */
    of?: string | null
    /** Its votes, when the title says them (`independent`, `votes`). */
    votes?: number | null
}

export type Views = 'decisions' | 'steps'

export type Vote = {
    /** An abstention: not tallied. */
    canceled?: boolean
    /** The minion or card in play casting it. */
    copy?: number | null
    favor: boolean
    n: number
    seat: number
    source: VoteSource
}

/** Where votes come from. */
export type VoteSource = 'edge' | 'vampire' | 'printed' | 'card' | 'in_play' | 'pool' | 'unknown'

/**
 * The votes the seat can cast now each way, all at once with
 * `Command::VoteAll` (its vampires' and its calling card's).
 */
export type VoteTotals = {
    against: number
    favor: number
}

/** The window a decision is asked in. */
export type Window = 
    | 'unlock_abilities'
    | 'master_phase'
    | 'minion_phase'
    | 'action_as_announced'
    | 'action_after_resolution'
    | 'action_would_be_successful'
    | 'action_blocks_declined'
    | 'action_blocked'
    | 'action_after_combat'
    | 'torpor_block'
    | 'rescue_cost'
    | 'entry_effect'
    | 'block_toll'
    | 'distribution'
    | 'blood_hunt_polling'
    | 'referendum_terms'
    | 'referendum_polling'
    | 'discard_down'
    | 'action_terms'
    | 'library_search'
    | 'action_exchange'
    | 'draw'
    | 'bottom_cards'
    | 'influence_transfers'
    | 'discard_phase'
    | 'combat_before_range'
    | 'combat_determine_range'
    | 'combat_before_strikes'
    | 'combat_strike'
    | 'combat_before_resolution'
    | 'combat_prevention'
    | 'combat_prevention_allocation'
    | 'combat_additional_strikes'
    | 'combat_press'
    | 'combat_end_of_round'
    | 'play_terms'
    | 'rider_terms'
    | 'phase_impulse'
    | 'card_as_played'
    | 'action_block'
    | 'resolution_choice'
    | 'unknown'

/** A seat's zone. */
export type Zone = 
    | 'crypt'
    | 'library'
    | 'hand'
    | 'ash_heap'
    | 'removed'
    | 'uncontrolled'
    | 'ready'
    | 'torpor'
    | 'in_play'
    | 'limbo'
    | 'unknown'

export type ZoneRef = {
    seat: number
    zone: Zone
}
