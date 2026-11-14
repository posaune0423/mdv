# Development

Building from source on macOS requires Xcode Command Line Tools, including the Swift compiler. Cargo compiles the WebKit helper and bundles it into `mdv`, so running the distributed binary does not require a Swift compiler. Linux builds do not compile the helper.

## Measuring startup time

Build the release binary and measure the actual interactive mode in a PTY with fixed dimensions.

```bash
cargo build --release
uv run scripts/benchmark-startup.py --runs 7 --output /tmp/mdv-startup.json --capture /tmp/mdv-first-frame.png -- ./target/release/mdv README.md
```

Measurement starts immediately before process creation and ends when the PTY receives the first page image transmission and placement command. The default size is 120 columns by 40 rows, with 8 by 16 px cells. The benchmark verifies PNG transmission, so a plain-text fallback does not count as success. It saves the first run, individual run values, median, and binary SHA-256 to JSON; `--capture` saves the PNG of the first transmitted viewport. It does not include the time Ghostty/Kitty's compositor takes to display the image. Use the same document, theme, and dimensions for comparisons, do not run builds or tests concurrently, and do not equate the first run with an empty OS cache.

The rendering worker is reused for 120 seconds. Record startup without an existing worker separately from opening another document with an already running worker. Completed images are not cached; changes to local images, documents, themes, and dimensions are reflected in every render. The worker helper and logs are stored in `/tmp/mdv-render-<effective-uid>-<helper-hash>/`, whose parent directory is accessible only to its owner. Headless display does not start a worker. Retaining the worker and decoded RGBA data uses additional memory.

Set `MDV_PROFILE_STARTUP=1 RUST_LOG=mdv=info` to output timings for each Rust stage. Use `--transcript /tmp/mdv-startup.pty` to save raw PTY output. To enable per-stage helper logs, first start the worker from a command with those environment variables set.

`docs/benchmarks/startup-2026-10-22.json` records the initial improvements. At that time, the measurement tool rescanned the entire buffer whenever it received a large PNG, so the results, especially for README, include measurement overhead. For subsequent comparisons, use the corrected tool that checks only the tail, and remeasure the pre-change binary with that same tool.

## Measuring scrolling

```bash
uv run scripts/benchmark-scroll.py --output /tmp/mdv-scroll.json -- ./target/release/mdv README.md
uv run scripts/benchmark-scroll.py --input-kind wheel --duration 1 --output /tmp/mdv-wheel.json -- ./target/release/mdv docs/TECH.md
```

Using a fixed-size PTY, the benchmark records the first position change after a single input, frame intervals during continuous input, and image transmission counts and byte totals. `first_changed_frame_latency_ms` excludes initial frames in which the image does not change. Wheel measurements validate SGR wheel events sent by the terminal; they do not measure the feel or rendering time of a physical trackpad and compositor. Do not run builds, tests, or other measurements concurrently when comparing results.

At most two images are retained, each up to approximately three viewport heights. Only the visible region is transmitted initially; surrounding regions are prepared in the background and sent ahead of use. The entire page is not turned into a terminal texture. PNG chunks contain no more than 4096 base64 bytes, and no other graphics command is interleaved during a transmission. Image data is released on reload, resize, and exit, and stale prefetch results are discarded.

## Local helper script

The repository root includes a small `./mdv` shell helper that rebuilds the debug binary when sources change, then runs `target/debug/mdv`.

```bash
chmod +x ./mdv   # once, if needed
./mdv ./some.md
```

## Rust quality gates

```bash
make ci    # fmt-check, clippy -D warnings, full test suite
```

CI additionally checks tracked text for literal Japanese; Unicode test data should use escapes.

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

## Windows development and verification

Windows x64 launches an installed Edge/Chrome browser with a dedicated temporary profile to render HTML/CSS, embedded fonts, images, and SVG to PNG. `MDV_BROWSER` specifies an explicit browser executable path. macOS retains the existing WebKit/TIFF backend.

Windows image display uses WezTerm's OSC1337 support. Windows Terminal's Kitty keyboard/input support is separate from image display and cannot be used for this viewer's image mode. The resident images and source-crop updates used by Ghostty/Kitty on macOS remain unchanged. WezTerm receives the visible region as a lossless PNG on every frame, so the same transfer volume and frame rate as the Mac resident-image approach are not guaranteed.

```powershell
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --release
.\target\release\mdv.exe README.md
```

Windows CI requires Edge and verifies fonts/images/SVG, pixels at the end of long documents, local image updates, missing-image diagnostics, actual CLI PNG transmission/scrolling/resize/exit through ConPTY, replacement of a running executable, and fresh/overwrite installs using the PowerShell installer. Verifying ConPTY receipt and checking actual rendering and input feel in the WezTerm compositor are separate boundaries.

Chrome is required on macOS only when running Chromium regression tests for the Windows backend.

```bash
MDV_TEST_BROWSER='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' cargo test --lib chromium_snapshot -- --include-ignored
```

For distribution, CI builds macOS/Windows artifacts from the same source SHA, and a single publisher updates `bin/mdv` and `bin/mdv.exe` together. If main has advanced, old artifacts are not applied to newer source; publication is left to the next run. PRs produce downloadable artifacts without modifying the binaries on main.
