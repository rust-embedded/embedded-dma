//! Traits to aid the correct use of buffers in DMA abstractions.
//!
//! This library provides the [`ReadBuffer`] and [`WriteBuffer`] unsafe traits to be used as bounds to
//! buffers types used in DMA operations.
//!
//! There are some subtleties to the extent of the guarantees provided by these traits, all of these
//! subtleties are properly documented in the safety requirements in this crate. However, as a
//! measure of redundancy, some are listed below:
//!
//! * The traits only guarantee a stable location while no `&mut self` methods are called upon
//! `Self` (with the exception of [`write_buffer`](trait.WriteBuffer.html#tymethod.write_buffer) in
//! our case). This is to allow types like `Vec`, this restriction doesn't apply to `Self::Target`.
//!
//! * [`ReadBuffer`] and [`WriteBuffer`] guarantee a stable location for as long as the DMA transfer
//! occurs. Given the intrinsics of `mem::forget` and the Rust language itself, a
//! 'static lifetime is usually required.
//!
//! The above list is not exhaustive, for a complete set of requirements and guarantees, the
//! documentation of each trait and method should be analyzed.
#![no_std]

use core::{
    mem::{self, MaybeUninit},
    ops::{Deref, DerefMut},
};
use stable_deref_trait::StableDeref;

/// Trait for buffers that can be given to DMA for reading.
///
/// # Safety
///
/// The implementing type must be safe to use for DMA reads. This means:
///
/// - It must be a pointer that references the actual buffer.
/// - As long as no `&mut self` method is called on the implementing object:
///   - `read_buffer` must always return the same value, if called multiple
///     times.
///   - The memory specified by the pointer and size returned by `read_buffer`
///     must not be freed during the transfer it is used in as long as `self` is not dropped.
pub unsafe trait ReadBuffer {
    type Word;

    /// Provide a buffer usable for DMA reads.
    ///
    /// The return value is:
    ///
    /// - pointer to the start of the buffer
    /// - buffer size in words
    ///
    /// # Safety
    ///
    /// Once this method has been called, it is unsafe to call any `&mut self`
    /// methods on this object as long as the returned value is in use (by DMA).
    unsafe fn read_buffer(&self) -> (*const Self::Word, usize);
}

/// Trait for buffers that can be given to DMA for writing.
///
/// # Safety
///
/// The implementing type must be safe to use for DMA writes. This means:
///
/// - It must be a pointer that references the actual buffer.
/// - `Target` must be a type that is valid for any possible byte pattern.
/// - As long as no `&mut self` method, except for `write_buffer`, is called on
///   the implementing object:
///   - `write_buffer` must always return the same value, if called multiple
///     times.
///   - The memory specified by the pointer and size returned by `write_buffer`
///     must not be freed during the transfer as long as `self` is not dropped.
pub unsafe trait WriteBuffer {
    type Word;

    /// Provide a buffer usable for DMA writes.
    ///
    /// The return value is:
    ///
    /// - pointer to the start of the buffer
    /// - buffer size in words
    ///
    /// # Safety
    ///
    /// Once this method has been called, it is unsafe to call any `&mut self`
    /// methods, except for `write_buffer`, on this object as long as the
    /// returned value is in use (by DMA).
    unsafe fn write_buffer(&mut self) -> (*mut Self::Word, usize);
}

// Blanket implementations for common DMA buffer types.

unsafe impl<B, T> ReadBuffer for B
where
    B: Deref<Target = T> + StableDeref + 'static,
    T: ReadTarget + ?Sized,
{
    type Word = T::Word;

    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        self.as_read_buffer()
    }
}

unsafe impl<B, T> WriteBuffer for B
where
    B: DerefMut<Target = T> + StableDeref + 'static,
    T: WriteTarget + ?Sized,
{
    type Word = T::Word;

    unsafe fn write_buffer(&mut self) -> (*mut Self::Word, usize) {
        self.as_write_buffer()
    }
}

/// Trait for DMA word types used by the blanket DMA buffer impls.
///
/// # Safety
///
/// Types that implement this trait must be valid for every possible byte
/// pattern. This is to ensure that, whatever DMA writes into the buffer,
/// we won't get UB due to invalid values.
pub unsafe trait Word {}

unsafe impl Word for u8 {}
unsafe impl Word for i8 {}
unsafe impl Word for u16 {}
unsafe impl Word for i16 {}
unsafe impl Word for u32 {}
unsafe impl Word for i32 {}
unsafe impl Word for u64 {}
unsafe impl Word for i64 {}

/// Trait for `Deref` targets used by the blanket `DmaReadBuffer` impl.
///
/// This trait exists solely to work around
/// https://github.com/rust-lang/rust/issues/20400.
///
/// # Safety
///
/// - `as_read_buffer` must adhere to the safety requirements
///   documented for [`ReadBuffer::read_buffer`].
pub unsafe trait ReadTarget {
    type Word: Word;

    fn as_read_buffer(&self) -> (*const Self::Word, usize) {
        let len = mem::size_of_val(self) / mem::size_of::<Self::Word>();
        let ptr = self as *const _ as *const Self::Word;
        (ptr, len)
    }
}

/// Trait for `DerefMut` targets used by the blanket `DmaWriteBuffer` impl.
///
/// This trait exists solely to work around
/// https://github.com/rust-lang/rust/issues/20400.
///
/// # Safety
///
/// - `as_write_buffer` must adhere to the safety requirements
///   documented for [`WriteBuffer::write_buffer`].
pub unsafe trait WriteTarget {
    type Word: Word;

    fn as_write_buffer(&mut self) -> (*mut Self::Word, usize) {
        let len = mem::size_of_val(self) / mem::size_of::<Self::Word>();
        let ptr = self as *mut _ as *mut Self::Word;
        (ptr, len)
    }
}

unsafe impl<W: Word> ReadTarget for W {
    type Word = W;
}

unsafe impl<W: Word> WriteTarget for W {
    type Word = W;
}

unsafe impl<T: ReadTarget> ReadTarget for [T] {
    type Word = T::Word;
}

unsafe impl<T: WriteTarget> WriteTarget for [T] {
    type Word = T::Word;
}

unsafe impl<T: ReadTarget, const N: usize> ReadTarget for [T; N] {
    type Word = T::Word;
}

unsafe impl<T: WriteTarget, const N: usize> WriteTarget for [T; N] {
    type Word = T::Word;
}

unsafe impl<T: WriteTarget> WriteTarget for MaybeUninit<T> {
    type Word = T::Word;
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::any::Any;

    fn api_read<W, B>(buffer: B) -> (*const W, usize)
    where
        B: ReadBuffer<Word = W>,
    {
        unsafe { buffer.read_buffer() }
    }

    fn api_write<W, B>(mut buffer: B) -> (*mut W, usize)
    where
        B: WriteBuffer<Word = W>,
    {
        unsafe { buffer.write_buffer() }
    }

    #[test]
    fn read_api() {
        const SIZE: usize = 128;
        static BUF: [u8; SIZE] = [0u8; SIZE];

        let (ptr, size_local) = api_read(&BUF);
        assert!(unsafe { (&*ptr as &dyn Any).is::<u8>() });
        assert_eq!(size_local, SIZE);
    }

    #[test]
    fn write_api() {
        const SIZE: usize = 128;
        static mut BUF: [u8; SIZE] = [0u8; SIZE];

        let (ptr, size_local) = api_write(unsafe { &mut BUF });
        assert!(unsafe { (&*ptr as &dyn Any).is::<u8>() });
        assert_eq!(size_local, SIZE);
    }
}
