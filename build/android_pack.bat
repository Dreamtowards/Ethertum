# 1. rustup target add aarch64-linux-android
# 2. cargo install cargo-apk
# 3. Install Android SDK (https://developer.android.com/tools/sdkmanager) 
#    $ANDROID_HOME, $ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/{version}/

cargo +stable apk build --profile android-debug --package mobile

pause