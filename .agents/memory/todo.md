## Current Task

- [x] raw HTML 復元後に `&nbsp;` などの double-escaped entity が残る再現点を固定する
- [x] supported raw HTML の text segment で entity を 1 段だけ戻す最小修正を入れる
- [x] 関連テストと build/run 経路で混入が消えたことを確認する

## Current Task Review

- Root cause: `restore_supported_raw_html` は `&lt;...&gt;` の tag だけを復元しており、raw HTML 内テキストに含まれる `&amp;nbsp;` のような double-escaped entity はそのまま残っていた。その結果、復元後の HTML に literal `&nbsp;` が混入した。
- Fix: `src/render/github_html/postprocess.rs` に double-escaped HTML entity を 1 段だけ戻す処理を追加し、code/pre の外側テキスト segment にだけ適用した。これで `&amp;nbsp;` は `&nbsp;` に戻り、ブラウザ側で通常どおり non-breaking space として解釈される。
- Regression coverage:
- `tests/unit/render_pipeline/github_html.rs` に raw HTML 内の `&nbsp;` が `&amp;nbsp;` のまま残らないことを確認するテストを追加した。
- 既存の centered fixture テストも再実行し、README 風 badge row の復元が壊れていないことを確認した。
- Exact-path verification:
- `make build && TERM=xterm-kitty ./bin/mdv ~/Work/daikolabs/marry-fun/README.md` を再実行し、出力上に literal `&nbsp;` が混入していないことを確認した。
- Verification:
- `cargo test github_html_decodes_double_escaped_entities_inside_restored_raw_html -- --nocapture` passed
- `cargo test centered_block_fixture_ignores_extra_attrs_but_restores_centered_html -- --nocapture` passed
- `cargo fmt --all --check` passed
- `make build && TERM=xterm-kitty ./bin/mdv ~/Work/daikolabs/marry-fun/README.md` passed

## Current Task

- [x] Reproduce the user's exact local run path and verify whether `bin/mdv` was stale
- [x] Make `make build` refresh `bin/mdv` so `make build && ./bin/mdv ...` uses the latest local binary
- [x] Re-run the user's command path and record the verification outcome below

## Current Task Review

- Root cause: `make build` only refreshed `target/release/mdv`, while the user's actual command ran `./bin/mdv`. The two binaries were different before the fix, so the user was launching a stale executable.
- Workflow fix: [`Makefile`](/Users/asumayamada/Private/posaune0423/mdv/Makefile) now refreshes `bin/mdv` as part of `make build`; `build-tracked-bin` remains as an alias name for the same local packaging path.
- Docs/tests: [`docs/DEVELOPMENT.md`](/Users/asumayamada/Private/posaune0423/mdv/docs/DEVELOPMENT.md) now states that `make build` refreshes the local runnable `bin/mdv`, and [`tests/integration/distribution_contract.rs`](/Users/asumayamada/Private/posaune0423/mdv/tests/integration/distribution_contract.rs) now requires that contract.
- Exact-path verification:
- `make build && ./bin/mdv ~/Work/daikolabs/marry-fun/README.md` was executed before the fix and showed that `./bin/mdv` was being used while stale.
- After the fix, `make build && shasum -a 256 target/release/mdv bin/mdv && ./bin/mdv ~/Work/daikolabs/marry-fun/README.md` produced matching SHA-256 values for `target/release/mdv` and `bin/mdv`.
- This exec environment is not Ghostty/Kitty, so the exact command still stops at `interactive mode requires Ghostty or Kitty`; however `TERM=xterm-kitty ./bin/mdv ~/Work/daikolabs/marry-fun/README.md` was also executed and entered the alternate-screen graphics path successfully.
- Verification:
- `cargo test make_build_refreshes_the_local_bin_copy -- --nocapture` passed
- `cargo fmt --all --check` passed

## Current Task

- [x] Add dedicated fixtures for `details/summary`, `picture/source`, and GitHub math expressions
- [x] Add regression tests proving those constructs render in GitHub HTML and math survives plain-text normalization
- [x] Extend raw HTML restoration and Comrak options to support those cases
- [x] Run targeted tests/formatting and record the outcome below

## Current Task Review

- Fixture coverage: added `tests/fixtures/gfm/html-details/`, `tests/fixtures/gfm/html-picture/`, and `tests/fixtures/gfm/math/` so GitHub-style disclosure markup, responsive image markup, and math syntax each have dedicated regression inputs.
- Raw HTML fix: `src/render/github_html/postprocess.rs` now restores `details`, `summary`, `picture`, and `source` tags in the rich HTML path, preserving only a safe subset of attributes such as `open`, `media`, and `srcset`.
- Math support: `src/render/markdown_pipeline.rs` now enables Comrak `math_dollars` and `math_code`, and `src/render/markdown.rs` preserves math nodes in plain-text normalization so headless output does not drop equations.
- Presentation tweak: `src/render/github_html/styles.rs` now gives display math a block layout in the GitHub HTML path so `$$...$$` formulas remain visibly separate.
- Docs: `docs/MARKDOWN.md` now lists GitHub-style math expressions as a supported feature.
- Verification:
- `cargo test details_fixture_restores_details_and_summary_markup -- --nocapture` passed
- `cargo test picture_fixture_restores_picture_and_source_markup -- --nocapture` passed
- `cargo test github_html_emits_github_style_math_markup -- --nocapture` passed
- `cargo test gfm_fixtures_generate_expected_html_fragments -- --nocapture` passed
- `cargo test preserves_math_text_in_plain_document_normalization -- --nocapture` passed
- `cargo test headless_render_keeps_math_expressions_visible -- --nocapture` passed
- `cargo test gfm_fixtures_render_through_webkit_and_terminal_graphics_path -- --nocapture` passed
- `cargo fmt --all --check` passed

## Current Task

- [x] Add a README-style raw HTML fixture covering centered block wrappers with extra attributes
- [x] Add regression tests that require those wrappers and nested images/links to be restored instead of escaped
- [x] Generalize raw HTML restoration so supported GitHub-style tags survive harmless extra attributes
- [x] Run targeted tests/formatting and record the outcome below

## Current Task Review

- Fixture coverage: added `tests/fixtures/gfm/html-centered-blocks/` with a README-style `<p align="center" style="...">` wrapper, stacked local images, and linked badge images that also carry ignored `style` attributes.
- Root cause: raw HTML restoration only accepted exact single-attribute forms such as `<p align="center">` and rejected otherwise-supported tags as soon as extra attributes appeared, so GitHub-style markup with harmless extras stayed escaped.
- Sanitization fix: `src/render/github_html/postprocess.rs` now restores supported block tags, anchors, and images after strictly parsing attributes, preserving the safe subset and dropping harmless extras instead of escaping the whole tag.
- Layout fix: `src/render/github_html/styles.rs` now centers singleton images for any `[align]` wrapper, instead of only handling the paragraph-specific case.
- Verification:
- `cargo test centered_block_fixture_ignores_extra_attrs_but_restores_centered_html -- --nocapture` passed
- `cargo test gfm_fixtures_generate_expected_html_fragments -- --nocapture` passed
- `cargo test github_html_uses_generic_align_rules_for_singleton_images -- --nocapture` passed
- `cargo test webkit_snapshot_renders_centered_block_fixture_assets -- --nocapture` passed
- `cargo fmt --all --check` passed

## Current Task

- [x] Add crates.io-oriented package metadata to `Cargo.toml`
- [x] Run a minimal manifest verification
- [x] Commit the metadata update
- [x] Push the commit to `origin/main`

## Current Task Review

- Package metadata: `Cargo.toml` now declares `rust-version`, a crates.io-friendly `description`, `keywords`, and `categories`.
- Discoverability: the added metadata aligns the package with terminal Markdown viewing on crates.io without inventing custom manifest fields such as `topic`.
- Verification:
- `cargo metadata --format-version 1 --no-deps >/dev/null` passed

## Current Task

- [x] Add a regression test that proves empty `.mdv-webkit` parent directories are removed after cleanup
- [x] Implement minimal workspace cleanup so `.mdv-webkit` does not linger when empty
- [x] Run targeted WebKit snapshot tests and record the outcome below

## Current Task Review

- Cleanup contract: WebKit snapshot teardown now removes the per-run workspace and also removes the parent `.mdv-webkit` directory when it is left empty.
- Scope guard: the new cleanup helper only prunes parents literally named `.mdv-webkit`, so unrelated directories are untouched.
- Regression coverage: `src/io/webkit_snapshot/tests.rs` now asserts that cleanup removes the empty `.mdv-webkit` parent after a workspace is created and torn down.
- Verification:
- `cargo test cleanup_workspace_removes_empty_mdv_webkit_parent -- --nocapture` passed
- `cargo test snapshot_workspace_uses_temp_dir -- --nocapture` passed
- `cargo fmt --all --check` passed

- [x] Move `bin/mdv` generation responsibility from humans to CI
- [x] Refresh `bin/mdv` automatically after pushes to `main`
- [x] Update local build/docs/tests to match the new CI-owned binary contract
- [x] Run the relevant verification and record the outcome below

## Current Task Review

- CI contract: `.github/workflows/ci.yml` now keeps the existing `checks` job and adds a `refresh_tracked_binary` job that runs only for non-bot pushes to `main`, builds `bin/mdv` on `macos-latest`, and commits the file back only when the bytes changed.
- Loop prevention: the refresh job is gated by `github.actor != 'github-actions[bot]'`, so the follow-up bot commit can still run checks without recursively regenerating `bin/mdv`.
- Build contract: `make build` now only builds `target/release/mdv`; `make build-tracked-bin` reproduces the CI packaging path locally without implying that contributors should hand-maintain the tracked repo binary.
- Docs and CLI copy: README, development/tech/structure docs, `llm.txt`, and the `update` subcommand help now describe `bin/mdv` as CI-generated from `main`, not manually refreshed by contributors.
- Regression coverage:
- `tests/integration/distribution_contract.rs` now requires the CI workflow to include the tracked-binary refresh job, bot-loop guard, and `git add bin/mdv` commit path.
- Verification:
- `cargo test --test integration ci_workflow_refreshes_the_tracked_binary_on_main_pushes -- --nocapture` passed
- `cargo test --test integration distribution_contract -- --nocapture` passed
- `cargo test --test integration help_output -- --nocapture` passed
- `cargo fmt --all --check` passed

## Current Task

- [x] Make `mdv --version` print only the numeric version string
- [x] Update regression coverage to require exact numeric-only stdout
- [x] Run the targeted verification and record the outcome below

## Current Task Review

- Contract change: `mdv --version` now prints only `CARGO_PKG_VERSION` followed by a newline, without the `mdv ` prefix.
- Implementation: `src/cli/mod.rs` intercepts the standalone `--version` and `-V` invocation before Clap emits its default banner, then exits successfully after printing the raw version string.
- Regression coverage: `tests/integration/help_output.rs` now requires exact stdout equality with `0.1.0\n`-style output instead of substring matching.
- Verification:
- `cargo test --test integration version_flag_prints_the_package_version -- --nocapture` passed
- `cargo test --test integration help_ -- --nocapture` passed
- `cargo run --quiet -- --version` printed `0.1.0`
- `cargo fmt --all --check` passed

## Current Task

- [x] Remove release, release-assets, and main-channel automation from the repository
- [x] Remove changelog and stale release references from docs, workflows, scripts, and tests
- [x] Change `scripts/install.sh` to download the tracked `main` branch `bin/mdv`, show ANSI loading feedback, and print the requested ASCII banner on success
- [x] Keep `mdv update`, but make it compare the current executable with GitHub `main`'s `bin/mdv` and replace only when newer content exists
- [x] Add regression coverage for `mdv --version` and the new install/update distribution contract
- [x] Run formatting and the relevant test suite, then record the outcome below

## Current Task Review

- Distribution model changed from release assets to a tracked repo artifact: `scripts/install.sh` now downloads GitHub `main`'s `bin/mdv`, shows ANSI spinner feedback during download, and prints the requested ASCII banner after install.
- `mdv update` was kept, but its behavior now matches the simplified model: it downloads GitHub `main`'s `bin/mdv`, compares it byte-for-byte with the current executable, and replaces the current file only when the contents differ.
- Release automation and related files were removed: `main-channel.yml`, `release-assets.yml`, `release.yml`, release packaging scripts, release-please metadata, and `CHANGELOG.md`.
- CLI/docs/tests were updated to match the new contract, and regression coverage now checks both `mdv --version` and the main-binary install/update path.
- Verification:
- `cargo fmt --all` passed
- `sh -n scripts/install.sh` passed
- `cargo test --workspace --all-targets --all-features` passed
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
- `cargo run --quiet -- --version` returned `0.1.0`
- `cargo run --quiet -- update --help` showed the new main-binary replacement help text

## Current Task

- [x] Add `lefthook` configuration for local git hooks using the existing repo quality gates
- [x] Add a simple install entrypoint for `lefthook`
- [x] Document the hook workflow and add regression coverage for the config
- [x] Run verification and record the outcome below

## Current Task Review

- Added [`lefthook.yml`](/Users/asumayamada/Private/posaune0423/mdv/lefthook.yml) with `pre-commit` gates for `cargo fmt --all -- --check` and `cargo check --workspace --all-targets --all-features`, plus a `pre-push` gate that reuses `make ci`.
- Added [`hooks-install`](/Users/asumayamada/Private/posaune0423/mdv/Makefile) to [`Makefile`](/Users/asumayamada/Private/posaune0423/mdv/Makefile) and documented the local hook workflow in [`docs/DEVELOPMENT.md`](/Users/asumayamada/Private/posaune0423/mdv/docs/DEVELOPMENT.md).
- Added regression coverage in [`tests/integration/distribution_contract.rs`](/Users/asumayamada/Private/posaune0423/mdv/tests/integration/distribution_contract.rs) so the repo keeps the expected hook commands.
- Installed `lefthook@2.1.3` via `mise use -g lefthook@2.1.3` and synced hooks with `lefthook install`.
- Verification:
- `cargo fmt --all` passed
- `cargo test --test integration distribution_contract -- --nocapture` passed
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
- `cargo check --workspace --all-targets --all-features` passed
- `lefthook install` synced `pre-commit` and `pre-push`
- `lefthook run pre-commit --force` passed
- `lefthook run pre-push --force` passed

## Documentation Task

- [x] Audit the repository entrypoints, rendering pipeline, packaging, and tests for `mdv`
- [x] Rewrite `docs/PRD.md` to describe the product from the current repository state
- [x] Create `docs/TECH.md` covering runtime architecture, dependencies, pipelines, and constraints
- [x] Create `docs/STRUCTURE.md` covering the source tree, module responsibilities, and test layout
- [x] Review the generated docs for accuracy against the implementation and record the outcome below

## Documentation Task Review

- Replaced the old PRD with an implementation-based description of the current product contract, including real runtime behavior, CLI options, supported blocks, and known scope limits.
- Added `docs/TECH.md` to document the actual dual rendering pipeline, key crates, external runtime dependencies, diagnostics flow, CI/release process, and current platform constraints.
- Added `docs/STRUCTURE.md` to map the repository layout, module boundaries, important files, and test organization from `src/` through `tests/`.
- Explicitly documented the current mismatch between README claims and runtime reality: rich interactive rendering depends on the macOS WebKit snapshot path and does not currently have a runtime fallback.
- Verification:
- `git diff --check` passed
- Reviewed `docs/PRD.md`, `docs/TECH.md`, and `docs/STRUCTURE.md` against the current implementation and test layout

# Current Task

- [x] Identify the heading and bold font-weight regression in the GitHub HTML rendering path
- [x] Restore GitHub-compatible heading and bold weight behavior and add regression coverage
- [x] Instrument the WebKit snapshot path so image and Mermaid render failures surface actionable diagnostics
- [x] Fix the WebKit snapshot helper execution path so local assets render reliably during the full test suite
- [x] Add regression tests for image rendering, Mermaid metrics, broken asset diagnostics, and GitHub typography matrices across headings and inline emphasis
- [x] Run `cargo fmt --all`, `cargo test --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`

## Review

- Root cause for typography: `src/render/github_html.rs` pinned `.markdown-body` to `font-variation-settings: "wght" 400`, which overrode GitHub's selector-specific `font-weight` rules and kept headings and `<strong>` text from reaching their intended weight.
- Typography fix: removed the prose-level weight lock while preserving the embedded GitHub font faces and added WebKit regression coverage that checks computed GitHub typography for `h1` through `h6`, `strong`, `em`, and inline `code`.
- Root cause for missing image and Mermaid diagnostics: the snapshot helper only returned a PNG and Mermaid CLI discarded stderr, so failures were hard to debug and tests could not assert rendered dimensions or aspect ratio.
- Diagnostics fix: `src/io/webkit_snapshot.rs` now emits a JSON report with font readiness, image metrics, Mermaid metrics, and computed typography weights; snapshot failures now include broken asset details instead of failing silently.
- Reliability fix: simplified the macOS snapshot helper execution path to always run a workspace-local `snapshot.swift` script. This removed the cached helper path that was triggering `WKWebView` sandbox failures in the full suite.
- Mermaid fix: `src/io/mermaid_cli.rs` now preserves stderr on renderer failures, so parse/runtime errors surface in logs and tests.
- Verification:
- `cargo fmt --all` passed
- `cargo test --workspace --all-targets` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Confirm whether the current runtime path uses embedded fonts or falls back to system fonts
- [x] Add a regression test for the remaining GitHub typography mismatch before changing implementation
- [x] Apply the minimum typography fix needed for closer GitHub fidelity
- [x] Run targeted tests and capture whether external font downloads are necessary

## Current Task Review

- Runtime font sourcing: the GitHub HTML snapshot path still embeds bundled `Mona Sans VF` and `Monaspace Neon Var` via `@font-face` data URLs, so no external font download is required for snapshot rendering.
- Root cause for the remaining mismatch: the renderer was overriding GitHub's own body and monospace font stacks, which made the output diverge from github.com even though the font assets were present.
- Root cause for the "default text looks too thin" follow-up: the renderer was also forcing `-webkit-font-smoothing: antialiased` and `text-rendering: optimizeLegibility` on `body`, which makes macOS WebKit text appear lighter than GitHub's own markdown styling.
- Typography fix: removed the `.markdown-body` body/code font-family overrides and kept GitHub's shipped CSS stacks in control so WebKit can resolve the same weight and family rules GitHub expects.
- Smoothing fix: removed the extra text smoothing overrides and added a regression test to keep the body typography closer to GitHub's default appearance.
- Regression coverage: `tests/unit/render_pipeline/github_html.rs` now asserts we do not reintroduce those font-stack overrides, and the WebKit typography diagnostics test expects the GitHub monospace stack instead of a forced custom mono face.
- Verification:
- `cargo test github_html_ -- --nocapture` passed
- `cargo test webkit_snapshot_matches_github_typography_for_headings_and_inline_emphasis -- --nocapture` passed

## Refactor Task

- [x] Stabilize the partial `src/ui/terminal` split and move tests into `src/ui/terminal/tests.rs`
- [x] Extract shared GFM parser options into a reusable render pipeline module
- [x] Split `src/render/github_html` into focused submodules
- [x] Split `src/io/webkit_snapshot` into focused submodules
- [x] Rewire moved render-pipeline tests under `tests/unit/render_pipeline/`
- [x] Run `cargo fmt --all`, `cargo test --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`

## Refactor Review

- The refactor preserved behavior at the module boundaries by keeping the existing render-pipeline fixtures, WebKit snapshot diagnostics tests, and terminal viewer tests green after the file splits.
- During verification, two pre-existing contract gaps surfaced and were fixed instead of papered over:
- Headless plain-text output now keeps Markdown link destinations as `label <url>`, matching the integration and plain-text tests.
- Interactive terminal rendering strips those link destinations back out and preserves punctuation spacing, so the TUI keeps the previous visible text contract.
- Verification:
- `cargo fmt --all` passed
- `cargo test --workspace --all-targets` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Add a regression test that captures the GitHub-compatible `caution` alert icon markup
- [x] Replace the hardcoded `caution` SVG with the GitHub-compatible icon path
- [x] Run targeted render-pipeline tests and record the outcome below

## Current Task Review

- Root cause: `src/render/github_html/postprocess.rs` injected a bespoke `octicon-stop` path for `Caution` that did not match GitHub's current stop icon, so the callout badge rendered with the wrong silhouette and looked crushed.
- Fix: replaced the hardcoded `Caution` SVG path with the GitHub-compatible stop octicon path while preserving the existing DOM shape and classes.
- Regression coverage: `tests/unit/render_pipeline/github_html.rs` now asserts that `CAUTION` alerts include the GitHub stop icon class and path fragment.
- Verification:
- `cargo test github_html_ -- --nocapture` passed
- `cargo fmt --all --check` passed
- `cargo check --workspace --all-targets` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Add a regression that requires the rich fixture image to render at a visibly non-trivial size
- [x] Replace the tiny example image fixture with a visible PNG asset
- [x] Run targeted snapshot and render checks and record the outcome below

## Current Task Review

- Root cause: the image pipeline was working, but `examples/rich_markdown.md` referenced a `4x4` fixture image. The GitHub HTML path renders images at intrinsic size, so the final WebKit snapshot only painted a `4px` image, which is effectively invisible in terminal viewing.
- Fix: replaced `examples/pixel.png` with a visible `96x96` PNG fixture while keeping the same filename and Markdown source, so the rich example now demonstrates image rendering instead of an almost invisible pixel.
- Regression coverage: [tests/unit/render_pipeline/webkit_snapshot.rs](../../tests/unit/render_pipeline/webkit_snapshot.rs#L271) now requires the rich fixture image to render at least `32px` wide and tall in the snapshot path.
- Verification:
- `cargo test webkit_snapshot_keeps_rich_fixture_image_and_mermaid_visible -- --nocapture` passed
- `cargo test webkit_snapshot_renders_local_markdown_images -- --nocapture` passed
- `cargo fmt --all --check` passed

## Current Task

- [x] Re-check the HTML typography path against live GitHub styles instead of the old local assumptions
- [x] Add regression coverage for the prose font stack and rendered heading/code typography
- [x] Update the HTML render CSS so headings and bold text resolve to the same stack and weight family GitHub uses
- [x] Run full verification and record the outcome below

## Current Task Review

- Root cause: the previous follow-up fix over-corrected toward the older vendored markdown CSS and away from GitHub's current live prose stack. The HTML renderer still looked off because the local expectations were stale, not because `font-weight: 600` was missing.
- Live GitHub verification: inspected `github.com` directly and confirmed current `.markdown-body` prose resolves to `Mona Sans VF` with the GitHub sans-serif fallback stack, while inline code resolves to GitHub's `ui-monospace` stack. Headings and `strong` remain `600`.
- HTML typography fix: `src/render/github_html/styles.rs` now explicitly applies the GitHub prose stack with `Mona Sans VF` to `.markdown-body`, uses the GitHub monospace stack for `code`/`pre`/`kbd`/`samp`, and keeps the embedded prose font face in standard `format('woff2')` form so WebKit loads it reliably.
- Regression coverage: `tests/unit/render_pipeline/github_html.rs` now checks for the embedded GitHub prose stack and valid WOFF2 syntax, while `tests/unit/render_pipeline/webkit_snapshot.rs` verifies rendered headings and strong/em text resolve to `Mona Sans VF`, and inline code resolves to the GitHub monospace stack with the expected metrics.
- Verification:
- `cargo fmt --all` passed
- `cargo test --workspace --all-targets` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Inspect the README source and current raw HTML handling in the markdown normalization path
- [x] Add a regression test for raw HTML blocks and inline HTML being preserved as plain text
- [x] Fix markdown normalization so raw HTML is visible instead of being dropped
- [x] Run targeted verification and record the outcome below

## Current Task Review

- Root cause: `src/render/markdown.rs` treated `NodeValue::HtmlBlock` like a normal AST subtree and collected children, but raw HTML blocks keep their source in `literal` and have no text children. `NodeValue::HtmlInline` was also ignored, so inline tags disappeared inside paragraphs.
- Fix: map `HtmlBlock` directly to a paragraph containing the block literal as plain text, trimming only trailing line endings, and keep `HtmlInline` literals as inline text segments.
- README impact: headless and structured TUI rendering now show the README's wrapper tags like `<div align="center">` and `<br/>` as plain text instead of silently dropping them, which matches the README's own contract for raw HTML.
- Regression coverage: `tests/unit/markdown_normalize.rs` now asserts both block HTML and inline HTML survive normalization as plain text.
- Verification:
- `cargo fmt --all --check` passed
- `cargo check --workspace --all-targets` passed
- `cargo test markdown_normalize -- --nocapture` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Reproduce the README display failure from an actual user-facing invocation path
- [x] Add an integration test for `mdv -` reading README-like Markdown from stdin
- [x] Implement stdin support for headless rendering
- [x] Run verification and record the outcome below

## Current Task Review

- Reproduction path: `cat README.md | mdv -` failed with `No such file or directory (os error 2)` because the CLI treated `-` as a literal filename instead of standard input.
- Fix: `src/app/mod.rs` now detects `-`, reads Markdown from stdin in headless mode, parses it using a virtual cwd-based path, and returns a clear error when `-` is used without piped stdin.
- README impact: piping the repository README through `mdv -` now renders the document correctly, including the raw HTML tags that the previous normalization fix preserved as plain text.
- Regression coverage: `tests/integration/render_features.rs` now asserts `mdv -` succeeds with stdin content that includes README-like inline HTML.
- Verification:
- `cargo test --test integration -- --nocapture` passed
- `cat README.md | cargo run --quiet -- - | sed -n '1,24p'` showed the README content correctly
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Re-check the actual `mdv README.md` interactive path instead of only headless rendering
- [x] Add a regression test that requires runtime fallback when graphic page rendering is unavailable
- [x] Make interactive startup and reload fall back to terminal rendering instead of failing
- [x] Run verification and record the outcome below

## Current Task Review

- Root cause: the previous fixes only covered headless rendering and stdin input. In the real `mdv README.md` path, interactive startup still required `render_graphic_page()` to succeed. If snapshot rendering failed on the user's machine, the viewer failed instead of opening the README in the terminal-native renderer.
- Fix: `src/ui/terminal/mod.rs` now uses a shared render-state builder so both startup and rerender fall back to `render_document()` with a warning when graphic page rendering is unavailable.
- README impact: `mdv README.md` still uses the full graphic page path when it works, but now degrades to the structured terminal renderer instead of failing to display the README.
- Regression coverage: `src/ui/terminal/tests.rs` now asserts `TerminalViewer::try_new()` succeeds and falls back to terminal rendering when graphic mode fails.
- Verification:
- `cargo test --test integration -- --nocapture` passed
- `cargo test --lib ui::terminal::tests::try_new_falls_back_to_terminal_render_when_graphic_mode_fails -- --nocapture` passed
- `TERM_PROGRAM=ghostty cargo run --quiet -- README.md` started successfully and rendered the viewer until quit
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Remove the interactive TUI fallback that was masking graphic-render failures
- [x] Make startup failures surface actionable error text instead of degrading silently
- [x] Re-run integration, targeted unit tests, clippy, and release build

## Current Task Review

- User direction change: fallback was not acceptable. The interactive viewer should fail loudly with debuggable errors, not open a different renderer.
- Runtime change: `src/ui/terminal/mod.rs` no longer falls back to `render_document()` in `try_new()`. Startup now returns `interactive graphic render failed for <path>: ...`, which `main` prints to stderr and exits with code `1`.
- Runtime behavior on later rerenders: resize/reload keeps the existing graphic page and stores the failure warning instead of swapping into the TUI path.
- Test coverage: `src/ui/terminal/tests.rs` now asserts `TerminalViewer::try_new()` returns an error containing both the startup context and the underlying graphic failure reason.
- Verification:
- `cargo test --test integration -- --nocapture` passed
- `cargo test --lib ui::terminal::tests::try_new_surfaces_graphic_mode_failure -- --nocapture` passed
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `make build` passed

## Current Task

- [x] Add dedicated README-like fixtures for raw HTML wrappers and badge image rows under `tests/fixtures/gfm/`
- [x] Split render-pipeline coverage into per-fixture HTML and WebKit snapshot tests for those cases
- [x] Fix the graphic render path so README-style badge/image handling no longer prevents rendering
- [x] Run targeted tests, `cargo fmt --all --check`, and `cargo clippy --workspace --all-targets -- -D warnings`

## Current Task Review

- Root cause: the WebKit snapshot helper treated every broken image as fatal. README-specific remote badge SVGs could resolve to `complete=true` but `naturalWidth=0`, which aborted the whole snapshot path before the rest of the page rendered.
- Fixture split: added dedicated README-like fixtures under `tests/fixtures/gfm/html-wrappers`, `tests/fixtures/gfm/badges-local`, and `tests/fixtures/gfm/badges-remote` so raw HTML wrappers, successful badge rows, and failing remote badges are each covered independently.
- Regression coverage: `tests/unit/render_pipeline/gfm_fixtures.rs` now has explicit HTML-wrapper and badge HTML-fragment assertions, and `tests/unit/render_pipeline/webkit_snapshot.rs` now verifies both that local badge SVGs render at non-trivial size and that remote badge failures do not abort the page snapshot.
- Runtime fix: `src/io/webkit_snapshot/script.rs` now distinguishes blocking local assets from non-blocking remote assets. Local missing images still fail fast, but remote badges get a short chance to load and then stop blocking snapshot completion, which keeps README rendering debuggable instead of blank.

## Current Task

- [x] Define the expected CD path for publishing release assets to GitHub Releases
- [x] Add a repeatable local packaging/verification script that matches the workflow artifact format
- [x] Update the GitHub Actions release workflow to reuse that packaging path and tighten release publication behavior
- [x] Add an `mdv update` command contract that fetches the latest GitHub Release asset for the host platform
- [x] Change the default theme contract to `system` and resolve it from the host OS at runtime
- [x] Implement in-place binary replacement so the updated executable stays at the same path the user already invokes
- [x] Refresh release/update documentation and re-run verification for packaging, installability, CLI help, and workflow syntax; record the outcome below

## Current Task Review

- CD hardening: added `scripts/verify-release-metadata.sh`, `scripts/package-release.sh`, and `scripts/verify-release-archive.sh`, then rewired `.github/workflows/release.yml` and `.github/workflows/ci.yml` to reuse the same release packaging path that local `make release-smoke` now exercises.
- Release guardrails: the release workflow now validates that the pushed tag matches `Cargo.toml`, checks that `CHANGELOG.md` has the matching release section, verifies each packaged archive can be extracted, and only then publishes the GitHub Release.
- Self-update: `mdv update` now resolves the host release asset, downloads the latest GitHub Release archive, extracts the top-level `mdv` binary, and replaces the current executable in place so an existing PATH entry keeps resolving to the updated binary.
- Theme default: `--theme` now defaults to `system`, and the runtime resolves that to the host OS light/dark preference before rendering.
- Docs sync: README and project docs now mention `mdv update`, the `system` default theme, and the release-smoke / tag-validation workflow.
- Verification:
- `cargo run --quiet -- --help` passed
- `cargo run --quiet -- update --help` passed
- `cargo test --workspace --all-targets --all-features` passed
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
- `make release-smoke` passed
- `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); YAML.load_file(".github/workflows/release.yml")'` passed
- `git diff --check` passed
- Verification:
- `cargo test gfm_fixtures -- --nocapture` passed
- `cargo test webkit_snapshot_allows_remote_badge_failures_without_aborting_page_render -- --nocapture` passed
- `cargo test webkit_snapshot_surfaces_broken_image_diagnostics -- --nocapture` passed
- `cargo test --test unit -- --nocapture` passed
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Reproduce the user's exact `make build && ./bin/mdv AGENTS.md` failure before closing the task
- [x] Fix the release artifact path so the built binary survives macOS execution policy checks
- [x] Re-run build, lint, format, and the exact interactive startup command

## Current Task

- [x] Inspect the current install/update asset resolution and main-vs-release packaging path
- [x] Add regression coverage for the new main-channel install/update contract before changing implementation
- [x] Publish a rolling `main` binary channel and switch `scripts/install.sh` plus `mdv update` to consume it
- [x] Replace `release-smoke` naming with clearer packaging terminology where it still appears
- [x] Rework stable release automation around `release-please` while keeping asset packaging automated in GitHub Actions
- [x] Re-run packaging, tests, help output, workflow syntax checks, and record the outcome below

## Current Task Review

- Install/update contract: `scripts/install.sh` and `mdv update` now default to the rolling `main` channel instead of `releases/latest`, and both accept `MDV_CHANNEL` to pin a specific release tag when needed.
- Rolling binaries: added [`.github/workflows/main-channel.yml`](../../.github/workflows/main-channel.yml) to build all supported targets on every `main` push, move the `main` tag forward, and refresh the prerelease assets that install/update consume.
- Stable release automation: replaced manual tag-push release publishing with [`.github/workflows/release.yml`](../../.github/workflows/release.yml) driven by `googleapis/release-please-action`, plus [`release-please-config.json`](../../release-please-config.json) and [`.release-please-manifest.json`](../../.release-please-manifest.json).
- Packaging reuse: extracted the shared multi-target archive build into [`.github/workflows/package-artifacts.yml`](../../.github/workflows/package-artifacts.yml) so rolling `main` and stable releases use the same packaging path.
- CI naming: renamed the old `release-smoke` CI job/target to `packaging` / `package-check` so the CI workflow no longer uses release wording for a verification-only job.
- Docs sync: updated README, `llm.txt`, and project docs so install/update, release automation, and packaging commands match the new workflow.
- Verification:
- `cargo fmt --all --check` passed
- `cargo test --workspace --all-targets --all-features` passed
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
- `cargo run --quiet -- update --help` passed
- `make package-check` passed
- `ruby -e 'require "yaml"; %w[.github/workflows/ci.yml .github/workflows/main-channel.yml .github/workflows/package-artifacts.yml .github/workflows/release.yml].each { |f| YAML.load_file(f) }'` passed
- `git diff --check` passed

## Current Task Review

- Root cause: the previous close-out stopped at tests and partial startup checks. The user's exact release command still failed because macOS was killing `./bin/mdv` with `SIGKILL (Code Signature Invalid)` under `taskgated`.
- Evidence: unified logs and the generated `.ips` crash reports showed `namespace: "CODESIGNING"` and `indicator: "Invalid Page"` / `Taskgated Invalid Signature`, so this was not a renderer bug.
- Fix: `Makefile` now re-signs both `target/release/mdv` and `bin/mdv` on Darwin immediately after `cargo build --release` and after copying into `bin/`, so the executable the user runs is the one that was just signed.
- Final verification now includes the real command path, not just tests:
- `make build && TERM_PROGRAM=ghostty ./bin/mdv AGENTS.md` reached interactive viewer startup and rendered the page instead of getting killed
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed

## Current Task

- [x] Reproduce the user's exact `make build && ./bin/mdv README.md` blank-screen behavior
- [x] Compare README and `AGENTS.md` diagnostics to isolate the README-specific render difference
- [x] Implement the minimum fix and add targeted regression coverage if needed
- [x] Re-run the exact user command plus final verification before closing

## Current Task Review

- Root cause: README was no longer failing in HTML/WebKit generation, but interactive page mode still transmitted the entire full-page snapshot PNG to Ghostty/Kitty as one image and relied on placement-time source cropping. `AGENTS.md` stayed under that practical limit; `README.md` produced a very tall raster, so the terminal could end up showing a blank page even though snapshot generation itself succeeded.
- Fix: `src/ui/page_graphics.rs` now exposes `viewport_raster()`, which crops the already-generated snapshot down to just the currently visible viewport and re-encodes only that slice as PNG. `src/ui/terminal/graphics.rs` now transmits that viewport-sized raster per draw instead of uploading the whole README page as a single giant image.
- Runtime impact: the exact `make build && ./bin/mdv README.md` path now reaches the viewer with a first-frame PNG sized to the visible viewport instead of the full document height, removing the README-specific giant-image path that was unique versus `AGENTS.md`.
- Regression coverage:
- `tests/unit/page_graphics.rs` now asserts the viewport raster encoder emits only the visible crop with the expected dimensions.
- `src/ui/terminal/tests.rs` now asserts page-mode viewport commands retransmit a cropped PNG for each scroll position instead of relying on one cached full-page transfer.
- Verification:
- `cargo test --test unit -- --nocapture` passed
- `cargo test --lib ui::terminal::tests::page_viewport_commands_retransmit_cropped_png_for_each_scroll_position -- --nocapture` passed
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `make build && ./bin/mdv README.md` reached interactive startup, and the first-frame transfer observed in the PTY was reduced to a viewport-sized PNG (`826x475`) instead of the previous full-document raster upload

## Current Task

- [x] Inspect rich-render raw HTML handling for README-style wrappers
- [x] Add regression coverage for supported README raw HTML in the GitHub HTML path
- [x] Restore supported raw HTML layout behavior like `<div align="center">` without enabling unsafe HTML wholesale
- [x] Re-run exact README startup plus final verification

## Current Task Review

- Root cause: the interactive GitHub HTML path still had `comrak` configured to escape all raw HTML, so README wrappers like `<div align="center">` and inline `<br/>` were rendered as literal text even though the snapshot path itself was now working.
- Fix: `src/render/github_html/postprocess.rs` now restores a tightly scoped allowlist of escaped raw HTML tags after Markdown rendering: `div` with safe `align` values, `br`, `sub`, `sup`, and anchors with safe `href` values. This keeps `script` and unsupported tags escaped while letting README-style presentational wrappers affect layout in the rich viewer.
- Styling: `src/render/github_html/styles.rs` now adds explicit alignment rules for `[align="center" | "left" | "right"]` so centered wrappers render predictably in the snapshot browser.
- Regression coverage:
- `tests/unit/render_pipeline/gfm_fixtures.rs` and `tests/fixtures/gfm/html-wrappers/expected-substrings.txt` now expect the centered wrapper and inline break to be restored as real HTML in the rich path.
- `tests/unit/render_pipeline/github_html.rs` now asserts supported raw HTML is restored while `<script>` remains escaped.
- Verification:
- `cargo test --test unit render_pipeline::gfm_fixtures -- --nocapture` passed
- `cargo test --test unit render_pipeline::github_html -- --nocapture` passed
- `cargo test --test unit -- --nocapture` passed
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `make build && ./bin/mdv README.md` reached interactive startup again after the raw-HTML restore change

## Current Task

- [x] Apply a small cleanup refactor on the README-rendering changes before handoff
- [x] Re-run final verification after the refactor
- [x] Commit the rendering fixes and push the branch

## Current Task Review

- Refactor scope: kept the cleanup narrow to avoid destabilizing the rendering fixes. `src/render/github_html/styles.rs` now separates common viewer layout overrides from theme-specific code-block colors, so the new alignment support does not duplicate the shared CSS in both light and dark themes.
- Final verification after the cleanup:
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `cargo test --test unit -- --nocapture` passed
- `make build && ./bin/mdv README.md` reached interactive startup after the refactor as well

## Current Task

- [x] Inspect Mermaid renderer theme handling in rich and interactive paths
- [x] Add failing regression coverage for dark-theme Mermaid rendering
- [x] Propagate the selected viewer theme into Mermaid rendering
- [x] Re-run final verification including dark-theme README launch

## Current Task Review

- Root cause: Mermaid CLI rendering was theme-agnostic. The viewer theme affected surrounding HTML and syntax colors, but Mermaid diagrams were always rendered with Mermaid CLI defaults because no `-t/--theme` argument was passed.
- Fix: `src/io/mermaid_cli.rs` now accepts `Theme` for Mermaid renders, maps viewer `light` to Mermaid `default` and viewer `dark` to Mermaid `dark`, includes that theme in the render cache key, and passes `-t <theme>` to Mermaid CLI. The theme is now propagated from all rendering entry points:
- `src/render/github_html/mermaid.rs` for the WebKit/GitHub HTML snapshot path
- `src/render/text.rs` for headless plain-text Mermaid rendering
- `src/ui/terminal/mod.rs` for interactive lazy Mermaid rasterization
- Regression coverage:
- `tests/unit/mermaid_cli.rs` now asserts Mermaid CLI receives `-t dark` and that cached output is separated by theme.
- Verification:
- `cargo test --test unit mermaid_cli -- --nocapture` passed after the new theme tests were added
- `cargo test --test unit -- --nocapture` passed
- `cargo fmt --all --check` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
- `make build && ./bin/mdv --theme dark README.md` reached interactive startup with the dark-theme code path

## Current Task

- [ ] Define a lightweight changesets-style contract that fits this Rust-only repo
- [ ] Implement release-fragment tooling that updates `Cargo.toml` and `CHANGELOG.md`
- [ ] Add regression coverage for fragment application and metadata validation
- [ ] Update contributor/release docs for the new workflow
- [ ] Re-run verification and record the outcome below

## Current Task Review

- Scope: added a repo-native changesets-style release flow without introducing a Node toolchain.
- Contract:
- release fragments now live under `.changeset/*.md`
- each fragment declares `bump: patch|minor|major` front matter and carries the markdown body that should land in the release notes
- `scripts/apply-changesets.sh` now computes the highest pending bump, updates `Cargo.toml`, folds fragment bodies into `CHANGELOG.md`, updates compare/release links, and deletes the processed fragments
- `scripts/verify-release-metadata.sh` now also rejects releases while pending `.changeset/*.md` files still exist
- Developer workflow:
- `make release-prepare` applies pending fragments
- `make package-check` still verifies release metadata and packaged archives before tagging
- Regression coverage:
- `tests/release_changesets.rs` now verifies fragment application, version bumping, changelog link updates, fragment cleanup, and release metadata rejection when pending fragments remain
- Verification:
- `cargo test --test release_changesets -- --nocapture` passed
- `cargo clippy --test release_changesets -- -D warnings` passed
- `cargo fmt --all --check` passed
- `make release-prepare` passed with `changesets: no pending fragments`
- `make package-check` passed
- `git diff --check` passed
- Repo-wide verification status:
- `cargo test --workspace --all-targets --all-features` is currently blocked by an existing uncommitted test expectation in `tests/integration/distribution_contract.rs` that requires a release-please-driven workflow (`release_workflow_is_release_please_driven`)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` is currently blocked by existing unwrap/expect violations in other modified test files outside this task's scope

## Current Task

- [x] Add failing coverage for the staged `mdv update` flow, including GitHub version parsing and install yes/no messaging
- [x] Rework `src/io/self_update.rs` to show install-like loading/status text, compare GitHub vs local versions, and emit the install decision clearly
- [x] Run targeted verification and record the outcome below

## Current Task Review

- `mdv update` now prints staged, install-style status lines instead of jumping straight to a byte-compare result. The flow is now: check GitHub main state, show GitHub version, show local version, download the latest tracked binary, and print `Latest install required: yes|no`.
- Install decisions are now version-aware. If GitHub main is newer than the local binary, `mdv update` installs it; if the local binary is already on the same or newer version, it prints `Latest install required: no` and skips replacement instead of blindly downgrading to whatever bytes are on `main`.
- The no-op path now explains why it skipped, and the terminal copy includes versioned end-state messages such as `Already on v0.1.1` and `Successfully updated to vX.Y.Z`.
- The package version was bumped from `0.1.0` to `0.1.1`, and the product doc now reflects the new repository version.
- Verification:
- `cargo fmt --all` passed
- `cargo test --lib self_update -- --nocapture` passed
- `cargo test --workspace --all-targets --all-features` passed
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
- `cargo build && tmp-copy-of-target-debug-mdv update` showed:
- `GitHub main version: v0.1.0`
- `Local mdv version: v0.1.1`
- `Latest install required: no`
- `Already on v0.1.1`
