# `cargo install wasm-bindgen-cli`

wasm-bindgen --no-typescript --out-dir artifacts/wasm --target web target/wasm32-unknown-unknown/web-release/ethertia.wasm --out-name ethertia

# sudo apt install binaryen
# ./wasm-opt.exe -Oz -o ./wasm/ethertia_opt.wasm ./wasm/ethertia_bindgen_bg.wasm
# ./wasm-opt.exe -O -ol 100 -s 100 -o ./wasm/ethertia.wasm ../target/wasm32-unknown-unknown/release/ethertia.wasm

#mkdir artifacts
cp -r assets/ artifacts/wasm/
zip --recurse-paths artifacts/ethertia.zip artifacts/wasm


# Compress Note:
# release default:          39.2 MB
# release opt-level="z":    17.3 MB
# wasm-bindgen              15.7 MB
# wasm-opt -Oz              13.8 MB