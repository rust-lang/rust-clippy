#!/usr/bin/env bash
set -x

if [[ -z "$INTEGRATION" ]]; then
    exit 0
fi

rm ~/.cargo/bin/cargo-clippy
cargo install --force --debug --path .

echo "Running integration test for crate ${INTEGRATION}"

mkdir -p "checkout/$INTEGRATION"
curl -sSL "https://github.com/$INTEGRATION/archive/master.tar.gz" | tar -xzf - -C "checkout/$INTEGRATION"
cd "checkout/$INTEGRATION" || exit 1

# run clippy on a project, try to be verbose and trigger as many warnings as possible for greater coverage
RUST_BACKTRACE=full \
cargo clippy \
    --all-targets \
    --all-features \
    -- --cap-lints warn -W clippy::pedantic -W clippy::nursery \
    2>& 1 \
| tee clippy_output

if grep -q "internal compiler error\|query stack during panic\|E0463" clippy_output; then
    exit 1
fi
