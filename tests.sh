#!/usr/bin/env bash
cargo test --color=always --no-default-features && \
cargo test --color=always --no-default-features --features extended && \
cargo test --color=always --no-default-features --features optional-value && \
cargo test --color=always --no-default-features --features parse-comments && \
cargo test --color=always --no-default-features --features parse-comments,extended && \
cargo test --color=always --no-default-features --features parse-parameters && \
cargo test --color=always --no-default-features --features parse-expressions && \
cargo test --color=always --no-default-features --features parse-expressions,parse-parameters && \
cargo test --color=always --features extended,optional-value && \
cargo build --release --features no_std,extended --target thumbv7m-none-eabi

