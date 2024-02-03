# rustup target install wasm32-unknown-unknown
# cargo install wasm-server-runner

export WASM_SERVER_RUNNER_ADDRESS=0.0.0.0
cargo run --release --target wasm32-unknown-unknown --no-default-features --features "experimental_channel" --bin ethertia