# 0.2.0 (June 10, 2020)

### Breaking changes
- Require [`v2::OutputPins`](https://github.com/rust-embedded/embedded-hal/blob/9e6ab5a1ee8900830bd4fe56f0a84ddb0bccda3f/src/digital/v2.rs)
- [`PCD8544::new`](https://github.com/kellerkindt/pcd8544/blob/98ef5b7d0264aa610bd758940478975d08270f32/src/lib.rs#L77) now returns a `Result`
- [`OutputPins` are now taken by ownership](https://github.com/kellerkindt/pcd8544/blob/98ef5b7d0264aa610bd758940478975d08270f32/src/lib.rs#L70) instead of mut refs [#2](https://github.com/kellerkindt/pcd8544/pull/2), thanks @kolen

### Changes
- Upgrade to Rust 2018 Edition
- All methods changing the state of an `OutputPin` now return a `Result`, which should be used
