# BMS Resource Toolbox — AGENTS.md

## Lint 配置

### 以下 Lint 禁止使用 `#[allow(...)]` 或 `#[expect(...)]` 等方式压制

- `dead_code`
- `missing_docs`
- `clippy::missing_errors_doc`
- `clippy::missing_panics_doc`

## 文件系统 API

- 全项目禁止使用 `std::fs`。所有文件系统操作必须使用 `tokio::fs`（引入方式：`use tokio::fs`，调用方式：`fs::xxx`）。
- 例外：`tokio::task::spawn_blocking` 闭包内可使用 `std::fs`（此乃 spawn_blocking 作为异步友好模式的标准实践），但文件打开须在闭包外通过 `tokio::fs::File::open().await?.into_std().await` 完成（注意 tokio 1.52+ 中 `into_std()` 是 async 方法，需 await）。

## CI 检查

提交前必须运行在参考文件中的所有检查（运行时不需要 `--verbose` 参数）。
任一检查失败，不得提交。

### 参考文件

- [rust.yml](.github/workflows/rust.yml)
- [cargo-deny.yml](.github/workflows/cargo-deny.yml)

