// Copyright (c) The partial-io Contributors
// SPDX-License-Identifier: MIT

//! This module contains an `AsyncWrite` wrapper that breaks writes up
//! according to a provided iterator.
//!
//! This is separate from `PartialWrite` because on `WouldBlock` errors, it
//! causes `futures` to try writing or flushing again.

use crate::{futures_util::FuturesOps, PartialOp};
use futures::{io, prelude::*};
use pin_project::pin_project;
use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

/// A wrapper that breaks inner `AsyncWrite` instances up according to the
/// provided iterator.
///
/// Available with the `futures03` feature for `futures` traits, and with the `tokio1` feature for
/// `tokio` traits.
///
/// # Examples
///
/// This example uses `tokio`.
///
/// ```rust
/// # #[cfg(feature = "tokio1")]
/// use partial_io::{PartialAsyncWrite, PartialOp};
/// # #[cfg(feature = "tokio1")]
/// use std::io::{self, Cursor};
/// # #[cfg(feature = "tokio1")]
/// use tokio::io::AsyncWriteExt;
///
/// # #[cfg(feature = "tokio1")]
/// #[tokio::main]
/// async fn main() -> io::Result<()> {
///     let writer = Cursor::new(Vec::new());
///     // Sequential calls to `poll_write()` and the other `poll_` methods simulate the following behavior:
///     let iter = vec![
///         PartialOp::Err(io::ErrorKind::WouldBlock),   // A not-ready state.
///         PartialOp::Limited(2),                       // Only allow 2 bytes to be written.
///         PartialOp::Err(io::ErrorKind::InvalidData),  // Error from the underlying stream.
///         PartialOp::Unlimited,                        // Allow as many bytes to be written as possible.
///     ];
///     let mut partial_writer = PartialAsyncWrite::new(writer, iter);
///     let in_data = vec![1, 2, 3, 4];
///
///     // This causes poll_write to be called twice, yielding after the first call (WouldBlock).
///     assert_eq!(partial_writer.write(&in_data).await?, 2);
///     let cursor_ref = partial_writer.get_ref();
///     let out = cursor_ref.get_ref();
///     assert_eq!(&out[..], &[1, 2]);
///
///     // This next call returns an error.
///     assert_eq!(
///         partial_writer.write(&in_data[2..]).await.unwrap_err().kind(),
///         io::ErrorKind::InvalidData,
///     );
///
///     // And this one causes the last two bytes to be written.
///     assert_eq!(partial_writer.write(&in_data[2..]).await?, 2);
///     let cursor_ref = partial_writer.get_ref();
///     let out = cursor_ref.get_ref();
///     assert_eq!(&out[..], &[1, 2, 3, 4]);
///
///     Ok(())
/// }
///
/// # #[cfg(not(feature = "tokio1"))]
/// # fn main() {
/// #     assert!(true, "dummy test");
/// # }
/// ```
#[pin_project]
pub struct PartialAsyncWrite<W> {
    #[pin]
    inner: W,
    ops: FuturesOps,
}

impl<W> PartialAsyncWrite<W> {
    /// Creates a new `PartialAsyncWrite` wrapper over the writer with the specified `PartialOp`s.
    pub fn new<I>(inner: W, iter: I) -> Self
    where
        I: IntoIterator<Item = PartialOp> + 'static,
        I::IntoIter: Send,
    {
        PartialAsyncWrite {
            inner,
            ops: FuturesOps::new(iter),
        }
    }

    /// Sets the `PartialOp`s for this writer.
    pub fn set_ops<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = PartialOp> + 'static,
        I::IntoIter: Send,
    {
        self.ops.replace(iter);
        self
    }

    /// Sets the `PartialOp`s for this writer in a pinned context.
    pub fn pin_set_ops<I>(self: Pin<&mut Self>, iter: I) -> Pin<&mut Self>
    where
        I: IntoIterator<Item = PartialOp> + 'static,
        I::IntoIter: Send,
    {
        let mut this = self;
        this.as_mut().project().ops.replace(iter);
        this
    }

    /// Returns a shared reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Returns a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Returns a pinned mutable reference to the underlying writer.
    pub fn pin_get_mut(self: Pin<&mut Self>) -> Pin<&mut W> {
        self.project().inner
    }

    /// Consumes this wrapper, returning the underlying writer.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

// ---
// Futures impls
// ---

impl<W> AsyncWrite for PartialAsyncWrite<W>
where
    W: AsyncWrite,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.project();
        let inner = this.inner;

        this.ops.poll_impl(
            cx,
            |cx, len| match len {
                Some(len) => inner.poll_write(cx, &buf[..len]),
                None => inner.poll_write(cx, buf),
            },
            buf.len(),
            "error during poll_write, generated by partial-io",
        )
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        let this = self.project();
        let inner = this.inner;

        this.ops.poll_impl_no_limit(
            cx,
            |cx| inner.poll_flush(cx),
            "error during poll_flush, generated by partial-io",
        )
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        let this = self.project();
        let inner = this.inner;

        this.ops.poll_impl_no_limit(
            cx,
            |cx| inner.poll_close(cx),
            "error during poll_close, generated by partial-io",
        )
    }
}

/// This is a forwarding impl to support duplex structs.
impl<W> AsyncRead for PartialAsyncWrite<W>
where
    W: AsyncRead,
{
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.project().inner.poll_read(cx, buf)
    }

    #[inline]
    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context,
        bufs: &mut [io::IoSliceMut],
    ) -> Poll<io::Result<usize>> {
        self.project().inner.poll_read_vectored(cx, bufs)
    }
}

/// This is a forwarding impl to support duplex structs.
impl<W> AsyncBufRead for PartialAsyncWrite<W>
where
    W: AsyncBufRead,
{
    #[inline]
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<&[u8]>> {
        self.project().inner.poll_fill_buf(cx)
    }

    #[inline]
    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().inner.consume(amt)
    }
}

/// This is a forwarding impl to support duplex structs.
impl<W> AsyncSeek for PartialAsyncWrite<W>
where
    W: AsyncSeek,
{
    #[inline]
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context,
        pos: io::SeekFrom,
    ) -> Poll<io::Result<u64>> {
        self.project().inner.poll_seek(cx, pos)
    }
}

// ---
// Tokio impls
// ---

#[cfg(feature = "tokio1")]
mod tokio_impl {
    use super::PartialAsyncWrite;
    use std::{
        io::{self, SeekFrom},
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

    impl<W> AsyncWrite for PartialAsyncWrite<W>
    where
        W: AsyncWrite,
    {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.project();
            let inner = this.inner;

            this.ops.poll_impl(
                cx,
                |cx, len| match len {
                    Some(len) => inner.poll_write(cx, &buf[..len]),
                    None => inner.poll_write(cx, buf),
                },
                buf.len(),
                "error during poll_write, generated by partial-io",
            )
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
            let this = self.project();
            let inner = this.inner;

            this.ops.poll_impl_no_limit(
                cx,
                |cx| inner.poll_flush(cx),
                "error during poll_flush, generated by partial-io",
            )
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
            let this = self.project();
            let inner = this.inner;

            this.ops.poll_impl_no_limit(
                cx,
                |cx| inner.poll_shutdown(cx),
                "error during poll_shutdown, generated by partial-io",
            )
        }
    }

    /// This is a forwarding impl to support duplex structs.
    impl<W> AsyncRead for PartialAsyncWrite<W>
    where
        W: AsyncRead,
    {
        #[inline]
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            self.project().inner.poll_read(cx, buf)
        }
    }

    /// This is a forwarding impl to support duplex structs.
    impl<W> AsyncBufRead for PartialAsyncWrite<W>
    where
        W: AsyncBufRead,
    {
        #[inline]
        fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<&[u8]>> {
            self.project().inner.poll_fill_buf(cx)
        }

        #[inline]
        fn consume(self: Pin<&mut Self>, amt: usize) {
            self.project().inner.consume(amt)
        }
    }

    /// This is a forwarding impl to support duplex structs.
    impl<W> AsyncSeek for PartialAsyncWrite<W>
    where
        W: AsyncSeek,
    {
        #[inline]
        fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
            self.project().inner.start_seek(position)
        }

        #[inline]
        fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
            self.project().inner.poll_complete(cx)
        }
    }
}

impl<W> fmt::Debug for PartialAsyncWrite<W>
where
    W: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PartialAsyncWrite")
            .field("inner", &self.inner)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;

    use crate::tests::assert_send;

    #[test]
    fn test_sendable() {
        assert_send::<PartialAsyncWrite<File>>();
    }
}
