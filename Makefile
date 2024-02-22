PROFILE_RELEASE=--release
PROFILE_TEST=--profile test-release
FEATURES_CONCURRENT_TESTING=--features concurrent,testing

watch:
	cargo watch -w miden-lib/asm -x build

test:
	cargo test $(PROFILE_TEST) $(FEATURES_CONCURRENT_TESTING) -- --skip prove
	cargo test $(PROFILE_RELEASE) $(FEATURES_CONCURRENT_TESTING) prove

fmt:
	cargo +nightly fix --allow-staged --allow-dirty --all-targets --all-features
	cargo +nightly fmt
	cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings
