#!/usr/bin/env bash

# https://stackoverflow.com/a/821419/19020549
set -Eeuo pipefail

# https://stackoverflow.com/a/3355423/19020549
cd "$(dirname "$0")"

cd ../frontend
npm run build
npm run test

cd ../backend
cargo test
