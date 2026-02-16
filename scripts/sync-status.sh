#!/bin/bash
# Sync crate status to website data
# Reads crate-status.toml files and generates TypeScript output
cd "$(dirname "$0")/.."
node --experimental-strip-types scripts/sync-status.ts "$@"
