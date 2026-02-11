# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# ユーザーIDを @表示名 形式で表示する

## Context
現在メッセージのユーザー欄にSlack APIから取得したユーザーID（例: `U081R4ZS5E2`）がそのまま表示されている。`users.info` APIでユーザー名を解決し、`@kanta` のようなメンション形式で表示するようにする。

## 変更内容

### 1. `src/slack_api.rs` — ユーザー情報取得関数を追加
- `fetch_user_info(user_id: &str, token: &str...

### Prompt 2

権限が足りなくて取得できていなさそう。extract_messagesでやっているように、足りていない権限があればエラーを出すようにしたい。

### Prompt 3

[Request interrupted by user for tool use]

