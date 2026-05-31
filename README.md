# BMS Resource Toolbox

[![CI](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/rust.yml/badge.svg)](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/MiyakoMeow/bms-resource-toolbox/graph/badge.svg)](https://codecov.io/gh/MiyakoMeow/bms-resource-toolbox)
[![Cargo Deny](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/cargo-deny.yml/badge.svg)](https://github.com/MiyakoMeow/bms-resource-toolbox/actions/workflows/cargo-deny.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

BMS（Beatmania）谱面资源管理工具箱。Rust 实现，提供 CLI 接口 `bms-res-tb`。

命令采用 `bms-res-tb <组> <子命令> [参数]` 结构。全局标志 `-y`（`--yes`）可跳过所有确认提示。

## 功能

### BMS 根目录（`folder`）

| 命令 | 说明 |
|------|------|
| `folder set-name --mode set-all` | 设置为「标题 [艺术家]」 |
| `folder set-name --mode set-title` | 仅设置为标题 |
| `folder set-name --mode set-artist` | 仅设置为艺术家 |
| `folder set-name --mode append-all` | 追加「标题 [艺术家]」（仅纯数字目录） |
| `folder set-name --mode append-title` | 追加标题 |
| `folder set-name --mode append-artist` | 追加艺术家名 |
| `folder undo` | 撤销重命名 |
| `folder copy-numbered` | 克隆带编号的文件夹名 |
| `folder scan-similar` | 扫描相似文件夹名 |
| `folder remove-zero-media` | 清理空尺寸媒体文件和临时文件 |

### BMS 大包目录（`pack`）

| 命令 | 说明 |
|------|------|
| `pack split` | 按首字符（A-Z、平假名、片假名、汉字等）分类文件夹 |
| `pack move-in` | 跨目录移动/合并作品 |
| `pack move-out` | 移出一层目录 |
| `pack merge-same-name` | 合并文件名相似的子文件夹 |
| `pack merge-to-siblings` | 合并文件名相似的子文件夹到各平级目录 |
| `pack merge-split` | 合并被拆分的文件夹 |
| `pack undo-split` | 撤销首字符分类 |

### BMS 媒体（`media`）

| 命令 | 说明 |
|------|------|
| `media audio --mode wav-to-flac` | WAV → FLAC |
| `media audio --mode flac-to-ogg` | FLAC → OGG（压缩） |
| `media audio --mode wav-to-ogg` | WAV → OGG（压缩） |
| `media audio --mode flac-to-wav` | FLAC → WAV（反向） |
| `media video --format avi` | MP4 → AVI |
| `media video --format wmv2` | MP4 → WMV |
| `media video --format mpeg1` | MP4 → MPEG |
| `media remove-unneed --preset oraja` | ORAJA 规则：mp4>avi>wmv, flac/wav>ogg, flac>wav, mpg>wmv |
| `media remove-unneed --preset wav-flac` | 有 WAV 时移除对应 FLAC |
| `media remove-unneed --preset mpg-wmv` | 有 MPG 时移除对应 WMV |

### BMS 原文件（`source`）

| 命令 | 说明 |
|------|------|
| `source unzip-numeric` | 解压编号文件至编号目录 |
| `source unzip-named` | 按原名解压至对应目录 |
| `source set-number` | 为文件添加数字编号前缀 |

### BMS 大包脚本（`pack`）

| 命令 | 说明 |
|------|------|
| `pack raw-hq-setup` | 初始打包：原包 → HQ 版 |
| `pack raw-hq-update` | 更新打包：原包 → HQ 版（差分包） |
| `pack raw-to-hq` | 已缓存原包 → HQ 版大包 |
| `pack hq-to-lq` | HQ 版 → LR2 兼容 LQ 版大包 |

### BMS 活动（`event`）

| 命令 | 说明 |
|------|------|
| `event jump` | 跳转至 BMS 活动作品信息页 |
| `event check-folders` | 检查编号对应文件夹是否存在 |
| `event create-folders` | 创建编号空文件夹 |
| `event generate-table` | 生成活动作品 xlsx 表格 |

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

# 按 BMS 信息重命名文件夹（设置为「标题 [艺术家]」）
bms-res-tb folder set-name --mode set-all -p /path/to/bms/root

# 仅追加艺术家名
bms-res-tb folder set-name --mode append-artist -p /path/to/bms/root

# 音频转换（WAV → FLAC）
bms-res-tb media audio --mode wav-to-flac -p /path/to/bms/root

# 音频转换（FLAC → OGG）
bms-res-tb media audio --mode flac-to-ogg -p /path/to/bms/root

# 视频转换（MP4 → AVI）
bms-res-tb media video --format avi -p /path/to/bms/root

# 清理冗余媒体（ORAJA 规则）
bms-res-tb media remove-unneed --preset oraja -p /path/to/bms/root

# 生成 HQ 版大包
bms-res-tb pack raw-to-hq -p /path/to/root

# 跳过确认提示
bms-res-tb -y pack merge-same-name --from /src --to /dst
```

## 相关项目

- [bms-table-rs](https://github.com/MiyakoMeow/bms-table-rs) — BMS 难度表抓取与解析库（Rust）
- [bms-resource-scripts](https://gitee.com/MiyakoMeow/bms-resource-scripts) — 本工具箱的原始 Python 版本

## 许可证

Apache License 2.0
