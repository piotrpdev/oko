#!/bin/bash

# https://stackoverflow.com/a/821419/19020549
set -Eeuo pipefail

cd ./web
npm run build
npm run test

cd ../oko-rs
cargo test
