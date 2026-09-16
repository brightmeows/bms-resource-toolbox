# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bms-resource-toolbox/releases/tag/bms_res_tb_cli-v0.1.0) - 2026-09-16

### Added

- *(cli)* interactive menu mode

### Fixed

- *(cli)* select rename mode before prompting for path
- *(cli)* remove help message line from menu prompt
- *(cli)* exit on any inquire error (incl. Ctrl+C), single exit line
- *(cli)* add "0: 退出" option and ensure Ctrl+C exits menu
- *(cli)* group menu by module with tens-boundary numbering
- *(cli)* tilde expansion and stdout output for interactive menu

### Other

- *(cli)* merge set-name and append-name into single rename command
- *(deps)* add pre-commit hooks for clippy and fmt
- fmt
- 拆分单 crate 为 workspace（domain/infra/cli） ([#22](https://github.com/brightmeows/bms-resource-toolbox/pull/22))
