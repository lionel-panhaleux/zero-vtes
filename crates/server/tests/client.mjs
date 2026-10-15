// A full game against a running zvtes-server, as a client would play it
// (docs/protocol.md §2): POST /games, the WebSocket, `join` with the
// token, answers by option index, a `feedback` mid-game, the record at
// the end; with ZVTES_ADMIN_TOKEN, the feedback read from /admin/feedback.
// `node crates/server/tests/client.mjs [http://127.0.0.1:8040]`
const base = process.argv[2] ?? 'http://127.0.0.1:8040'
const version = await (await fetch(`${base}/version`)).json()
console.log(`server: engine ${version.engine}, protocol ${version.protocol}, profiles ${version.profiles}`)
const decks = await (await fetch(`${base}/catalog`)).json()
const bot = version.profiles.includes('network') ? 'network' : 'aggro_bleed'
const spec = {
    time_limit: 40,
    client: 'client.mjs',
    seats: [0, 1, 2, 3].map(i => ({
        name: i == 0 ? 'Me' : `Bot ${i}`,
        deck: decks[(i * 4) % decks.length][1],
        player: i == 0 ? { type: 'human', settings: { text: true } } : { type: 'bot', profile: bot },
    })),
}
const two = await fetch(`${base}/games`, { method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ ...spec, seats: spec.seats.map(s => ({ ...s, player: { type: 'human' } })) }) })
if (two.status != 400) throw new Error('four humans accepted')
const res = await fetch(`${base}/games`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(spec) })
const { game, seat, token } = await res.json()
console.log(`game ${game}, seat ${seat}`)
const ws = new WebSocket(`${base.replace('http', 'ws')}/games/${game}/ws`)
const inbox = []
let wake
ws.onmessage = e => { inbox.push(JSON.parse(e.data)); wake?.() }
await new Promise(r => (ws.onopen = r))
const next = async () => { while (!inbox.length) await new Promise(r => (wake = r)); return inbox.shift() }
ws.send(JSON.stringify({ type: 'join', token: 'wrong' }))
if ((await next()).error != 'not_joined') throw new Error('a wrong token joined')
ws.send(JSON.stringify({ type: 'join', token }))
let seq = 0, decisions = 0, last, sent, noted
for (;;) {
    const m = await next()
    if (m.error == 'rate_limited') {
        // Unnumbered: wait, then send the answer again.
        await new Promise(r => setTimeout(r, 1000))
        ws.send(sent)
        continue
    }
    if (m.seq != seq + 1) throw new Error(`seq ${m.seq} after ${seq}`)
    seq = m.seq
    if (m.type == 'error') throw new Error(JSON.stringify(m))
    if (m.type == 'over') { last = m; break }
    if (m.type == 'noted') { noted = m.decision; continue }
    if (m.type == 'decision') {
        decisions++
        // A report mid-game (§4 `feedback`): noted at this decision.
        if (decisions == 3) ws.send(JSON.stringify({ type: 'feedback', text: `client.mjs report at ${m.decision}` }))
        const o = m.options[Math.floor(Math.random() * m.options.length)]
        if (!o.text) throw new Error('text asked, none given')
        sent = JSON.stringify({ type: 'answer', decision: m.decision, option: o.index })
        ws.send(sent)
    }
}
console.log(`over: ${last.reason.type} after ${decisions} decisions, ${seq} messages, vp_halves ${last.vp_halves}`)
const rec = await (await fetch(`${base}/games/${game}/record`, { headers: { authorization: `Bearer ${token}` } })).json()
if (!rec.result || rec.spec.seats[seat].name != 'human') throw new Error('bad record')
console.log(`record: ${rec.entries.length} entries, bots ${rec.bots.map(b => b.profile)}`)
const fed = rec.entries.find(e => e.type == 'feedback')
if (noted === undefined || !fed || fed.decision != noted) throw new Error('feedback not recorded at its decision')
// With the operator's token, the report is listed for the admin.
if (process.env.ZVTES_ADMIN_TOKEN) {
    const list = await (await fetch(`${base}/admin/feedback`, { headers: { authorization: `Bearer ${process.env.ZVTES_ADMIN_TOKEN}` } })).json()
    if (!list.some(f => f.game == game && f.decision == noted)) throw new Error('feedback not listed at /admin/feedback')
    console.log(`feedback: noted at decision ${noted}, listed for the admin`)
}
ws.close()

// RECORD_AFTER=s: once the game's thread is gone (an idle timeout under
// s), the record comes from the store.
if (process.env.RECORD_AFTER) {
    await new Promise(r => setTimeout(r, 1000 * Number(process.env.RECORD_AFTER)))
    const later = await fetch(`${base}/games/${game}/record`, { headers: { authorization: `Bearer ${token}` } })
    const stored = await later.json()
    if (stored.entries.length != rec.entries.length || !stored.result) throw new Error('stored record differs')
    const wrong = await fetch(`${base}/games/${game}/record`, { headers: { authorization: 'Bearer nope' } })
    if (wrong.status == 200) throw new Error('a wrong token read the record')
    console.log(`stored record: ${stored.entries.length} entries, read back after the game ended`)
}
