# TECH: `mdv`

| Item | Value |
| --- | --- |
| Version | 1.0 |
| Status | Active |
| Product | `mdv` |
| Language | Rust 2024 |
| Toolchain | Rust `1.92.0` |
| Document Basis | Repository audit on 2026-03-29 |

## 1. 技術概要

`mdv` は Rust 製の single-binary CLI で、Markdown を一度 `Document` に正規化したうえで、2 つの出力経路に分岐する。

- 対話モード: GitHub HTML 風に整形したページを PNG に snapshot し、Kitty graphics protocol で terminal に表示する
- 非対話モード: `Document` から plain-text rendering を生成して stdout に出す

エントリポイントはシンプルで、`src/main.rs` が `mdv::cli::parse()` と `mdv::app::run()` を呼ぶだけになっている。

## 2. 主要依存

| Crate | 用途 |
| --- | --- |
| `clap` | CLI 引数解析 |
| `anyhow` / `thiserror` | エラー文脈の付与 |
| `comrak` | GFM ベースの Markdown parse と HTML 出力 |
| `crossterm` | TTY 制御、キー入力、alternate screen |
| `image` | 画像 decode、PNG 変換、寸法取得 |
| `resvg` | SVG rasterize 関連 |
| `syntect` | code block syntax highlight |
| `serde` / `serde_json` | snapshot diagnostics の JSON 処理 |
| `tracing` / `tracing-subscriber` | 起動、描画、外部コマンド周辺の trace |

## 3. ランタイムモード

### 3.1 Headless Mode

条件:

- `stdout` または `stdin` が TTY ではない

挙動:

- `render::text::render_plain_text()` を使う
- alternate screen は使わない
- image / Mermaid はメタ情報ベースの degrade 表現を返す

### 3.2 Interactive Mode

条件:

- `stdout` と `stdin` がともに TTY
- terminal 判定が Ghostty または Kitty

挙動:

- `TerminalViewer::try_new()` を生成
- raw mode と alternate screen に入る
- graphic page を描画し、キー入力ループで scroll / reload / open link を処理する

制約:

- 現行実装では graphic page 生成が前提で、失敗時の runtime fallback はない
- `webkit_snapshot` は非 macOS で未対応のため、interactive rich path は実質 macOS 前提

## 4. データフロー

### 4.1 Parse Pipeline

1. `app::run()` がファイル本文を読む
2. `render::markdown::parse_document()` が `comrak` AST を走査する
3. AST を `core::document::Document` と `BlockKind` 群へ正規化する
4. `DocumentMeta` に title、links、source length を格納する

`Document` はこのアプリの共通中間表現であり、headless と interactive の双方がこれを起点にする。

### 4.2 Interactive Render Pipeline

1. `render::github_html::build_github_html()` が GitHub 風 HTML を生成する
2. Mermaid code fence は事前に `replace_mermaid_code_blocks()` で SVG か fallback HTML に差し替える
3. `io::webkit_snapshot::render_html_to_png()` が HTML を PNG と diagnostics report に変換する
4. `ui::page_graphics::build_graphic_page()` が PNG の display size と viewport slice 情報を作る
5. `ui::terminal` が Kitty graphics command を発行して terminal に置く

### 4.3 Headless Render Pipeline

1. `render_plain_text()` が `Document` を block ごとに走査する
2. block を plain-text へ整形する
3. image / Mermaid / footnote / table を可読な degrade 表現に変換する
4. stdout に UTF-8 で出力する

## 5. モジュール責務

| Module | 責務 |
| --- | --- |
| `cli` | `MdvArgs` 定義、theme enum、引数 parse |
| `app` | 起動 orchestration、TTY 判定、render mode 分岐 |
| `core` | `Document`、`BlockKind`、テーマ token、layout 補助、diff |
| `render::markdown` | GFM parse と `Document` 正規化 |
| `render::text` | headless 表示と structured text layout |
| `render::github_html` | GitHub CSS ベースの HTML 生成と Mermaid 埋め込み |
| `render::svg` | SVG ベースの描画補助と関連テスト |
| `io::fs` | ドキュメントの読み込みと更新時刻取得 |
| `io::image_decoder` | ローカル画像の解決と PNG 化 |
| `io::mermaid_cli` | `mmdc` / `npx @mermaid-js/mermaid-cli` 呼び出しと cache |
| `io::webkit_snapshot` | Swift helper を使った HTML snapshot |
| `io::browser` | 既定ブラウザ起動 |
| `io::kitty_graphics` | Kitty graphics escape sequence 生成 |
| `ui::terminal` | イベントループ、status line、graphics placement |
| `ui::page_graphics` | full-page PNG の viewport 切り出し |

## 6. 外部依存とシステム前提

### 6.1 Mermaid

Mermaid は `MermaidCliRenderer::from_env()` で次の順に探索する。

1. `MDV_MERMAID_CMD`
2. `mmdc`
3. `npx -y @mermaid-js/mermaid-cli`

レンダリング結果は cache directory に保存される。

### 6.2 WebKit Snapshot

HTML snapshot は Swift helper 経由で `WKWebView` を使っている。これは `src/io/webkit_snapshot/mod.rs` と同配下の script / diagnostics / paths モジュールで構成される。

生成物:

- `snapshot.png`
- `snapshot-report.json`

diagnostics には以下が含まれる。

- image readiness
- Mermaid metrics
- typography metrics
- asset failure diagnostics

### 6.3 Browser Open

`io::browser` は OS ごとの既定ブラウザ起動コマンドを選ぶ。対話モードで `o` を押したときのみ使用する。

## 7. テーマと見た目

テーマは `Theme::Light` と `Theme::Dark` の 2 種類だけである。

interactive path では次を組み合わせる。

- GitHub の light / dark CSS アセット
- 埋め込み `Mona Sans VF` と `Monaspace Neon Var`
- `syntect` の syntax highlight theme

headless path では `ThemeTokens` は持つが、最終出力は plain text 中心で、interactive ほど theme 差は大きくない。

## 8. ビルド・検証・配布

### 8.1 ローカル開発

- `cargo build --release`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features`

`Makefile` でも同等の entrypoint が定義されている。

### 8.2 CI

GitHub Actions の `ci.yml` は次を実行する。

- format check
- clippy
- test

Rust toolchain は `1.92.0` に固定されている。

### 8.3 Release

`release.yml` は tag push を契機に次を行う。

- tag と `Cargo.toml` / `CHANGELOG.md` の整合を検証
- Linux x86_64 / arm64 ビルド
- macOS x86_64 / arm64 ビルド
- `tar.gz` アーカイブ作成
- packaged archive の extract / `mdv --help` smoke check
- `SHA256SUMS` 生成
- GitHub Release 公開

インストーラは `scripts/install.sh` が GitHub Releases から適切なアーカイブを取得する。

## 9. テスト戦略

テストは 3 層に分かれている。

- `tests/unit`: parser、renderer、snapshot、terminal 補助、Mermaid adapter などの単体確認
- `tests/integration`: CLI と出力契約の確認
- `tests/e2e`: rich fixture や asset 解決を含む実行経路の確認

特に重要なのは次。

- headless rendering contract
- GitHub HTML typography fidelity
- local image rendering
- Mermaid diagnostics
- terminal scroll / graphics placement

## 10. 既知の技術的制約

1. interactive rich rendering が macOS WebKit に依存している
2. `TerminalViewer::try_new()` に graphic page failure 時の fallback がない
3. link open は first link 固定で、link focus state を持たない
4. terminal 判定は Ghostty / Kitty に限定される
5. README の platform statement と現実装の runtime contract に差がある

## 11. 改善候補

- Linux でも動く snapshot backend か structured TUI fallback を用意する
- `Document` の `revision` や `SourceSpan` を reload diff に活用する
- interactive path の fallback を `render::text::render_document()` に戻す
- 複数リンクナビゲーションや検索 UI を足す場合は `DocumentMeta` と UI state を拡張する
