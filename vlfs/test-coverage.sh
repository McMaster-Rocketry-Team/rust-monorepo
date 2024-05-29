#! /bin/sh

# Run at the root of the project
CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' RUST_BACKTRACE=1 RUST_BACKTRACE=1 cargo test --features "std log ecc internal_test_coverage" --no-default-features --package vlfs --lib
rm -rf target/coverage
mkdir -p target/coverage
grcov . --binary-path ./target/debug/deps/ -s . -t lcov --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/tests.lcov
grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html
rm -f cargo-test-*.profraw
rm -f vlfs/cargo-test-*.profraw

echo "lcov files at at target/coverage/tests.lcov"
echo "html files at at target/coverage/html/index.html"