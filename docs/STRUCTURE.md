# STRUCTURE: `mdv`

| Item | Value |
| --- | --- |
| Version | 1.0 |
| Status | Active |
| Product | `mdv` |
| Document Basis | Repository audit on 2026-03-29 |

## 1. ルート構成

主要なトップレベルは次の通り。

```text
.
├── .agents/
├── .cargo/
├── .github/
├── docs/
├── examples/
├── scripts/
├── src/
├── tests/
├── Cargo.toml
├── Makefile
├── README.md
└── rust-toolchain.toml
```

## 2. ディレクトリ別の役割

| Path | Role |
| --- | --- |
| `docs/` | 製品・技術・構造ドキュメント、および初期設計メモ |
| `.agents/memory/` | エージェント用の task memory と lessons |
| `.github/workflows/` | CI、rolling main channel、release workflow |
| `scripts/` | 配布用 install script |
| `examples/` | rich Markdown fixture とサンプル画像 |
| `src/` | 本体実装 |
| `tests/` | unit / integration / e2e テスト |

## 3. `src/` の構造

```text
src/
├── app/
├── cli/
├── core/
├── io/
│   └── webkit_snapshot/
├── render/
│   ├── assets/
│   └── github_html/
├── support/
├── ui/
│   └── terminal/
├── lib.rs
└── main.rs
```

### 3.1 Entry Points

| File | Role |
| --- | --- |
| `src/main.rs` | プロセス entrypoint。CLI parse と `app::run()` 呼び出しのみ |
| `src/lib.rs` | 公開 module 定義と最小限の smoke test |

### 3.2 `src/cli/`

| File | Role |
| --- | --- |
| `src/cli/mod.rs` | `parse()` と startup message 補助 |
| `src/cli/args.rs` | `MdvArgs`、`Theme`、CLI schema |

### 3.3 `src/app/`

| File | Role |
| --- | --- |
| `src/app/mod.rs` | 起動 orchestration、TTY 判定、interactive / headless 分岐 |

`app` は実質的な composition root で、`cli`、`core`、`io`、`render`、`ui` を束ねる。

### 3.4 `src/core/`

| File | Role |
| --- | --- |
| `src/core/config.rs` | `AppConfig`、`MermaidMode` |
| `src/core/document.rs` | `Document`、`BlockKind`、inline style などの中間表現 |
| `src/core/layout.rs` | viewport と visible range 計算 |
| `src/core/theme.rs` | TUI 用 color token |
| `src/core/diff.rs` | `Document` 間の block diff |

ここは製品ロジックの基礎型を置く層で、renderer と UI の双方から参照される。

### 3.5 `src/render/`

| File | Role |
| --- | --- |
| `src/render/markdown.rs` | `comrak` AST を `Document` へ正規化 |
| `src/render/markdown_pipeline.rs` | GFM options の共通定義 |
| `src/render/text.rs` | headless renderer と structured text layout |
| `src/render/svg.rs` | SVG 描画補助 |
| `src/render/mod.rs` | render module 公開面 |

#### `src/render/github_html/`

| File | Role |
| --- | --- |
| `src/render/github_html/mod.rs` | GitHub HTML 生成の入口 |
| `src/render/github_html/styles.rs` | GitHub CSS と font-face 埋め込み |
| `src/render/github_html/mermaid.rs` | Mermaid code block の事前置換 |
| `src/render/github_html/postprocess.rs` | code block 整形、alert icon 注入、token retint |
| `src/render/github_html/tests.rs` | render pipeline 周辺の局所テスト |

#### `src/render/assets/`

- GitHub light / dark CSS
- 埋め込みフォント

### 3.6 `src/io/`

| File | Role |
| --- | --- |
| `src/io/fs.rs` | ファイル読み込みと更新時刻取得 |
| `src/io/browser.rs` | 既定ブラウザ起動 |
| `src/io/image_decoder.rs` | ローカル画像読み込み、PNG 化、寸法取得 |
| `src/io/kitty_graphics.rs` | Kitty graphics escape sequence encoder |
| `src/io/mermaid_cli.rs` | Mermaid CLI 実行と cache |
| `src/io/mod.rs` | IO module 公開面 |

#### `src/io/webkit_snapshot/`

| File | Role |
| --- | --- |
| `src/io/webkit_snapshot/mod.rs` | HTML snapshot 実行の公開 API |
| `src/io/webkit_snapshot/diagnostics.rs` | snapshot report の型 |
| `src/io/webkit_snapshot/paths.rs` | read-access root と temp workspace 計算 |
| `src/io/webkit_snapshot/script.rs` | Swift helper script の埋め込み |
| `src/io/webkit_snapshot/tests.rs` | path / diagnostics 周辺テスト |

### 3.7 `src/ui/`

| File | Role |
| --- | --- |
| `src/ui/mod.rs` | UI module 公開面 |
| `src/ui/page_graphics.rs` | full-page PNG の viewport slicing |

#### `src/ui/terminal/`

| File | Role |
| --- | --- |
| `src/ui/terminal/mod.rs` | `TerminalViewer`、event loop、draw、reload、status line |
| `src/ui/terminal/layout.rs` | article width、cell metrics、graphic page 構築 |
| `src/ui/terminal/graphics.rs` | graphics placement と visible command 収集 |
| `src/ui/terminal/highlight.rs` | code highlight の terminal 表現 |
| `src/ui/terminal/tests.rs` | viewer と placement 周辺テスト |

### 3.8 `src/support/`

| File | Role |
| --- | --- |
| `src/support/tracing.rs` | tracing 初期化 |

## 4. 実行フローに沿った責務分離

### 4.1 Startup

1. `src/main.rs`
2. `src/cli/args.rs`
3. `src/app/mod.rs`

### 4.2 Parse and Normalize

1. `src/io/fs.rs`
2. `src/render/markdown.rs`
3. `src/core/document.rs`

### 4.3 Interactive Graphic Rendering

1. `src/render/github_html/mod.rs`
2. `src/io/webkit_snapshot/mod.rs`
3. `src/ui/page_graphics.rs`
4. `src/ui/terminal/mod.rs`
5. `src/io/kitty_graphics.rs`

### 4.4 Headless Rendering

1. `src/render/text.rs`
2. `src/io/image_decoder.rs`
3. `src/io/mermaid_cli.rs`

## 5. `tests/` の構造

```text
tests/
├── e2e.rs
├── integration.rs
├── unit.rs
├── e2e/
├── integration/
└── unit/
    └── render_pipeline/
```

### 5.1 `tests/unit/`

責務:

- parser 正規化
- layout 計算
- image decoder
- Mermaid CLI adapter
- SVG / terminal render
- GitHub HTML / WebKit snapshot diagnostics

### 5.2 `tests/integration/`

責務:

- CLI の observable contract
- help output
- headless render feature coverage

### 5.3 `tests/e2e/`

責務:

- richer fixture を使った end-to-end 実行
- local image と fake Mermaid renderer を含む実行シナリオ

## 6. 補助ファイル

| File | Role |
| --- | --- |
| `README.md` | エンドユーザー向け紹介と使い方 |
| `Cargo.toml` | crate metadata、依存、lint policy |
| `Cargo.lock` | 依存 lock |
| `rust-toolchain.toml` | Rust version 固定 |
| `.cargo/config.toml` | cargo alias |
| `Makefile` | build / test / lint の短縮コマンド |
| `release-please-config.json` | stable release の version / changelog 設定 |
| `.release-please-manifest.json` | release-please が追跡する現在 version |
| `scripts/install.sh` | rolling `main` channel を既定にしたインストール |
| `.github/workflows/ci.yml` | CI と release-assets 検証 |
| `.github/workflows/main-channel.yml` | rolling `main` channel の配布物 publish |
| `.github/workflows/release-assets.yml` | reusable な multi-target release asset build |
| `.github/workflows/release.yml` | `release-please` と stable asset publish |

## 7. 設計上の見どころ

この repo の構造で重要なのは、`render` と `io` が単なる補助層ではなく、製品価値の中心にいる点である。

- `core` は共通モデルを提供する
- `render` は Markdown と GitHub-like fidelity を担う
- `io` は browser、Mermaid、WebKit、Kitty protocol という外界接続を担う
- `ui` は terminal の対話体験を担う
- `app` はそれらを組み立てる

strict clean architecture よりも、viewer 製品としての描画パイプラインを前面に出した構成になっている。

## 8. 今後構造化しやすい領域

- `DocumentMeta.links.first()` に依存する link UX を stateful navigation に分離する
- interactive fallback を `ui::terminal` 内で strategy として切り出す
- platform-specific snapshot backend を `io::webkit_snapshot` 以外へ拡張できるよう抽象化する
