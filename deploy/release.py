"""The server binary to deploy: a GitHub release's asset (built by
.github/workflows/release.yml), or a local build (BUILD_DIR, e.g. from
deploy/build.sh) while the repository has no release."""

import json
import os
import urllib.request
from pathlib import Path

REPO = os.environ.get("ZVTES_REPO", "")
ASSET = "zvtes-server-x86_64-linux"
BUILD = Path(__file__).parent / "build"


def binary(tag: str = "") -> Path:
    if local := os.environ.get("BUILD_DIR"):
        return Path(local) / ASSET
    if not REPO:
        raise RuntimeError("set BUILD_DIR (deploy/build.sh) or ZVTES_REPO (owner/name on GitHub)")
    headers = {"Accept": "application/vnd.github+json"}
    if token := os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = f"Bearer {token}"
    api = f"https://api.github.com/repos/{REPO}/releases/" + (f"tags/{tag}" if tag else "latest")
    release = json.load(urllib.request.urlopen(urllib.request.Request(api, headers=headers), timeout=30))
    url = next(a["browser_download_url"] for a in release["assets"] if a["name"] == ASSET)
    out = BUILD / release["tag_name"] / ASSET
    if not out.exists():
        out.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(url, out.with_suffix(".part"))
        out.with_suffix(".part").rename(out)
    print(f"Deploying {release['tag_name']} from {REPO}")
    return out
