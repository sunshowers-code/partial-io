# Changelog

## [0.5.4] - 2022-09-27

### Fixed

- For proptest, generate `PartialOp::Limited` byte counts starting at 1 rather than 0. This is because
  0 can mean no more data is available in the stream.

## [0.5.3] - 2022-09-27

### Updated

- Documentation updates.

## [0.5.2] - 2022-09-27

### Added

- Support for generating random `PartialOp`s using `proptest`.

## [0.5.1] - 2022-09-27

### Changed

- Updated repository location.

## [0.5.0] - 2021-01-27

### Changed
- Updated `quickcheck` to version 1.0. Feature renamed to `quickcheck1`.
- Updated `tokio` to version 1.0. Feature renamed to `tokio1`.

## [0.4.0] - 2020-09-24

### Changed
- Updated to quickcheck 0.9, tokio 0.2 and futures 0.3.

For information about earlier versions, please review the [commit history](https://github.com/sunshowers-code/partial-io/commits/main).

[0.5.4]: https://github.com/sunshowers-code/partial-io/releases/tag/0.5.4
[0.5.3]: https://github.com/sunshowers-code/partial-io/releases/tag/0.5.3
[0.5.2]: https://github.com/sunshowers-code/partial-io/releases/tag/0.5.2
[0.5.1]: https://github.com/sunshowers-code/partial-io/releases/tag/0.5.1

<!-- older releases are on the facebookincubator repo -->
[0.5.0]: https://github.com/facebookincubator/rust-partial-io/releases/tag/0.5.0
[0.4.0]: https://github.com/facebookincubator/rust-partial-io/releases/tag/0.4.0
