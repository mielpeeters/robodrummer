#!/bin/bash

# build appliaction
cargo build --release --bin=run;

./target/release/run

python3 plot.py

# eog plot.png
