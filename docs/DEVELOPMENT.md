# Development

## Local helper script

The repository root includes a small `./mdv` shell helper that rebuilds the debug binary when sources change, then runs `target/debug/mdv`.

```bash
chmod +x ./mdv   # once, if needed
./mdv ./some.md
```

## Quality gates (same as CI)

```bash
make ci    # fmt-check, clippy -D warnings, full test suite
```

| Command | Purpose |
|---------|---------|
| `make fmt` / `make fmt-check` | `rustfmt` |
| `make lint` | `clippy` with warnings denied |
| `make test` | All tests |
| `make test-unit` / `test-integration` / `test-e2e` | Split suites |
| `make release-smoke` | Validate release metadata, package a host-native archive, and verify the packaged binary |

## Contributing

Issues and pull requests are welcome. Please run `make ci` before opening a PR so formatting, Clippy, and tests match what GitHub Actions enforces.

## Releases

Versioning is **Cargo-first**: update `Cargo.toml` and [CHANGELOG.md](../CHANGELOG.md), then create and push an annotated tag `vMAJOR.MINOR.PATCH`. [`.github/workflows/release.yml`](../.github/workflows/release.yml) builds archives and attaches them to a GitHub Release for that tag.

Before pushing the tag, run:

```bash
make release-smoke
```

The release workflow now rejects tags that do not match `Cargo.toml` and refuses to publish if the packaged archive cannot be extracted into a working `mdv` binary.

Once a release exists, installed users can update in place with:

```bash
mdv update
```

`mdv update` downloads the latest GitHub Release archive for the current host platform and replaces the currently running `mdv` executable, so an existing PATH entry keeps working when `mdv` was already being launched from PATH.

### Contributors

Everyone who lands a change shows up automatically on the [GitHub contributors graph](https://github.com/posaune0423/mdv/graphs/contributors).
