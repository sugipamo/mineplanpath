# plan-path

mineplanを繰り返し参照し、2ノード間で`edge_name`を切り替える回数が少ない経路を1つ返す外部解析MCPです。

厳密な全グラフ解析は行いません。mineplanの`focus(include_connections=true)`を最大50回呼び出し、そのとき観測できた部分グラフ上で0-1 BFSを行います。

## MCPツール

公開するツールは`find_path`だけです。

```json
{
  "name": "find_path",
  "arguments": {
    "from": "要件を整理する",
    "to": "リリースする"
  }
}
```

経路が見つかった場合：

```json
{
  "from": "要件を整理する",
  "to": "リリースする",
  "turns": 2,
  "tasks": [
    {
      "edge_name": "design",
      "sequence": ["要件を整理する", "設計を決める"]
    },
    {
      "edge_name": "implementation",
      "sequence": ["設計を決める", "実装する", "テストする"]
    },
    {
      "edge_name": "release",
      "sequence": ["テストする", "リリースする"]
    }
  ]
}
```

`turns`は連続するTask間で`edge_name`を切り替えた回数です。同じ`edge_name`が連続する区間は1つのTaskとしてまとめ、実際に通るノードを`sequence`へ順番どおり格納します。隣接するTaskでは、乗り換え地点のノードが前のTaskの末尾と次のTaskの先頭の両方に現れます。mineplanの辺は`previous / next`のどちら向きにも移動できます。

経路が見つからなかった場合：

```json
{
  "from": "要件を整理する",
  "to": "リリースする",
  "found": false
}
```

これは到達不能の証明ではなく、今回観測した範囲では見つからなかったことを示します。

## 起動

先にmineplanを起動してからplan-pathを起動します。

```sh
cargo run
```

- plan-path MCP: `http://127.0.0.1:3100/mcp`
- 接続先mineplan: `http://127.0.0.1:3000/mcp`

設定は環境変数で変更できます。

```sh
MINEPLAN_MCP_URL=http://127.0.0.1:3000/mcp \
PLAN_PATH_HTTP_PORT=3100 \
PLAN_PATH_MAX_FOCUS_CALLS=50 \
PLAN_PATH_FOCUS_LIMIT=50 \
cargo run
```

利用可能な設定は`cargo run -- --help`でも確認できます。

バイナリのバージョンは`--version`または`-V`で確認できます。リリースバイナリにはGitタグを埋め込みます。

```sh
plan-path -V
# plan-path v0.1.0
```

## 探索モデル

探索状態は`(node, current_edge_name)`です。

- 同じ`edge_name`の辺へ進む: コスト0
- 違う`edge_name`の辺へ進む: コスト1
- 始点で最初の`edge_name`を選ぶ: コスト0
- 始点と終点が同じ: `turns: 0`, `tasks: []`

各focusで得た辺は、永続的な`edge_id`で重複排除します。経路を発見した時点で、観測済みグラフ内の低コスト経路を返します。

## 開発

```sh
cargo fmt --check
cargo test
```
