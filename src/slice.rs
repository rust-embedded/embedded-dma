pub use sealed::{ReadBufferExt, WriteBufferExt};

use crate::slice::sealed::BufferRange;
use crate::{ReadBuffer, WriteBuffer};

mod sealed {
    use core::ops::{Range, RangeFrom, RangeFull, RangeTo};

    use crate::slice::{ReadBufferSlice, WriteBufferSlice};
    use crate::{ReadBuffer, WriteBuffer};

    /// An extension trait used for [crate::ReadBuffer]
    pub trait ReadBufferExt: Sized + ReadBuffer {
        /// Turn the given [crate::ReadBuffer] into a [ReadBufferSlice].
        fn into_read_buffer_slice<B: BufferRange>(self, range: B) -> Option<ReadBufferSlice<Self>> {
            ReadBufferSlice::new(self, range)
        }
    }

    /// An extension trait used for [crate::WriteBuffer]
    pub trait WriteBufferExt: Sized + WriteBuffer {
        /// Turn the given [crate::WriteBuffer] into a [WriteBufferSlice].
        fn into_write_buffer_slice<B: BufferRange>(
            self,
            range: B,
        ) -> Option<WriteBufferSlice<Self>> {
            WriteBufferSlice::new(self, range)
        }
    }

    pub trait BufferRange {
        fn into_range(self, len: usize) -> Option<Range<usize>>;
    }

    impl BufferRange for Range<usize> {
        fn into_range(self, len: usize) -> Option<Range<usize>> {
            if self.start > self.end || self.end > len {
                // degenerate range
                None
            } else {
                Some(self)
            }
        }
    }

    impl BufferRange for RangeTo<usize> {
        fn into_range(self, len: usize) -> Option<Range<usize>> {
            if self.end > len {
                None
            } else {
                Some(Range {
                    start: 0,
                    end: self.end,
                })
            }
        }
    }

    impl BufferRange for RangeFull {
        fn into_range(self, len: usize) -> Option<Range<usize>> {
            Some(Range { start: 0, end: len })
        }
    }

    impl BufferRange for RangeFrom<usize> {
        fn into_range(self, len: usize) -> Option<Range<usize>> {
            if self.start > len {
                None
            } else {
                Some(Range {
                    start: self.start,
                    end: len,
                })
            }
        }
    }
}

impl<T: ReadBuffer + Sized> ReadBufferExt for T {}
impl<T: WriteBuffer + Sized> WriteBufferExt for T {}

/// A [ReadBufferSlice] is a slice which wraps a [ReadBuffer] and implements [ReadBuffer]
/// - See [WriteBufferSlice] for a similar use-case.
pub struct ReadBufferSlice<T: ReadBuffer> {
    ptr: *const T::Word,
    len: usize,
    inner: T,
}

impl<T: ReadBuffer> ReadBufferSlice<T> {
    /// Create a new [BufferSlice]
    fn new(inner: T, range: impl BufferRange) -> Option<Self> {
        // all invariants are satisfied—we are consuming inner—we know no
        // &mut self methods can be called until

        let (ptr, len) = unsafe { inner.read_buffer() };

        let into = range.into_range(len);

        into.map(|range| unsafe {
            Self {
                ptr: ptr.add(range.start),
                len: range.len(),
                inner,
            }
        })
    }

    /// Consume the [ReadBufferSlice] and return the wrapped value
    pub fn inner(self) -> T {
        self.inner
    }
}

/// A [WriteBufferSlice] is a slice which wraps a [WriteBuffer] and implements [WriteBuffer]
/// # Use Case
/// Many HALs use the length of a {Read,Write}Buffer to configure DMA Transfers. However, changing
/// the length of the buffer can be complicated. For instance consider the case where we want to
/// change the length of a slice for a DMA transfer:
/// ```
/// use embedded_dma::{ReadBufferExt, WriteBuffer, WriteBufferExt};
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
///     let buffer_slice = buffer.into_write_buffer_slice(..length).unwrap();
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
pub struct WriteBufferSlice<T: WriteBuffer> {
    ptr: *mut T::Word,
    len: usize,
    inner: T,
}

impl<T: WriteBuffer> WriteBufferSlice<T> {
    /// Create a new [BufferSlice]
    fn new(mut inner: T, range: impl BufferRange) -> Option<Self> {
        let (ptr, len) = unsafe { inner.write_buffer() };

        let into = range.into_range(len);

        into.map(|range| unsafe {
            Self {
                ptr: ptr.add(range.start),
                len: range.len(),
                inner,
            }
        })
    }

    /// Consume the [WriteBufferSlice] and return the wrapped value
    pub fn inner(self) -> T {
        self.inner
    }
}

unsafe impl<T: ReadBuffer> ReadBuffer for ReadBufferSlice<T> {
    type Word = T::Word;

    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        (self.ptr, self.len)
    }
}

unsafe impl<T: WriteBuffer> WriteBuffer for WriteBufferSlice<T> {
    type Word = T::Word;
    unsafe fn write_buffer(&mut self) -> (*mut Self::Word, usize) {
        (self.ptr, self.len)
    }
}
