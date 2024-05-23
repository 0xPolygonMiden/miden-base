.DEFAULT_GOAL := help

.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# -- variables --------------------------------------------------------------------------------------

WARNINGS=RUSTDOCFLAGS="-D warnings"
DEBUG_ASSERTIONS=RUSTFLAGS="-C debug-assertions"

# -- linting --------------------------------------------------------------------------------------


.PHONY: clippy
clippy: ## Runs Clippy with configs
	cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings


.PHONY: fix
fix: ## Runs Fix with configs
	cargo +nightly fix --allow-staged --allow-dirty --all-targets --all-features


.PHONY: format
format: ## Runs Format using nightly toolchain
	cargo +nightly fmt --all


.PHONY: format-check
format-check: ## Runs Format using nightly toolchain but only in check mode
	cargo +nightly fmt --all --check


.PHONY: lint
lint: format fix clippy ## Runs all linting tasks at once (Clippy, fixing, formatting)

# --- docs ----------------------------------------------------------------------------------------


.PHONY: doc
doc: ## Generates & checks documentation 
	$(WARNINGS) cargo doc --all-features --keep-going --release


.PHONY: doc-serve
doc-serve: ## Serves documentation site
	./scripts/serve-doc-site.sh

# --- testing -------------------------------------------------------------------------------------


.PHONY: test-default
test-default: ## Run default tests excluding `prove`
	$(DEBUG_ASSERTIONS) cargo nextest run --profile default --cargo-profile test-release --features concurrent,testing --filter-expr "not test(prove)"


.PHONY: test-prove
test-prove: ## Run `prove` tests (tests which use the Miden prover)
	$(DEBUG_ASSERTIONS) cargo nextest run --profile prove --cargo-profile test-release --features concurrent,testing --filter-expr "test(prove)"


.PHONY: test
test: ## Run all tests
	$(DEBUG_ASSERTIONS) $(MAKE) -j2 test-default test-prove

# --- building ------------------------------------------------------------------------------------


.PHONY: build
build: ## By default we should build in release mode
	cargo build --release


.PHONY: build-no-std
build-no-std: ## Build without the standard library
	cargo build --verbose --no-default-features --target wasm32-unknown-unknown --workspace --exclude miden-mock --exclude miden-bench-tx

# --- benchmarking --------------------------------------------------------------------------------


.PHONY: bench-tx
bench-tx: ## Run transaction benchmarks
	cargo run --bin bench-tx