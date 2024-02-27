# install Android SDK
# rustup target add aarch64-linux-android
# cargo install cargo-apk

cargo +stable apk build --profile android-debug --package mobile