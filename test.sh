#!/bin/bash -e
ROOT=$(git rev-parse --show-toplevel)
solana program dump PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY $ROOT/target/deploy/phoenix.so -um
#Replace error with info, debug, trace as appropriate to debug.
RUST_LOG=error cargo test-sbf
