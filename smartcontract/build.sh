set -e
export RUSTFLAGS='-C link-arg=-s'
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/smartcontract.wasm ./res/