# /// script
# dependencies = ["websockets"]
# ///
"""Plays the first option of every decision against three scripted bots:
`uv run --script scripts/minimal_client.py [https://engine.krcg.org]`."""

import asyncio, json, sys, urllib.request
import websockets

SERVER = sys.argv[1] if len(sys.argv) > 1 else "https://engine.krcg.org"

def get(path):
    return json.load(urllib.request.urlopen(f"{SERVER}{path}"))

async def main():
    decks = dict(get("/catalog"))
    deck = decks["Toreador (Fifth Edition)"]
    seats = [{"name": "Me", "deck": deck, "player": {"type": "human"}}] + [
        {"name": f"Bot {i}", "deck": deck, "player": {"type": "bot", "profile": "wall"}}
        for i in range(1, 4)
    ]
    body = json.dumps({"seats": seats, "client": "my-client 0.1"}).encode()
    request = urllib.request.Request(
        f"{SERVER}/games", body, {"Content-Type": "application/json"}
    )
    game = json.load(urllib.request.urlopen(request))
    socket = SERVER.replace("http", "ws", 1)
    async with websockets.connect(f"{socket}/games/{game['game']}/ws") as ws:
        await ws.send(json.dumps({"type": "join", "token": game["token"]}))
        answer = None
        async for frame in ws:
            message = json.loads(frame)
            if message["type"] == "decision":
                option = message["options"][0]["index"]
                answer = {"type": "answer", "decision": message["decision"], "option": option}
                await ws.send(json.dumps(answer))
            elif message["type"] == "error" and message["error"] == "rate_limited":
                await asyncio.sleep(1)  # not taken: send it again
                await ws.send(json.dumps(answer))
            elif message["type"] == "over":
                print(message["reason"], message["vp_halves"])
                break

asyncio.run(main())
