// Node smoke test of the wasm package: a full game, seat 1 "human"
// answered at random, three bots; every message parses; a stale answer
// throws. `wasm-pack build crates/wasm --target nodejs --out-dir
// ../../target/wasm-node && node crates/wasm/tests/smoke.mjs`.
import { createRequire } from 'node:module'
const require = createRequire(import.meta.url)
const pkg = require('../../../target/wasm-node/zvtes_wasm.js')

const decks = JSON.parse(pkg.catalog())
const bots = ['aggro_bleed', 'wall', 'aggro_bleed']
const spec = {
    seed: 7,
    time_limit: 40,
    seats: [0, 1, 2, 3].map(i => ({
        name: i == 0 ? 'You' : `Bot ${i}`,
        deck: decks[(i * 3) % 14][1],
        player: i == 0 ? { kind: 'human' } : { kind: 'bot', style: bots[i - 1] },
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
    return Uint8Array.from(out)
}

const game = new pkg.Game(JSON.stringify(spec))
let messages = 0
for (;;) {
    const out = JSON.parse(game.advance())
    messages++
    if (out.events && !out.events.every(e => typeof e == 'string' && e)) throw new Error('bad events')
    if (out.type == 'over') {
        console.log(`over after ${messages} messages:`, out.vps)
        break
    }
    if (out.type == 'decision' && out.allocation) {
        const outs = JSON.parse(game.allocate(out.decision, amounts(out.allocation)))
        if (!outs.length) throw new Error('an allocation answered by nothing')
        continue
    }
    if (out.type == 'decision' && out.votes && Math.random() < 0.5) {
        JSON.parse(game.voteAll(out.decision, out.votes.favor > 0))
        continue
    }
    if (out.type == 'decision') {
        const i = Math.floor(Math.random() * out.options.length)
        JSON.parse(game.answer(out.decision, i))
        let stale = false
        try { game.answer(out.decision, i) } catch { stale = true }
        if (!stale) throw new Error('stale answer accepted')
    }
    if (messages > 50000) throw new Error('no end')
}
const replay = JSON.parse(game.replay())
console.log(`replay: ${replay.log.length} decisions`)

// With an exported policy ( `python -m train.export <checkpoint> target/policy.zvp` ) :
// a game against three network bots.
import { existsSync, readFileSync } from 'node:fs'
const policy = new URL('../../../target/policy.zvp', import.meta.url)
if (existsSync(policy)) {
    const networkSpec = {
        ...spec,
        seats: spec.seats.map((s, i) => (i == 0 ? s : { ...s, player: { kind: 'bot', style: 'network' } })),
    }
    let refused = false
    try { new pkg.Game(JSON.stringify(networkSpec)) } catch { refused = true }
    if (!refused) throw new Error('network bots accepted without a policy')
    const net = pkg.Game.withPolicy(JSON.stringify(networkSpec), readFileSync(policy))
    const start = performance.now()
    let botDecisions = 0
    for (let n = 0; ; n++) {
        const out = JSON.parse(net.advance())
        if (out.type == 'over') {
            const ms = (performance.now() - start) / Math.max(1, botDecisions)
            console.log(`network bots: over after ${n} messages:`, out.vps, `(${ms.toFixed(1)} ms per bot decision)`)
            break
        }
        if (out.type == 'decision') net.answer(out.decision, Math.floor(Math.random() * out.options.length))
        else botDecisions++
        if (n > 50000) throw new Error('no end')
    }
} else {
    console.log('no target/policy.zvp: network bots not tested')
}
