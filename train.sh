#!/bin/bash

# build appliaction
cargo build --release --bin=train;

~/.target/release/train

# python3 plot.py

# eog plot.png
