watch:
	cargo watch -w miden-lib/asm -x build

test:
	cargo test --features testing
