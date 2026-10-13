// Node smoke test of the wasm package (docs/protocol.md, "Embedded"): a
// full game through `handle`, the human seat answered at random by option
// index, three bots; `seq` counts up; a stale answer is refused; a rejoin
// resends what followed. `wasm-pack build crates/wasm --target nodejs
// --out-dir ../../target/wasm-node && node crates/wasm/tests/smoke.mjs`.
import { createRequire } from 'node:module'
import { existsSync, readFileSync } from 'node:fs'
const require = createRequire(import.meta.url)
const pkg = require('../../../target/wasm-node/zvtes_wasm.js')

const decks = JSON.parse(pkg.catalog())
const bots = ['aggro_bleed', 'wall', 'aggro_bleed']
const spec = {
    seed: 7,
    time_limit: 40,
    client: 'smoke.mjs',
    seats: [0, 1, 2, 3].map(i => ({
        name: i == 0 ? 'You' : `Bot ${i}`,
        deck: decks[(i * 3) % decks.length][1],
        player: i == 0 ? { type: 'human' } : { type: 'bot', profile: bots[i - 1] },
    })),
}
if (pkg.checkDeck(JSON.stringify({ 1: 1 })) == 'null') throw new Error('bad deck accepted')

// Valid amounts for an allocation: 1 to the first `min_nonzero`, the rest
// to whoever has room.
function amounts(a) {
    const out = a.recipients.map((_, i) => (i < a.min_nonzero ? 1 : 0))
    let left = a.points - a.min_nonzero
    for (let i = 0; left > 0; i = (i + 1) % out.length) {
        if (a.recipients[i].cap == null || out[i] < a.recipients[i].cap) {
            out[i]++
            left--
        }
    }
    return out
}

/** Plays a game to its end; returns [over, messages, ms]. */
function play(game, check) {
    let seq = 0
    const send = cmd => {
        const outs = JSON.parse(game.handle(JSON.stringify(cmd)))
        for (const o of outs) {
            if (o.seq != seq + 1) throw new Error(`seq ${o.seq} after ${seq}`)
            seq = o.seq
        }
        return outs
    }
    let outs = send({ type: 'join' })
    if (outs[0].type != 'joined') throw new Error('joined first')
    let messages = outs.length
    const start = performance.now()
    for (let n = 0; ; n++) {
        const over = outs.find(o => o.type == 'over')
        if (over) return [over, messages, performance.now() - start]
        const d = outs.findLast(o => o.type == 'decision')
        if (!d) throw new Error(`no decision: ${JSON.stringify(outs.at(-1))}`)
        let cmd
        if (d.allocation && Math.random() < 0.5) {
            cmd = { type: 'allocate', decision: d.decision, amounts: amounts(d.allocation) }
        } else if (d.votes && Math.random() < 0.5) {
            cmd = { type: 'vote_all', decision: d.decision, favor: d.votes.favor > 0 }
        } else {
            const o = d.options[Math.floor(Math.random() * d.options.length)]
            cmd = { type: 'answer', decision: d.decision, option: o.index }
        }
        const before = seq
        outs = send(cmd)
        messages += outs.length
        if (outs[0].type == 'error') throw new Error(`refused: ${JSON.stringify(outs[0])}`)
        if (check && !outs.some(o => o.type == 'over')) {
            const again = send({ id: 1, ...cmd })
            if (again[0].type != 'error' || again[0].reply_to != 1) throw new Error('stale answer accepted')
            const resent = JSON.parse(game.handle(JSON.stringify({ type: 'join', after: before })))
            if (resent.length != outs.length + 1) throw new Error('rejoin did not resend')
        }
        if (n > 50000) throw new Error('no end')
    }
}

const game = new pkg.Game(JSON.stringify(spec))
const [over, messages] = play(game, true)
console.log(`over after ${messages} messages: ${over.reason.type}, vp_halves ${over.vp_halves}`)
const record = JSON.parse(game.record())
if (!record.result || !record.entries.length) throw new Error('no record')
console.log(`record: ${record.entries.length} entries, engine ${record.engine}`)

// With an exported policy ( `python -m train.export <checkpoint> target/policy.zvp` ) :
// a game against three network bots.
const policy = new URL('../../../target/policy.zvp', import.meta.url)
if (existsSync(policy)) {
    const networkSpec = {
        ...spec,
        seats: spec.seats.map((s, i) => (i == 0 ? s : { ...s, player: { type: 'bot', profile: 'network' } })),
    }
    let refused = false
    try { new pkg.Game(JSON.stringify(networkSpec)) } catch { refused = true }
    if (!refused) throw new Error('network bots accepted without a policy')
    const net = pkg.Game.withPolicy(JSON.stringify(networkSpec), readFileSync(policy))
    const [over, messages, ms] = play(net, false)
    const bot = JSON.parse(net.record()).entries.filter(e => e.type == 'decision' && e.by == 'bot').length
    console.log(`network bots: ${over.reason.type} after ${messages} messages (${(ms / Math.max(1, bot)).toFixed(1)} ms per bot decision)`)
} else {
    console.log('no target/policy.zvp: network bots not tested')
}
