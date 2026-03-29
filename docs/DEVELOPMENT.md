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
| `make release-assets-check` | Validate release metadata, build a host-native release archive, and verify the extracted binary |

## Contributing

Issues and pull requests are welcome. Please run `make ci` before opening a PR so formatting, Clippy, and tests match what GitHub Actions enforces.

## Releases

Stable releases are **release-please-driven**. Push conventional commits such as `feat: ...`, `fix: ...`, and `deps: ...` to `main`; [`.github/workflows/release.yml`](../.github/workflows/release.yml) runs `release-please`, updates or opens the release PR, and when that PR lands, creates the `vMAJOR.MINOR.PATCH` tag plus GitHub Release.

Typical commit prefixes:

```text
feat: add watch-mode reload debounce
fix: preserve inline HTML in README rendering
chore: tighten release asset checks
```

Before merging a release PR, run:

```bash
make release-assets-check
```

The release workflow only builds and uploads stable assets after `release-please` reports `release_created=true`. Archive creation is shared with the rolling `main` channel, and `make release-assets-check` mirrors that release-asset path locally.

Once a release exists, installed users can update in place with:

```bash
mdv update
```

`mdv update` downloads the latest published archive for the selected channel on the current host platform and replaces the currently running `mdv` executable, so an existing PATH entry keeps working when `mdv` was already being launched from PATH. The default channel is `main`, and `MDV_CHANNEL=vMAJOR.MINOR.PATCH mdv update` lets you pin a specific release tag instead.

### Contributors

Everyone who lands a change shows up automatically on the [GitHub contributors graph](https://github.com/posaune0423/mdv/graphs/contributors).
