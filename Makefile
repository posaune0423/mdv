SHELL := /bin/sh
UNAME_S := $(shell uname -s)

.PHONY: build build-tracked-bin fmt fmt-check lint test test-unit test-integration test-e2e check ci hooks-install install-local run-local

build:
	cargo build --release
	mkdir -p bin
ifneq ($(UNAME_S),Darwin)
	cp target/release/mdv bin/mdv
else
	codesign --force --sign - target/release/mdv
	cp target/release/mdv bin/mdv
	codesign --force --sign - bin/mdv
endif

build-tracked-bin: build

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
	cargo test --workspace --all-targets --all-features

test-unit:
	cargo test --test unit --all-features

test-integration:
	cargo test --test integration --all-features

test-e2e:
	cargo test --test e2e --all-features

check:
	cargo check --workspace --all-targets --all-features

ci: fmt-check lint test

hooks-install:
	lefthook install

install-local:
	cargo install --path . --force

run-local: build
	./bin/mdv $(FILE)
