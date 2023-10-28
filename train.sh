#!/bin/bash

# build application
cargo build --release --bin=train;

~/.target/release/train
