from pathlib import Path

ssh_known_hosts_file = str(Path(__file__).parent.parent / "known_hosts")
ssh_strict_host_key_checking = "yes"
_sudo = True
# the server-setup fleet's deploy key
ssh_user = "deploy"
ssh_key = "~/.ssh/deploy"

# the OS user, its group, the database, the log tag, the root of every path
name = "zvtes"
domain = "engine.krcg.org"
port = 8040
# the page origins that may create games and open sockets: empty, any
# (the owner, 2026-10-16: Thomas's own builds of Succubus Club, on any
# port or preview host, play against this one; the seat's token is the
# gate, no cookie or credential rides a request)
origins = ""
# the forum shares the box: the engine's bots never take it whole
cpu_quota = "400%"
memory_max = "3G"
# The admin token's SHA-256 (the token itself stays with the owner:
# ~/.config/zvtes/admin_token; scripts/fetch_games.py pulls the records)
admin_sha256 = "c8bfd18f6e4189c2280d39a5760fd948d000ccdaeb83d47dc2a43bcaadb64c74"
