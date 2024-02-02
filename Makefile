PROFILE_RELEASE=--release
PROFILE_TEST=--profile test-release
FEATURES_CONCURRENT_TESTING=--features concurrent,testing
PROVE=prove

watch:
	cargo watch -w miden-lib/asm -x build

test:
	cargo test $(PROFILE_TEST) $(FEATURES_CONCURRENT_TESTING) -- --skip $(PROVE)
	cargo test $(PROFILE_RELEASE) $(FEATURES_CONCURRENT_TESTING) $(PROVE)