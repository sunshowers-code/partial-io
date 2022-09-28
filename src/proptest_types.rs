// Copyright (c) The partial-io Contributors
// SPDX-License-Identifier: MIT

//! Proptest support for partial IO operations.
//!
//! This module allows sequences of [`PartialOp`]s to be randomly generated. These
//! sequences can then be fed into a [`PartialRead`](crate::PartialRead),
//! [`PartialWrite`](crate::PartialWrite), [`PartialAsyncRead`](crate::PartialAsyncRead) or
//! [`PartialAsyncWrite`](crate::PartialAsyncWrite).
//!
//! Once `proptest` has identified a failing test case, it will shrink the sequence of `PartialOp`s
//! and find a minimal test case. This minimal case can then be used to reproduce the issue.
//!
//! Basic implementations are provided for:
//! - generating errors some of the time
//! - generating [`PartialOp`] instances, given a way to generate errors.
//!
//! # Examples
//!
//! ```rust
//! use partial_io::proptest_types::{partial_op_strategy, interrupted_strategy};
//! use proptest::{collection::vec, prelude::*};
//!
//! proptest! {
//!     #[test]
//!     fn proptest_something(ops: vec(partial_op_strategy(interrupted_strategy(), 128), 0..128)) {
//!         // Example buffer to read from, substitute with your own.
//!         let reader = std::io::repeat(42);
//!         let partial_reader = PartialRead::new(reader, ops);
//!         // ...
//!
//!         true
//!     }
//! }
//! ```
//!
//! For a detailed example, see `examples/buggy_write.rs` in this repository.

use crate::PartialOp;
use proptest::{option::weighted, prelude::*};
use std::io;

/// Returns a strategy that generates `PartialOp` instances given a way to generate errors.
///
/// To not generate any errors and only limit reads, pass in `Just(None)` as the error strategy.
pub fn partial_op_strategy(
    error_strategy: impl Strategy<Value = Option<io::ErrorKind>>,
    limit_bytes: usize,
) -> impl Strategy<Value = PartialOp> {
    // Don't generate 0 because for writers it can mean that writes are no longer accepted.
    (error_strategy, 1..=limit_bytes).prop_map(|(error_kind, limit)| match error_kind {
        Some(kind) => PartialOp::Err(kind),
        None => PartialOp::Limited(limit),
    })
}

/// Returns a strategy that generates `Interrupted` errors 20% of the time.
pub fn interrupted_strategy() -> impl Strategy<Value = Option<io::ErrorKind>> {
    weighted(0.2, Just(io::ErrorKind::Interrupted))
}

/// Returns a strategy that generates `WouldBlock` errors 20% of the time.
pub fn would_block_strategy() -> impl Strategy<Value = Option<io::ErrorKind>> {
    weighted(0.2, Just(io::ErrorKind::WouldBlock))
}

/// Returns a strategy that generates `Interrupted` errors 10% of the time and `WouldBlock` errors
/// 10% of the time.
pub fn interrupted_would_block_strategy() -> impl Strategy<Value = Option<io::ErrorKind>> {
    weighted(
        0.2,
        prop_oneof![
            Just(io::ErrorKind::Interrupted),
            Just(io::ErrorKind::WouldBlock),
        ],
    )
}
