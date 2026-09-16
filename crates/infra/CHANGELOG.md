# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/brightmeows/bms-resource-toolbox/releases/tag/bms_res_tb_infra-v0.1.0) - 2026-09-16

### Added

- *(cli)* interactive menu mode

### Other

- *(domain)* parallelize work-dir iterators with tokio spawn+semaphore
- optimize #[expect] attributes — remove 8 unnecessary suppressions
- add clippy.toml and align workspace lints with drive-pal-rs
- 拆分单 crate 为 workspace（domain/infra/cli） ([#22](https://github.com/brightmeows/bms-resource-toolbox/pull/22))
