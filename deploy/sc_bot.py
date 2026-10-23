"""The Succubus Club fork's bots-only edition served on frankfurt as a
static site (owner, 2026-10-21: sc-bot.krcg.org, built on the box): the
fork's branch at a pinned commit, built by its own user with Debian's
Node, the build synced into nginx's root, an SPA site with TLS.

    cd deploy && SC_COMMIT=<sha> uv run --group deploy pyinfra inventory.py sc_bot.py --diff

The domain must resolve to the box first: the certificate is HTTP-01.
"""

import os

from pyinfra.operations import apt, files, server
from server_setup import nginx_site

name = "sc_bot"
domain = "sc-bot.krcg.org"
repo = "https://github.com/lionel-panhaleux/succubus-club.git"
branch = "zero-vtes-engine"
commit = os.environ["SC_COMMIT"]
home = f"/opt/{name}"
src = f"{home}/src"
root = f"/var/www/{name}"

# Debian 13's Node (20.19) is what Vite 7 asks for.
# No package-list refresh: server-setup keeps the lists, and a
# third-party repository mid-sync (Grafana's) fails an update.
apt.packages(name="Node and git", packages=["nodejs", "npm", "git", "rsync"])

server.group(name="Build group", group=name, system=True)
server.user(
    name="Build user",
    user=name,
    group=name,
    home=home,
    shell="/usr/sbin/nologin",
    system=True,
)
files.directory(name="Build home", path=home, user=name, group=name, mode="755")

# The commit pinned, detached: a branch pulled would build whatever it
# moved to.
server.shell(
    name="Fork at the pinned commit",
    commands=[
        f"test -d {src}/.git || git clone -q --branch {branch} {repo} {src}",
        f"git -C {src} fetch -q origin {branch}",
        f"git -C {src} checkout -q --detach {commit}",
    ],
    _sudo_user=name,
)

# The forum and the engine share the box: the build runs niced, and only
# when the commit differs from the one built last. The fork's lock is
# npm 11's (Debian's npm 9 refuses it): that npm, pinned, for the build.
npm = "npx --yes npm@11.21.0"
server.shell(
    name="Build",
    commands=[
        f"cd {src} && test \"$(cat {home}/built 2>/dev/null)\" = {commit}"
        f" || (HOME={home} nice -n 10 {npm} ci --no-audit --no-fund"
        f" && HOME={home} nice -n 10 {npm} run build && echo {commit} > {home}/built)"
    ],
    _sudo_user=name,
)

files.directory(name="Site root", path=root, user="root", group="www-data", mode="755")
server.shell(
    name="Site files",
    commands=[f"rsync -a --delete {src}/dist/ {root}/"],
)

nginx_site(site=name, domain=domain, type="spa", root=root)
