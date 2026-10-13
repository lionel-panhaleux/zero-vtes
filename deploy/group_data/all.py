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
# the platforms whose pages may create games and open sockets
origins = "https://succubus.club,https://www.succubus.club"
# the forum shares the box: the engine's bots never take it whole
cpu_quota = "400%"
memory_max = "3G"
