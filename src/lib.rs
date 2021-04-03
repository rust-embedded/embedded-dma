//! Traits to aid the correct use of buffers in DMA abstractions.
//!
//! This library provides the `ReadBuffer` and `WriteBuffer` unsafe traits to be used as bounds to
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
//! * The location is only guaranteed to be stable for the duration of `Self`, that means that
//! `Self` doesn't need to be `'static`, i.e. `&'a [u8]` is valid. This can be a bit subtle for
//! most DMA abstractions, because they almost always require `'static`, given the intrinsics of
//! `mem::forget` and the Rust language itself. Those APIs must also bound to `'static` and not only
//! `WriteBuffer`/`ReadBuffer`. The reason we don't require `'static` in the traits themselves is
//! because it would block implementations that can deal with stack allocated buffers, like APIs
//! that use closures to prevent memory corruption.
//!
//! If your API also needs a `'static` bound, prefer the use of [StaticReadBuffer] and
//! [StaticWriteBuffer]. They are a stricter version that requires a `'static` lifetime invariant,
//! while also allowing end users to __unsafely__ bypass it.
//!
//! If you are not sure which version of the traits you should be bounding to in your DMA
//! implementations, prefer the "Static" versions, they are sound for a bigger number of techniques
//! that deal with DMA.
//!
//! The above list is not exhaustive, for a complete set of requirements and guarantees, the
//! documentation of each trait and method should be analyzed.
//!
//! [StaticReadBuffer]: trait.StaticReadBuffer.html
//! [StaticWriteBuffer]: trait.StaticWriteBuffer.html
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
///     must not be freed as long as `self` is not dropped.
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
///     must not be freed as long as `self` is not dropped.
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
///   documented for `ReadBuffer::read_buffer`.
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
///   documented for `WriteBuffer::write_buffer`.
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

macro_rules! dma_target_array_impls {
    ( $( $i:expr, )+ ) => {
        $(
            unsafe impl<T: ReadTarget> ReadTarget for [T; $i] {
                type Word = T::Word;
            }

            unsafe impl<T: WriteTarget> WriteTarget for [T; $i] {
                type Word = T::Word;
            }
        )+
    };
}

#[rustfmt::skip]
dma_target_array_impls!(
     0,   1,   2,   3,   4,   5,   6,   7,   8,   9,
    10,  11,  12,  13,  14,  15,  16,  17,  18,  19,
    20,  21,  22,  23,  24,  25,  26,  27,  28,  29,
    30,  31,  32,  33,  34,  35,  36,  37,  38,  39,
    40,  41,  42,  43,  44,  45,  46,  47,  48,  49,
    50,  51,  52,  53,  54,  55,  56,  57,  58,  59,
    60,  61,  62,  63,  64,  65,  66,  67,  68,  69,
    70,  71,  72,  73,  74,  75,  76,  77,  78,  79,
    80,  81,  82,  83,  84,  85,  86,  87,  88,  89,
    90,  91,  92,  93,  94,  95,  96,  97,  98,  99,
   100, 101, 102, 103, 104, 105, 106, 107, 108, 109,
   110, 111, 112, 113, 114, 115, 116, 117, 118, 119,
   120, 121, 122, 123, 124, 125, 126, 127, 128, 129,
   130, 131, 132, 133, 134, 135, 136, 137, 138, 139,
   140, 141, 142, 143, 144, 145, 146, 147, 148, 149,
   150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
   160, 161, 162, 163, 164, 165, 166, 167, 168, 169,
   170, 171, 172, 173, 174, 175, 176, 177, 178, 179,
   180, 181, 182, 183, 184, 185, 186, 187, 188, 189,
   190, 191, 192, 193, 194, 195, 196, 197, 198, 199,
   200, 201, 202, 203, 204, 205, 206, 207, 208, 209,
   210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
   220, 221, 222, 223, 224, 225, 226, 227, 228, 229,
   230, 231, 232, 233, 234, 235, 236, 237, 238, 239,
   240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
   250, 251, 252, 253, 254, 255, 256,

   1 <<  9,
   1 << 10,
   1 << 11,
   1 << 12,
   1 << 13,
   1 << 14,
   1 << 15,
   1 << 16,
);

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
        let local_buf = [0u8; SIZE];

        let (ptr, size_local) = api_read(&BUF);
        assert!(unsafe { (&*ptr as &dyn Any).is::<u8>() });
        assert!(size_local == SIZE);
    }

    #[test]
    fn write_api() {
        const SIZE: usize = 128;
        static mut BUF: [u8; SIZE] = [0u8; SIZE];
        let mut local_buf = [0u8; SIZE];

        let (ptr, size_local) = api_write(unsafe { &mut BUF });
        assert!(unsafe { (&*ptr as &dyn Any).is::<u8>() });
        assert!(size_local == SIZE);
    }
}
