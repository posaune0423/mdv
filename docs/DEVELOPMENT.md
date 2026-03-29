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

## Git hooks

This repo uses [`lefthook`](https://github.com/evilmartians/lefthook) for local git hooks.

```bash
brew install lefthook
make hooks-install
```

Configured hooks:

- `pre-commit`: `cargo fmt --all -- --check` and `cargo check --workspace --all-targets --all-features`
- `pre-push`: `make ci`

| Command | Purpose |
|---------|---------|
| `make fmt` / `make fmt-check` | `rustfmt` |
| `make lint` | `clippy` with warnings denied |
| `make test` | All tests |
| `make test-unit` / `test-integration` / `test-e2e` | Split suites |
| `make build` | Build `target/release/mdv` and refresh the local `bin/mdv` copy so `./bin/mdv` runs the latest local build |
| `make build-tracked-bin` | Alias for `make build`; kept for the CI packaging path name |
| `make hooks-install` | Install git hooks from `lefthook.yml` into `.git/hooks` |

## Contributing

Issues and pull requests are welcome. Please run `make ci` before opening a PR so formatting, Clippy, and tests match what GitHub Actions enforces.

## Distribution

This repository currently has no release automation and no changelog workflow. Distribution is intentionally simple:

- `scripts/install.sh` downloads GitHub `main`'s CI-generated `bin/mdv` and installs it into the selected bin directory.
- `mdv update` downloads that same CI-generated `bin/mdv`, compares it to the current executable, and replaces the current file only when the bytes differ.
- Pushes to `main` run `.github/workflows/ci.yml`, which builds `bin/mdv` on macOS and commits the refreshed artifact back to `main` when the bytes change.
- `make build` refreshes the local `bin/mdv` copy as a runnable development artifact, while CI still owns the committed `main` branch `bin/mdv` that install/update consume.

### Contributors

Everyone who lands a change shows up automatically on the [GitHub contributors graph](https://github.com/posaune0423/mdv/graphs/contributors).
