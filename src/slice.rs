use crate::slice::sealed::BufferRange;
use crate::{ReadBuffer, WriteBuffer};
use core::ops::Range;
pub use sealed::BufferExt;

mod sealed {
    use crate::slice::BufferSlice;
    use core::ops::{Range, RangeFrom, RangeFull, RangeTo};

    /// An extension trait used for [crate::ReadBuffer] and [crate::WriteBuffer]
    pub trait BufferExt: Sized {
        /// Turn the given [crate::ReadBuffer] or [crate::WriteBuffer] into a BufferSlice.
        /// This method has no use if the struct does not implement one of the listed traits.
        fn into_buffer_slice<B: BufferRange>(self, range: B) -> BufferSlice<Self> {
            BufferSlice::new(self, range)
        }
    }

    pub trait BufferRange {
        fn into_range(self) -> Range<usize>;
    }

    impl BufferRange for Range<usize> {
        fn into_range(self) -> Range<usize> {
            if self.start > self.end {
                Range { start: 0, end: 0 }
            } else {
                self
            }
        }
    }

    impl BufferRange for RangeTo<usize> {
        fn into_range(self) -> Range<usize> {
            Range {
                start: 0,
                end: self.end,
            }
        }
    }

    impl BufferRange for RangeFull {
        fn into_range(self) -> Range<usize> {
            Range {
                start: 0,

                // this uses deprecated syntax to allow for a smaller MSRV
                end: core::usize::MAX,
            }
        }
    }

    impl BufferRange for RangeFrom<usize> {
        fn into_range(self) -> Range<usize> {
            Range {
                start: self.start,

                // this uses deprecated syntax to allow for a smaller MSRV
                end: core::usize::MAX,
            }
        }
    }
}

impl<T: Sized> BufferExt for T {}

/// A [BufferSlice] is a slice which wraps either a [ReadBuffer] or a [WriteBuffer]
/// - When it wraps a [ReadBuffer], it implements [ReadBuffer]
/// - When it wraps a [WriteBuffer], it implements [WriteBuffer]
/// - To prevent panics and to enforce safety, the given range is coerced between `[0, len)`, where
/// `len` is the length of the original buffer
/// # Use Case
/// Many HALs use the length of a {Read,Write}Buffer to configure DMA Transfers. However, changing
/// the length of the buffer can be complicated. For instance consider the case where we want to
/// change the length of a slice for a DMA transfer:
/// ```
/// use embedded_dma::{BufferExt, WriteBuffer};
/// struct DmaTransfer<Buf> {
///     buf: Buf,
/// }
///
/// impl<Buf: WriteBuffer> DmaTransfer<Buf> {
///     /// stars the DMA transfer
///     fn start(buf: Buf) -> Self {
///         // DMA logic would go here
///         Self { buf }
///     }
///
///     /// returns if DMA transaction is done.
///     /// this could be called in a polling loop in order to be non-blocking
///     fn is_done(&self) -> bool {
///         true
///     }
///
///     /// returns resources held by the DMA transfer.
///     fn free(self) -> Buf {
///         // busy loop which waits until DMA is done to ensure safety
///         while !self.is_done() {}
///         self.buf
///     }
/// }
///
/// /// This function is bad because we cannot obtain the original slice—just a subset of it.
/// fn dma_transfer_bad1(buffer: &'static mut [u8], length: usize) -> &'static mut [u8] {
///     let sub_slice = &mut buffer[..length];
///     let transfer = DmaTransfer::start(sub_slice);
///     while !transfer.is_done() {}
///     transfer.free()
/// }
///
/// /// This function is bad because we cannot unsplit the slice.
/// fn dma_transfer_bad2(buffer: &'static mut [u8], length: usize) -> &'static mut [u8] {
///     let (slice_a, slice_b) = buffer.split_at_mut(length);
///     let transfer = DmaTransfer::start(slice_a);
///     while !transfer.is_done() {}
///     let slice_a = transfer.free();
///     // can't unsplit!!!
///     slice_a
/// }
///
/// /// This function is good—we can get the whole slice back!
/// fn dma_transfer(buffer: &'static mut [u8], length: usize) -> &'static mut [u8] {
///     let buffer_slice = buffer.into_buffer_slice(..length);
///     let transfer = DmaTransfer::start(buffer_slice);
///     while !transfer.is_done() {}
///     let buffer_slice = transfer.free();
///     buffer_slice.inner()
/// }
///
/// const SIZE: usize = 1024;
///
/// let buffer = {
///     static mut BUFFER: [u8; 1024] = [0_u8; SIZE];
///     unsafe { &mut BUFFER }
/// };
///
/// assert_eq!(buffer.len(), SIZE);
///
/// let returned_buffer = dma_transfer(buffer, 123);
///
/// assert_eq!(returned_buffer.len(), SIZE);
/// ```
pub struct BufferSlice<T> {
    inner: T,
    range: Range<usize>,
}

impl<T> BufferSlice<T> {
    /// Create a new [BufferSlice]
    fn new(inner: T, range: impl BufferRange) -> Self {
        // The range must be span a non-negative length for internal logic to work
        Self {
            inner,
            range: range.into_range(),
        }
    }

    /// Consume the [BufferSlice] and return the wrapped value
    pub fn inner(self) -> T {
        self.inner
    }

    /// Coerce the given range into [0..len)
    fn coerced_range(&self, len: usize) -> Range<usize> {
        let start = self.range.start.min(len);
        let end = self.range.end.min(len);
        Range { start, end }
    }
}

unsafe impl<T: ReadBuffer> ReadBuffer for BufferSlice<T> {
    type Word = T::Word;

    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        let (ptr, len) = self.inner.read_buffer();

        let range = self.coerced_range(len);

        (ptr.add(range.start), range.len())
    }
}

unsafe impl<T: WriteBuffer> WriteBuffer for BufferSlice<T> {
    type Word = T::Word;
    unsafe fn write_buffer(&mut self) -> (*mut Self::Word, usize) {
        let (ptr, len) = self.inner.write_buffer();

        let range = self.coerced_range(len);

        (ptr.add(range.start), range.len())
    }
}
