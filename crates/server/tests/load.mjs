// A load test (docs/online-plan.md "The server"): `n` games at once against a
// running zvtes-server, each a human answering at random as fast as the
// messages come and three bots (the network profile when served).
// `node crates/server/tests/load.mjs [http://127.0.0.1:8040] [n]`
const base = process.argv[2] ?? 'http://127.0.0.1:8040'
const n = Number(process.argv[3] ?? 16)
const version = await (await fetch(`${base}/version`)).json()
const decks = await (await fetch(`${base}/catalog`)).json()
const bot = version.profiles.includes('network') ? 'network' : 'aggro_bleed'

async function game(i) {
    const spec = {
        time_limit: 40,
        client: 'load.mjs',
        seats: [0, 1, 2, 3].map(s => ({
            name: s == 0 ? 'Me' : `Bot ${s}`,
            deck: decks[(i * 7 + s * 4) % decks.length][1],
            player: s == 0 ? { type: 'human' } : { type: 'bot', profile: bot },
        })),
    }
    const res = await fetch(`${base}/games`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(spec),
    })
    if (res.status != 200) return { refused: res.status }
    const { game, token } = await res.json()
    const ws = new WebSocket(`${base.replace('http', 'ws')}/games/${game}/ws`)
    const inbox = []
    let wake
    ws.onmessage = e => { inbox.push(JSON.parse(e.data)); wake?.() }
    await new Promise(r => (ws.onopen = r))
    const next = async () => { while (!inbox.length) await new Promise(r => (wake = r)); return inbox.shift() }
    ws.send(JSON.stringify({ type: 'join', token }))
    let decisions = 0, messages = 0, sent
    const t0 = performance.now()
    for (;;) {
        const m = await next()
        messages++
        if (m.error == 'rate_limited') {
            await new Promise(r => setTimeout(r, 1000))
            ws.send(sent)
            continue
        }
        if (m.type == 'over') break
        if (m.type == 'decision') {
            decisions++
            const o = m.options[Math.floor(Math.random() * m.options.length)]
            sent = JSON.stringify({ type: 'answer', decision: m.decision, option: o.index })
            ws.send(sent)
        }
    }
    ws.close()
    return { decisions, messages, ms: performance.now() - t0 }
}

const t0 = performance.now()
const results = await Promise.all([...Array(n).keys()].map(game))
const ms = performance.now() - t0
const done = results.filter(r => r.ms)
const refused = results.filter(r => r.refused)
const sum = k => done.reduce((a, r) => a + r[k], 0)
console.log(
    `${n} games at once (${bot} bots): ${done.length} played, ${refused.length} refused, ` +
        `${(ms / 1000).toFixed(1)} s; ${sum('messages')} messages, ${sum('decisions')} human decisions; ` +
        `a game ${(sum('ms') / done.length / 1000).toFixed(1)} s on average`,
)
