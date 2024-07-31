#!/bin/sh

set -e

cargo build --release
./target/release/nash.exe ./examples/example-1.nash
./target/release/nash.exe ./examples/example-2.nash
./target/release/nash.exe ./examples/example-3.nash
