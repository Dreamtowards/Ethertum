.PHONY: clean run install boot-sim

DEVICE = ${DEVICE_ID}

clean:
	cargo clean
	rm -fr output

run: install
	xcrun simctl launch --console $(DEVICE) com.ethertia.client

boot-sim:
	xcrun simctl boot $(DEVICE) || true

install: boot-sim iphone-sim-debug
	xcrun simctl install $(DEVICE) output/Build/Products/Debug-iphonesimulator/mobile.app

iphone-sim-debug:
	IOS_TARGETS=x86_64-apple-ios xcodebuild -scheme mobile -configuration Debug -derivedDataPath output -destination "id=$(DEVICE)"

iphone-release:
	cargo build --release --target aarch64-apple-ios --package mobile
	
	IOS_TARGETS=aarch64-apple-ios \
	xcodebuild clean build -scheme mobile -configuration Release -derivedDataPath output -arch arm64 \
	CODE_SIGN_IDENTITY="" \
	CODE_SIGNING_REQUIRED=NO \
	CODE_SIGN_ENTITLEMENTS="" \
	CODE_SIGNING_ALLOWED="NO"