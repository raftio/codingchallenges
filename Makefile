.PHONY: build test clean fmt clippy

build:
	cargo build --workspace

test:
	cargo test --workspace

clean:
	cargo clean

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace

build-wc:
	cargo build -p wc

build-json-parser:
	cargo build -p json-parser

build-compression:
	cargo build -p compression
. . e