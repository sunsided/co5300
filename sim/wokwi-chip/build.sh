#!/usr/bin/env sh
# Builds the chip and places it next to co5300.chip.json, as Wokwi expects.
set -eu
cd "$(dirname "$0")"
cargo build --release
cp target/wasm32-unknown-unknown/release/co5300_wokwi_chip.wasm co5300.chip.wasm
