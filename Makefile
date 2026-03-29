SHELL := /bin/sh
UNAME_S := $(shell uname -s)

.PHONY: build fmt fmt-check lint test test-unit test-integration test-e2e check ci install-local run-local release-metadata release-package release-assets-check

HOST_TARGET := $(shell rustc -vV | sed -n 's/^host: //p')
PACKAGE_VERSION := $(shell sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
TARGET ?= $(HOST_TARGET)
OUT_DIR ?= target/release-dist

build:
	mkdir -p bin
	cargo build --release
ifneq ($(UNAME_S),Darwin)
	cp target/release/mdv bin/mdv
else
	codesign --force --sign - target/release/mdv
	cp target/release/mdv bin/mdv
	codesign --force --sign - bin/mdv
endif

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

run-local: build
	./bin/mdv $(FILE)

release-metadata:
	./scripts/verify-release-metadata.sh v$(PACKAGE_VERSION)

release-package:
	./scripts/package-release.sh $(TARGET) $(OUT_DIR)

release-assets-check: release-metadata release-package
	./scripts/verify-release-archive.sh $(OUT_DIR)/mdv-$(TARGET).tar.gz $(OUT_DIR)/SHA256SUMS.part
