# rustup target install wasm32-unknown-unknown
# cargo install wasm-server-runner

export WASM_SERVER_RUNNER_ADDRESS=0.0.0.0

# set CARGO_PROFILE_RELEASE_LTO true
# set CARGO_PROFILE_RELEASE_OPT_LEVEL z

cargo run --profile web-release --target wasm32-unknown-unknown --no-default-features --features "experimental_channel"