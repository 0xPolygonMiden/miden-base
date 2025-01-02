.DEFAULT_GOAL := help

.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

# -- variables --------------------------------------------------------------------------------------

WARNINGS=RUSTDOCFLAGS="-D warnings"
DEBUG_ASSERTIONS=RUSTFLAGS="-C debug-assertions"
ALL_FEATURES_BUT_ASYNC=--features concurrent,testing
BUILD_KERNEL_ERRORS=BUILD_KERNEL_ERRORS=1
# Enable backtraces for tests where we return an anyhow::Result. If enabled, anyhow::Error will
# then contain a `Backtrace` and print it when a test returns an error.
BACKTRACE=RUST_BACKTRACE=1

# -- linting --------------------------------------------------------------------------------------

.PHONY: clippy
clippy: ## Runs Clippy with configs
	cargo clippy --workspace --all-targets $(ALL_FEATURES_BUT_ASYNC) -- -D warnings


.PHONY: fix
fix: ## Runs Fix with configs
	cargo fix --workspace --allow-staged --allow-dirty --all-targets $(ALL_FEATURES_BUT_ASYNC)


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
	$(WARNINGS) cargo doc $(ALL_FEATURES_BUT_ASYNC) --keep-going --release


.PHONY: doc-serve
doc-serve: ## Serves documentation site
	./scripts/serve-doc-site.sh

# --- testing -------------------------------------------------------------------------------------

.PHONY: test-build
test-build: ## Build the test binary
	$(DEBUG_ASSERTIONS) cargo nextest run --cargo-profile test-release --features concurrent,testing --no-run


.PHONY: test-default
test-default: ## Run default tests excluding `prove`
	$(DEBUG_ASSERTIONS) $(BACKTRACE) cargo nextest run --profile default --cargo-profile test-release --features concurrent,testing --filter-expr "not test(prove)"


.PHONY: test-docs
test-docs: ## Run documentation tests
	$(WARNINGS) $(DEBUG_ASSERTIONS) cargo test --doc $(ALL_FEATURES_BUT_ASYNC)


.PHONY: test-prove
test-prove: ## Run `prove` tests (tests which use the Miden prover)
	$(DEBUG_ASSERTIONS) $(BACKTRACE) cargo nextest run --profile prove --cargo-profile test-release --features concurrent,testing --filter-expr "test(prove)"


.PHONY: test
test: test-default test-prove ## Run all tests

# --- checking ------------------------------------------------------------------------------------

.PHONY: check
check: ## Check all targets and features for errors without code generation
	${BUILD_KERNEL_ERRORS} cargo check --all-targets $(ALL_FEATURES_BUT_ASYNC)

# --- building ------------------------------------------------------------------------------------

.PHONY: build
build: ## By default we should build in release mode
	${BUILD_KERNEL_ERRORS} cargo build --release


.PHONY: build-no-std
build-no-std: ## Build without the standard library
	${BUILD_KERNEL_ERRORS} cargo build --no-default-features --target wasm32-unknown-unknown --workspace --lib --exclude miden-proving-service --features tx-prover


.PHONY: build-no-std-testing
build-no-std-testing: ## Build without the standard library. Includes the `testing` feature
	cargo build --no-default-features --target wasm32-unknown-unknown --workspace --exclude miden-bench-tx --exclude miden-proving-service --features testing,tx-prover


.PHONY: build-async
build-async: ## Build with the `async` feature enabled (only libraries)
	${BUILD_KERNEL_ERRORS} cargo build --lib --release --features async

.PHONY: build-remote-provers
build-remote-provers: ## Build the remote provers
	${BUILD_KERNEL_ERRORS} cargo build --release --lib --features tx-prover -p miden-remote-provers

# --- benchmarking --------------------------------------------------------------------------------

.PHONY: bench-tx
bench-tx: ## Run transaction benchmarks
	cargo run --bin bench-tx


# --- installing ----------------------------------------------------------------------------------

.PHONY: install-proving-service
install-proving-service: ## Install proving service's CLI
	cargo install --path bin/proving-service --bin miden-proving-service --locked --features concurrent

.PHONY: install-proving-service-testing
install-proving-service-testing: ## Install proving service's CLI intended for testing purposes
	cargo install --path bin/proving-service --bin miden-proving-service --locked --features concurrent,testing
