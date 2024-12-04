#!/bin/sh

export HTTP_PORT=4060
export EXTERNAL_BASE=localhost

RUST_BACKTRACE=1 RUST_LOG=debug RUST_LIB_BACKTRACE=1 cargo run

