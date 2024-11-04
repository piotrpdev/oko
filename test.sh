#!/bin/bash

# https://stackoverflow.com/a/821419/19020549
set -Eeuo pipefail

npm run --prefix web/ build
npm run --prefix web/ test
cargo test
