# BMS Resource Toolbox — AGENTS.md

## Lint 配置

Cargo.toml 和 `clippy.toml` 已包含所有 lint 规则（`dead_code`/`missing_docs`/`unwrap_used`/`indexing_slicing` = deny，`std::fs` 禁用等）。工具链自动强制执行。

AGENTS.md 不再重复这些规则。参见 `clippy.toml` 中 `disallowed-methods` 和 `disallowed-macros` 的完整列表。

### 例外说明

`spawn_blocking` 闭包内可用 `std::fs`，但文件打开须闭包外完成：`tokio::fs::File::open().await?.into_std().await`（tokio 1.52+ `into_std()` 为 async）。该例外已在 `extract.rs` 中以 `#[expect(clippy::disallowed_methods)]` 标注。

## Crate 结构

Cargo workspace，3 crate：

| Crate | 路径 | 说明 |
|-------|------|------|
| `bms_res_tb_domain` | `crates/domain/` | 业务逻辑（BMS 解析、文件整理、同步、转换编排） |
| `bms_res_tb_infra` | `crates/infra/` | 基础设施（文件系统、媒体转换、压缩解压） |
| `bms_res_tb_cli` | `crates/cli/` | CLI 入口（clap 定义 + dispatch） |

依赖方向：`bms_res_tb_cli` → `bms_res_tb_domain` → `bms_res_tb_infra`

### 引用规则

- domain → infra：`use bms_res_tb_infra::xxx;`
- cli → domain：`use bms_res_tb_domain::xxx;`

## CI 检查

提交前必须运行（无 `--verbose`），任一失败不得提交：

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace
cargo fmt --check
cargo doc --workspace --no-deps --all-features
```

### 参考文件

- [rust.yml](.github/workflows/rust.yml)
- [cargo-deny.yml](.github/workflows/cargo-deny.yml)

> `cargo doc` 等效 CI 中的 `RUSTDOCFLAGS="-D warnings"`，doc 警告也阻塞提交。
