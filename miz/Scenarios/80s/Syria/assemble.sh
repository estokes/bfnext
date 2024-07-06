#! /bin/bash

export RUST_LOG="info"
cd $(dirname ${BASH_SOURCE[0]})
cargo run --manifest-path=../../../../bftools/Cargo.toml miz --output ./Syria80s.miz --base ./base.miz --weapon ../weapon.miz --warehouse ../warehouse.miz --options ../options.miz
