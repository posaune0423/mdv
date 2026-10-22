# Development

macOS のソースビルドには Swift コンパイラを含む Xcode Command Line Tools が必要です。Cargo のビルド時に WebKit helper をコンパイルして `mdv` に同梱するため、配布バイナリの実行時には Swift コンパイラは不要です。Linux ビルドでは helper をコンパイルしません。

## 起動時間の計測

release バイナリをビルドして、実際の対話モードを固定サイズの PTY で計測できます。

```bash
cargo build --release
uv run scripts/benchmark-startup.py --runs 7 --output /tmp/mdv-startup.json --capture /tmp/mdv-first-frame.png -- ./target/release/mdv README.md
```

計測範囲はプロセス生成直前から、最初のページ画像の送信と配置命令を PTY で受信するまでです。既定は 120 列 × 40 行、セルは 8 × 16 px。PNG の送信を検証するため、plain-text fallback は成功として数えません。初回、各回の値、中央値、バイナリの SHA-256 を JSON に保存し、`--capture` で最初に送信した viewport の PNG を保存します。Ghostty/Kitty の compositor が画面に描画する時間は含みません。比較時は同じ文書・テーマ・サイズを使用し、ビルドやテストを同時に走らせず、初回を OS キャッシュが空の状態と同一視しないでください。

描画 worker は120秒間再利用されます。worker が存在しない起動と、起動済み worker で別の文書を開くケースを分けて記録してください。画像の完成結果はキャッシュせず、ローカル画像・文書・テーマ・サイズの変更を毎回描画へ反映します。worker の helper とログは `/tmp/mdv-render-<effective-uid>-<helper-hash>/` に置き、親ディレクトリは所有者だけがアクセス可能です。headless 表示では worker を起動しません。worker と展開済み RGBA の保持には追加のメモリを使用します。

`MDV_PROFILE_STARTUP=1 RUST_LOG=mdv=info` を付けると Rust の段階別時間を出力できます。`--transcript /tmp/mdv-startup.pty` で PTY の生出力を保存できます。helper の段階別ログを有効にするには、その環境変数を設定したコマンドから worker を初めて起動します。

`docs/benchmarks/startup-2026-10-22.json` は最初の改善時の記録です。当時の計測器は大きな PNG を受信するたびに全バッファを再走査していたため、特に README の測定値に計測器の負荷を含みます。以降の比較では末尾だけを検査する修正版を使い、修正前バイナリも同じ計測器で測り直します。

## スクロールの計測

```bash
uv run scripts/benchmark-scroll.py --output /tmp/mdv-scroll.json -- ./target/release/mdv README.md
uv run scripts/benchmark-scroll.py --input-kind wheel --duration 1 --output /tmp/mdv-wheel.json -- ./target/release/mdv docs/TECH.md
```

固定 PTY で単発入力の最初の位置変更、連続入力中のフレーム間隔、画像の転送回数・バイト数を記録します。`first_changed_frame_latency_ms` は画像が変わらない最初の frame を除外します。ホイール計測は terminal が送る SGR wheel event の検証で、実際の trackpad と compositor の感触・描画時間は対象外です。比較時はビルド・テストや別の計測を同時実行しないでください。

画像は最大2枚、各画像はおよそ3 viewport 高までを保持します。最初は可視領域だけを転送し、周辺領域はバックグラウンドで準備して先読み転送します。全ページを端末の texture にしません。PNG の chunk は4096 base64 bytes以下で、転送途中に別の graphics command を挟みません。reload・resize・終了で画像データを解放し、古い先読み結果は破棄します。

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

## Windows の開発・検証

Windows x64 はインストール済み Edge/Chrome を専用の一時 profile で起動し、HTML/CSS・埋め込みフォント・画像・SVGを PNG として描画します。`MDV_BROWSER` はブラウザ実行ファイルの明示パスです。macOS は既存の WebKit/TIFF を維持します。

Windows の画像表示は WezTerm の OSC1337 を使用します。Windows Terminal の Kitty keyboard/input 対応は画像表示とは別であり、この viewer の画像モードには利用できません。macOS Ghostty/Kitty の常駐画像・source crop 更新はそのままです。WezTerm では毎フレームの可視領域を可逆 PNG で送るため、Mac の常駐画像方式と同じ転送量・フレームレートは保証しません。

```powershell
cargo test --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --release
.\target\release\mdv.exe README.md
```

Windows CI は Edge の存在を必須にし、フォント/画像/SVG・長文末尾の画素・ローカル画像更新・欠損画像診断、ConPTYを通した実CLIのPNG転送・スクロール・resize・終了、実行中exeの置換、PowerShellインストーラの新規/上書きインストールを検証します。ConPTY受信の検証と実際のWezTerm compositorの描画・入力の感触は別の境界です。

macOS 上で Windows backend の Chromium 回帰テストを実行する場合のみ Chrome が必要です。

```bash
MDV_TEST_BROWSER='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' cargo test --lib chromium_snapshot -- --include-ignored
```

配布は同一ソースSHAの macOS/Windows成果物をCIで生成し、単一publisherが `bin/mdv` と `bin/mdv.exe` をまとめて更新します。mainが進んだ場合は古い成果物を新しいソースへ載せず、次のrunへ委ねます。PRではダウンロード用artifactまで生成し、mainのバイナリは変更しません。
