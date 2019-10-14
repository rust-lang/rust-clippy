#!/bin/bash
# Set up the appropriate rustc toolchain

cd "$(dirname "$0")" || exit

if ! command -v rustup-toolchain-install-master > /dev/null; then
  cargo install \
    --git https://github.com/lzutao/rustup-toolchain-install-master \
    --branch ci-more-oses \
    --bin rustup-toolchain-install-master \
    --debug
fi

rustup-toolchain-install-master -f -n master
rustup override set master
