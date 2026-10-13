#!/bin/sh
# A linux x86_64 server binary built in Docker (Debian bookworm: its glibc
# runs on frankfurt's trixie), into deploy/build/local/. Then:
# BUILD_DIR=deploy/build/local uv run --group deploy pyinfra deploy/inventory.py deploy/deploy.py
set -eu
cd "$(dirname "$0")/.."
mkdir -p deploy/build/local
engine=$(git rev-parse --short=12 HEAD)
[ -z "$(git status --porcelain --untracked-files=no)" ] || engine="$engine+dirty"
docker run --rm --platform linux/amd64 -v "$PWD":/src -w /src \
    -v zvtes-cargo:/usr/local/cargo/registry -e CARGO_TARGET_DIR=/src/target/linux -e ZVTES_ENGINE="$engine" \
    rust:1-bookworm cargo build --release -p zvtes-server
cp target/linux/release/zvtes-server deploy/build/local/zvtes-server-x86_64-linux
