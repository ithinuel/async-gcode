#!/usr/bin/env bash
set -e

cargo clippy
cargo test --color=always
echo "Parse comments"
cargo clippy --features parse-comments
cargo test --color=always --features parse-comments
echo "Parse trailing comment"
cargo clippy --features parse-trailing-comment
cargo test --color=always --features parse-trailing-comment
echo "Combine comments & trailing comment"
cargo clippy --features parse-trailing-comment,parse-comments
cargo test --color=always --features parse-trailing-comment,parse-comments

echo "Parse checksum"
cargo clippy --features parse-checksum
cargo test --color=always --features parse-checksum
echo "Combine checksum & trailing comment"
cargo clippy --features parse-trailing-comment,parse-checksum
cargo test --color=always --features parse-trailing-comment,parse-checksum
echo "Combine checksum & comments & trailing comment"
cargo clippy --features parse-trailing-comment,parse-comments,parse-checksum
cargo test --color=always --features parse-trailing-comment,parse-comments,parse-checksum

echo "Parse optional-value"
cargo clippy --features optional-value
cargo test --color=always --features optional-value
echo "Parse string-value"
cargo clippy --features string-value
cargo test --color=always --features string-value
echo "Parse parameters"
cargo clippy --features parse-parameters
cargo test --color=always --features parse-parameters
echo "Parse parameters & optional-value"
cargo clippy --features parse-parameters,optional-value
cargo test --color=always --features parse-parameters,optional-value
echo "Parse parameters & string-value"
cargo clippy --features parse-parameters,string-value
cargo test --color=always --features parse-parameters,string-value
echo "Parse expressions"
cargo clippy --features parse-expressions
cargo test --color=always --features parse-expressions

echo "Combine expressions & parameters"
cargo clippy --features parse-expressions,parse-parameters
cargo test --color=always --features parse-expressions,parse-parameters

echo "Combine all features"
cargo clippy --all-features
cargo test --color=always --all-features

echo "build for thumbv7em-none-eabihf"
# On stable dev-dependencies' features leak on build dependencies.
# `-Z features=dev_dep` is required until it lands on stable
cargo build --color=always --release --no-default-features \
            --features parse-parameters,parse-trailing-comment,optional-value,parse-checksum \
            --target thumbv7em-none-eabihf \
            -Z features=dev_dep

