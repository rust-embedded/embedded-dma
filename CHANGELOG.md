# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

## [v0.2.0] - 2021-02-01

### Changed
- [breaking change] `StaticReadBuffer` and `StaticWriteBuffer` no longer exist. Instead use `ReadBuffer`, `WriteBuffer`.

### Added
- Replace less strict `ReadBuffer` and `WriteBuffer` definitions with
  those of `StaticReadBuffer` and `StaticWriteBuffer`. This removes the separate static
  traits.

## [v0.1.2] - 2020-09-30

### Added
- Added `StaticReadBuffer` and `StaticWriteBuffer`, which are stricter versions of the original traits.

## [v0.1.1] - 2020-09-04

### Added
- Signed integer type `Word` trait implementations.

## v0.1.0 - 2020-08-20

Initial release

[unreleased]: https://github.com/rust-embedded/embedded-dma/compare/v0.2.0...HEAD
[v0.2.0]: https://github.com/rust-embedded/embedded-dma/compare/v0.1.2...v0.2.0
[v0.1.2]: https://github.com/rust-embedded/embedded-dma/compare/v0.1.1...v0.1.2
[v0.1.1]: https://github.com/rust-embedded/embedded-dma/compare/v0.1.0...v0.1.1
