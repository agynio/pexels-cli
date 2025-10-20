.PHONY: fmt lint test build run

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets --all-features -D warnings

test:
	cargo test --all-features --workspace

build:
	cargo build --workspace

run:
	cargo run --package pexels --
