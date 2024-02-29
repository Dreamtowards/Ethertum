# install Android SDK
# rustup target add aarch64-linux-android
# cargo install cargo-apk

RANLIB=$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib cargo +stable apk build --profile android-debug --package mobile