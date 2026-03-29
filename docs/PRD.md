# PRD: `mdv`

| Item | Value |
| --- | --- |
| Version | 1.0 |
| Status | Active |
| Product | `mdv` |
| Type | Rust 製 CLI Markdown Viewer |
| Repository Version | `0.1.0` |
| Primary Interface | Terminal CLI |
| Document Basis | Repository audit on 2026-03-29 |

## 1. 目的

`mdv` は、Markdown 文書を browser や editor preview に切り替えず、terminal 内で高い可読性のまま読むための single-binary viewer である。

このリポジトリから確認できる現在の製品方針は次の 2 本柱に集約される。

- 対話モードでは、GitHub 風の見た目をできるだけそのまま terminal に持ち込む
- 非対話モードでは、パイプや CI でも破綻しない plain-text rendering を返す

## 2. 解く課題

terminal 常用者が Markdown を読むとき、次の摩擦が発生しやすい。

- README、設計書、仕様書の確認のためだけに browser や IDE preview を開く必要がある
- 既存の terminal viewer では table、image、callout、Mermaid、syntax highlight の fidelity が落ちやすい
- CI や pipe では rich viewer が使えず、最低限読める代替表現が必要になる

`mdv` はこの摩擦を減らし、Markdown の閲覧体験を terminal workflow に戻すことを狙う。

## 3. 対象ユーザー

### Primary User

- Ghostty または Kitty を日常的に使うソフトウェアエンジニア
- README、設計 docs、仕様書を Markdown で読む人
- terminal から離れない作業フローを重視する人

### Secondary User

- CLI 中心でレビューや調査を進める技術者
- CI や shell pipeline で Markdown を読みやすく整形したい人
- GitHub に近い Markdown 表示をローカルでも欲しい人

## 4. 主要ユースケース

| # | Use Case | Current Outcome |
| --- | --- | --- |
| 1 | `mdv README.md` を実行してドキュメントを読む | 対話 TTY では graphic page 表示、非対話では plain-text 表示 |
| 2 | rich fixture のような画像付き設計書を確認する | ローカル画像を解決し、表示可能なら描画する |
| 3 | Mermaid を含む仕様書を読む | renderer が使えれば表示、使えなければ unavailable 表示に degrade する |
| 4 | 文書を編集しながら確認する | `--watch` で変更監視し再読み込みする |
| 5 | shell pipeline で Markdown を読む | stdout 非 TTY 時にヘッドレス整形結果を出力する |

## 5. 現在の製品スコープ

### 5.1 入力

- ローカル Markdown ファイルのパス
- 主に `.md` / `.markdown` を想定

### 5.2 CLI オプション

- `--watch`
- `--theme system|light|dark`
- `--no-mermaid`
- `update`

### 5.3 Markdown 表示対象

実装上、`Document` 中間表現に正規化されているブロックは以下。

- Heading
- Paragraph
- List
- BlockQuote
- Callout
- CodeFence
- Table
- Image
- Mermaid
- Rule
- Footnote

加えて inline では以下を保持する。

- bold
- italic
- inline code
- link text と link destination

### 5.4 対話モードの体験

対話モードは `stdout` と `stdin` の両方が TTY で、かつ terminal 判定が Ghostty または Kitty のときにだけ起動する。

現行実装から確認できる操作は以下。

- `j` / `Down`: 1 行下へ
- `k` / `Up`: 1 行上へ
- `PageDown`
- `PageUp`
- `g`: 先頭へ
- `G`: 末尾へ
- `r`: 再読み込み
- `o`: 文書内の最初のリンクを既定ブラウザで開く
- `q`: 終了

### 5.5 非対話モードの体験

非対話モードでは plain-text renderer を使い、次のように degrade する。

- link は `label <url>` 形式
- image は解決できれば寸法つきメタ表示、失敗時は missing 表示
- Mermaid は rendered / disabled / unavailable のいずれかで表現
- table、footnote、callout もテキストとして読める形に整形

## 6. 製品要件

### 6.1 Functional Requirements

1. Markdown ファイルを一度 parse して内部 `Document` に正規化できること
2. 対話 TTY では terminal viewer を起動できること
3. 非対話時は viewer ではなく plain-text を返せること
4. ローカル画像を相対パス解決できること
5. Mermaid renderer が無い場合でもプロセス全体は継続すること
6. `--watch` でファイル更新を検知し、再読み込みできること
7. `system` を既定に、`light` / `dark` を明示指定で切り替えられること
8. 最低限の keyboard navigation を持つこと
9. `update` で既定の rolling `main` channel、または指定 channel の最新配布物を現在の `mdv` 実行ファイルへ上書きインストールできること

### 6.2 Non-Functional Requirements

1. 単一バイナリで配布できること
2. `unsafe` を禁止し、Clippy 警告ゼロを維持すること
3. CI で `fmt`、`clippy -D warnings`、`cargo test` を通せること
4. rich rendering が失敗しても、少なくとも原因が分かるエラーまたは診断が返ること

## 7. 非ゴール

この repo からは、次は現時点で製品スコープ外または未実装と判断できる。

- Markdown 編集機能
- 検索 UI
- TOC パネル
- 複数リンクのフォーカス移動
- tmux / zellij の正式サポート
- remote image fetch
- arbitrary HTML 実行
- Windows 対応

## 8. 現行実装ベースの制約

README の配布ターゲットは macOS / Linux だが、現行の interactive rendering は GitHub HTML を WebKit で PNG 化する経路に依存している。`src/io/webkit_snapshot` は非 macOS で `bail!` するため、現在の rich interactive path は実質 macOS 前提である。

また、`o` キーは「現在フォーカス中のリンク」ではなく、`DocumentMeta.links.first()` を開く。将来の UX 文言や仕様を整えるなら、ここは仕様として固定するか、本当にフォーカス移動を実装するかを決める必要がある。

## 9. 成功指標

コードベースから妥当と判断できる短期指標は以下。

- rich fixture を使った integration / e2e テストが green であること
- README、examples 配下の文書が `mdv` で読み切れること
- Mermaid や local image の失敗時に silent failure にならないこと
- 対話モードと非対話モードで最低限の閲覧品質を維持できること

## 10. 次に詰めるべき項目

- Linux での rich interactive path をどう成立させるか
- graphic page 失敗時の runtime fallback を復活させるか
- link navigation を複数リンク対応に広げるか
- search / TOC など viewer UX の優先順位をどう置くか
