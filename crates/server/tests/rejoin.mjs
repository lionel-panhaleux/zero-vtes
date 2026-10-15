// A seat rejoining a game against a running zvtes-server (docs/protocol.md
// §2, `join` with `after`): a socket dropped and joined again with the last
// `seq` resumes in order at the pending decision; an older `seq` gets the
// messages after it again, in order; a join without `after` (a page
// refresh with the token kept) starts afresh from `joined` and the pending
// decision. Built-in bots: no policy needed.
// `node crates/server/tests/rejoin.mjs [http://127.0.0.1:8040]`
const base = process.argv[2] ?? 'http://127.0.0.1:8040'
const decks = await (await fetch(`${base}/catalog`)).json()
const spec = {
    time_limit: 40,
    client: 'rejoin.mjs',
    seats: [0, 1, 2, 3].map(i => ({
        name: i == 0 ? 'Me' : `Bot ${i}`,
        deck: decks[(i * 4) % decks.length][1],
        player: i == 0 ? { type: 'human' } : { type: 'bot', profile: 'aggro_bleed' },
    })),
}
const res = await fetch(`${base}/games`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(spec) })
const { game, token } = await res.json()
const check = (ok, what) => { if (!ok) throw new Error(what) }
const pause = ms => new Promise(r => setTimeout(r, ms))

// A connection joined with `join`'s extra fields; `next` waits for a message, null after `ms`
async function connect(join) {
    const ws = new WebSocket(`${base.replace('http', 'ws')}/games/${game}/ws`)
    const inbox = []
    let wake
    ws.onmessage = e => { inbox.push(JSON.parse(e.data)); wake?.() }
    await new Promise(r => (ws.onopen = r))
    ws.send(JSON.stringify({ type: 'join', token, ...join }))
    const next = async (ms = 3000) => {
        const end = Date.now() + ms
        while (!inbox.length) {
            if (Date.now() > end) return null
            await new Promise(r => { wake = r; setTimeout(r, 100) })
        }
        return inbox.shift()
    }
    return { ws, next }
}

// Answer `n` decisions at random from `seq` on; returns the last seq and the decision then pending
async function play(c, n, seq) {
    for (let answered = 0; ; ) {
        const m = await c.next()
        check(m, `nothing after seq ${seq}`)
        check(m.seq == seq + 1, `seq ${m.seq} after ${seq}`)
        seq = m.seq
        check(m.type != 'over' && m.type != 'error', `${m.type} before ${n} decisions`)
        if (m.type == 'decision') {
            if (answered == n) return { seq, pending: m }
            answered++
            const o = m.options[Math.floor(Math.random() * m.options.length)]
            c.ws.send(JSON.stringify({ type: 'answer', decision: m.decision, option: o.index }))
        }
    }
}

let c = await connect({})
let at = await play(c, 10, 0)
c.ws.close()
await pause(300)

// Joined again after the last seq: nothing is resent, the pending decision is still open
c = await connect({ after: at.seq })
check((await c.next(1000)) == null, 'a message resent though none was missed')
c.ws.send(JSON.stringify({ type: 'answer', decision: at.pending.decision, option: at.pending.options[0].index }))
at = await play(c, 5, at.seq)
console.log(`rejoined after seq: resumed in order, at seq ${at.seq}`)
c.ws.close()
await pause(300)

// Joined again after an older seq: what followed it, again, in order
c = await connect({ after: at.seq - 4 })
for (let s = at.seq - 3; s <= at.seq; s++) {
    const m = await c.next()
    check(m?.seq == s, `resent seq ${m?.seq}, expected ${s}`)
}
console.log('rejoined after an older seq: the 4 messages after it resent in order')
c.ws.close()
await pause(300)

// Joined again without `after`: afresh, `joined` then the pending decision
c = await connect({})
const joined = await c.next()
const decision = await c.next()
check(joined?.type == 'joined' && joined.view, `${joined?.type} first on a fresh join`)
check(decision?.type == 'decision' && decision.decision == at.pending.decision, 'not the pending decision')
console.log(`rejoined without after: joined, then decision ${decision.decision}`)
c.ws.close()
