# BMS Resource Toolbox

[![CI](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/rust.yml/badge.svg)](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/MiyakoMeow/bms-resource-toolbox/graph/badge.svg)](https://codecov.io/gh/MiyakoMeow/bms-resource-toolbox)
[![Cargo Deny](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/cargo-deny.yml/badge.svg)](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/cargo-deny.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

BMS（Beatmania）谱面资源管理工具箱。Rust 实现，提供 CLI 接口 `bms-res-tb`。

## 功能

### BMS 根目录

| 命令 | 说明 |
|------|------|
| `set-name-by-bms` | 按 BMS 信息设置文件夹名为「标题 [艺术家]」 |
| `append-name-by-bms` | 追加文件夹名「标题 [艺术家]」 |
| `append-artist-name-by-bms` | 追加艺术家名 |
| `copy-numbered-workdir-names` | 克隆带编号的文件夹名 |
| `scan-folder-similar-folders` | 扫描相似文件夹名 |
| `undo-set-name` | 撤销重命名 |
| `remove-zero-sized-media-files` | 清理空尺寸媒体文件和临时文件 |

### BMS 大包目录

| 命令 | 说明 |
|------|------|
| `split-folders-with-first-char` | 按首字符（A-Z、平假名、片假名、汉字等）分类文件夹 |
| `move-works-in-pack` | 跨目录移动/合并作品 |
| `move-out-works` | 移出一层目录 |
| `move-works-with-same-name` | 合并文件名相似的子文件夹 |
| `merge-split-folders` | 合并被拆分的文件夹 |
| `undo-split-pack` | 撤销首字符分类 |

### BMS 媒体

| 命令 | 说明 |
|------|------|
| `transfer-audio` | 音频格式转换（WAV/FLAC/OGG 互转） |
| `transfer-video` | 视频格式转换（MP4 → AVI/WMV/MPEG） |

### BMS 原文件

| 命令 | 说明 |
|------|------|
| `unzip-numeric-to-bms-folder` | 解压编号文件至编号目录 |
| `unzip-with-name-to-bms-folder` | 按原名解压至对应目录 |
| `set-file-num` | 为文件添加数字编号前缀 |

### BMS 大包脚本

| 命令 | 说明 |
|------|------|
| `pack-setup-rawpack-to-hq` | 初始打包：原包 → HQ 版 |
| `pack-update-rawpack-to-hq` | 更新打包：原包 → HQ 版（差分包） |
| `pack-raw-to-hq` | 已缓存原包 → HQ 版大包 |
| `pack-hq-to-lq` | HQ 版 → LR2 兼容 LQ 版大包 |

### BMS 活动

| 命令 | 说明 |
|------|------|
| `jump-to-work-info` | 跳转至 BMS 活动作品信息页 |
| `check-num-folder` | 检查编号对应文件夹是否存在 |
| `create-num-folders` | 创建编号空文件夹 |
| `generate-work-info-table` | 生成活动作品 xlsx 表格 |

## 安装

### 依赖

- [Rust](https://rustup.rs) 1.85+
- [ffmpeg](https://ffmpeg.org)
- [flac](https://xiph.org/flac/)
- [oggenc](https://www.rarewares.org/ogg-oggenc.php)

### 从源码编译

```bash
git clone https://github.com/MiyakoMeow/bms-resource-toolbox
cd bms-resource-toolbox
cargo build --release
./target/release/bms-res-tb --help
```

或直接安装：

```bash
cargo install --git https://github.com/MiyakoMeow/bms-resource-toolbox
```

## 使用

```bash
# 查看帮助
bms-res-tb --help

# 按 BMS 信息重命名文件夹
bms-res-tb set-name-by-bms -p /path/to/bms/root

# 音频转换（WAV → FLAC）
bms-res-tb transfer-audio -p /path/to/bms/root -m 0

# 生成 HQ 版大包
bms-res-tb pack-raw-to-hq -p /path/to/root
```

## 相关项目

- [bms-table-rs](https://github.com/MiyakoMeow/bms-table-rs) — BMS 难度表抓取与解析库（Rust）
- [bms-resource-scripts](https://gitee.com/MiyakoMeow/bms-resource-scripts) — 本工具箱的原始 Python 版本

## 许可证

Apache License 2.0
