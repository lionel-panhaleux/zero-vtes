#!/bin/sh
# The contract's messages and a record typed against crates/api/protocol.ts
# (docs/online-plan.md "Contract tests"): a game written as a TypeScript
# fixture by the schema test, checked with tsc --strict. TSC: a TypeScript
# compiler (the Succubus Club checkout's by default).
set -e
TSC=${TSC:-../succubus-club/node_modules/.bin/tsc}
DIR=$(mktemp -d)
CONTRACT_TS=$DIR cargo test --profile ci -q -p zvtes-host --test contract every_message >/dev/null
"$TSC" --noEmit --strict --target es2020 --moduleResolution node "$DIR/fixtures.ts"
echo "protocol.ts: the fixtures type-check"
rm -rf "$DIR"
