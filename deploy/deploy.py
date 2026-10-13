"""The engine served on frankfurt (docs/online-plan.md B7), the
server-setup way: a system user, the binary from a release (release.py),
the policy weights archived by hash, Postgres by peer auth (no secret),
a systemd unit, nginx + TLS for engine.krcg.org.

    cd deploy && uv run --group deploy pyinfra inventory.py deploy.py --diff

Environment: ZVTES_WEIGHTS (the .zvp to serve; default ../target/policy.zvp),
BUILD_DIR or ZVTES_REPO (+ RELEASE_TAG) for the binary.
"""

import hashlib
import os
from io import StringIO
from pathlib import Path

import jinja2
from pyinfra import host
from pyinfra.operations import files, server, systemd
from pyinfra.operations.util import any_changed
from release import binary
from server_setup import nginx_site, postgres_db

HERE = Path(__file__).resolve().parent
d = host.data
name = d.name
opt = f"/opt/{name}"
env_dir = f"/etc/{name}"
lib_dir = f"/var/lib/{name}"
values = dict(
    name=name,
    opt=opt,
    env_dir=env_dir,
    lib_dir=lib_dir,
    cpu_quota=d.cpu_quota,
    memory_max=d.memory_max,
)


def render(template: str) -> StringIO:
    env = jinja2.Environment(
        loader=jinja2.FileSystemLoader(HERE / "templates"),
        keep_trailing_newline=True,
        undefined=jinja2.StrictUndefined,
    )
    return StringIO(env.get_template(template).render(**values))


server.group(name="App group", group=name, system=True)
server.user(
    name="App user",
    user=name,
    group=name,
    home=opt,
    shell="/usr/sbin/nologin",
    system=True,
)
files.directory(name="App root", path=opt, user="root", group=name, mode="755")
files.directory(name="Weights", path=f"{opt}/weights", user="root", group=name, mode="755")
files.directory(name="Env dir", path=env_dir, user="root", group=name, mode="750")
files.directory(name="App lib dir", path=lib_dir, user=name, group=name, mode="750")

server_binary = files.put(
    name="Server binary",
    src=str(binary(os.environ.get("RELEASE_TAG", ""))),
    dest=f"{opt}/zvtes-server",
    user="root",
    group=name,
    mode="755",
)

# Every weights file served is kept by its hash: a record names the hash,
# and replays on it.
weights = Path(os.environ.get("ZVTES_WEIGHTS", HERE.parent / "target/policy.zvp"))
digest = hashlib.sha256(weights.read_bytes()).hexdigest()
weights_file = files.put(
    name="Policy weights",
    src=str(weights),
    dest=f"{opt}/weights/{digest}.zvp",
    user="root",
    group=name,
    mode="644",
)

postgres_db(database=name, owner=name)

env = files.put(
    name="Environment",
    src=StringIO(
        "".join(
            f"{k}={v}\n"
            for k, v in {
                "ZVTES_LISTEN": f"127.0.0.1:{d.port}",
                # peer authentication over the unix socket, as the OS user
                "ZVTES_DATABASE_URL": f"postgres:///{name}?host=/var/run/postgresql",
                "ZVTES_POLICY": f"{opt}/weights/{digest}.zvp",
                "ZVTES_ORIGINS": d.origins,
            }.items()
        )
    ),
    dest=f"{env_dir}/{name}.env",
    user="root",
    group=name,
    mode="640",
)

unit = files.put(
    name="Unit",
    src=render("zvtes.service.j2"),
    dest=f"/etc/systemd/system/{name}.service",
    mode="644",
)
systemd.daemon_reload(name="Reload units", _if=unit.did_change)
systemd.service(name=f"{name} running", service=f"{name}.service", running=True, enabled=True)
systemd.service(
    name=f"Restart {name}",
    service=f"{name}.service",
    restarted=True,
    _if=any_changed(server_binary, weights_file, env, unit),
)

nginx_site(
    site=name,
    domain=d.domain,
    type="proxy",
    upstream=f"http://127.0.0.1:{d.port}",
    client_max_body_size="256k",
)
