PROFILE_TEST=--profile test-release
FEATURES_CONCURRENT_TESTING=--features="concurrent, testing"

watch:
	cargo watch -w miden-lib/asm -x build

test:
	cargo test $(PROFILE_TEST) $(FEATURES_CONCURRENT_TESTING)
