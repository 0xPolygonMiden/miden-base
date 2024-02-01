PROFILE_RELEASE=--release
PROFILE_TEST=--profile test-release
FEATURES_CONCURRENT_TESTING=--features concurrent,testing
FEATURES_CONCURRENT_TESTING_PROVING=--features concurrent,testing,has_proving
HAS_PROVING=has_proving

watch:
	cargo watch -w miden-lib/asm -x build

test:
	cargo test $(PROFILE_TEST) $(FEATURES_CONCURRENT_TESTING)
	cargo test $(PROFILE_RELEASE) $(FEATURES_CONCURRENT_TESTING_PROVING) $(HA)