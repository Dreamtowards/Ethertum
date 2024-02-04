# NOTE: Disabling bevy/dynamic_linking may improve the performance of this linker(mold linker).
cargo run --config=../.cargo/config_build_fastest.toml --features "bevy/dynamic_linking" --bin ethertia