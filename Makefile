SHELL := /bin/sh

.PHONY: fmt fmt-check lint test test-unit test-integration test-e2e check ci install-local run-local

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

install-local:
	cargo install --path . --force

run-local:
	./mdv $(FILE)
