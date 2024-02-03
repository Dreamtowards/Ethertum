# `cargo install wasm-bindgen-cli`

wasm-bindgen --no-typescript --out-dir wasm --target web target/wasm32-unknown-unknown/release/ethertia.wasm --out-name bevy_game
cp -r assets wasm/
zip --recurse-paths ethertia.zip ./wasm