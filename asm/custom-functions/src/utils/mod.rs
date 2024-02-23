use core::ops::{Deref, DerefMut};

pub mod char_writer;
pub mod console;
pub mod graphics;
pub mod menu;

pub fn simple_rng(rng: &mut u32) -> u32 {
    *rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
    *rng
}

#[repr(align(0x20))]
pub struct AlignedBuf<const SIZE: usize> {
    buf: [u8; SIZE],
}

impl<const SIZE: usize> Default for AlignedBuf<SIZE> {
    fn default() -> Self {
        Self { buf: [0; SIZE] }
    }
}

impl<const SIZE: usize> Deref for AlignedBuf<SIZE> {
    type Target = [u8; SIZE];
    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl<const SIZE: usize> DerefMut for AlignedBuf<SIZE> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf
    }
}
