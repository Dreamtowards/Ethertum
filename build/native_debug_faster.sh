# NOTE: Speed Up rust-analyzer(vscode => settings.json)
# "rust-analyzer.cargo.extraEnv": {
#     "RUSTFLAGS": "-Clinker=rust-lld.exe"
# }

cargo run --config=../.cargo/config_build_faster.toml --features "bevy/dynamic_linking" --bin ethertia