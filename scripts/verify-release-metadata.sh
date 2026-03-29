#!/bin/sh

set -eu

die() {
  printf 'release metadata: %s\n' "$1" >&2
  exit 1
}

tag="${1:-}"
[ -n "$tag" ] || die "usage: scripts/verify-release-metadata.sh vMAJOR.MINOR.PATCH"

case "$tag" in
v*) ;;
*) die "tag must start with v: $tag" ;;
esac

cargo_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
[ -n "$cargo_version" ] || die "could not read package version from Cargo.toml"

expected_tag="v${cargo_version}"
[ "$tag" = "$expected_tag" ] || die "tag ${tag} does not match Cargo.toml version ${cargo_version}"

grep -Fq "## [${cargo_version}]" CHANGELOG.md \
  || die "CHANGELOG.md is missing a section for ${cargo_version}"

printf 'release metadata verified for %s\n' "$tag"
