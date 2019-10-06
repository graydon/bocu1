#![allow(dead_code)]

//! This crate exists just to provide a testing interface
//! between the IBM reference implementation (in C) and
//! the Rust bocu1 crate. If you're not presently doing
//! validation of the Rust crate against the C refimpl,
//! you can completely ignore this crate.

extern crate bocu1;
extern crate cty;
use bocu1::{DrainEncodedChunkIter, EncodedChunk};
use cty::{int32_t, uint8_t};

#[repr(C)]
struct Bocu1Rx {
    prev: int32_t,
    count: int32_t,
    diff: int32_t,
}

extern "C" {
    fn encodeBocu1(pPrev: *const int32_t, c: int32_t) -> int32_t;
    fn decodeBocu1(pRx: *mut Bocu1Rx, b: uint8_t) -> int32_t;
}

fn packed_to_chunk(packed: int32_t) -> EncodedChunk {
    let a = ((packed >> 24) & 0xff) as u8;
    let b = ((packed >> 16) & 0xff) as u8;
    let c = ((packed >> 8) & 0xff) as u8;
    let d = (packed & 0xff) as u8;
    let count = if a > 0 && a < 4 { a as usize } else { 4 };
    let bytes = match count {
        1 => [d, 0, 0, 0],
        2 => [c, d, 0, 0],
        3 => [b, c, d, 0],
        4 => [a, b, c, d],
        _ => unreachable!(),
    };
    EncodedChunk {
        bytes: bytes,
        count: count,
    }
}

pub struct RefImplEncodedChunkIter<IT>
where
    IT: Iterator<Item = char>,
{
    input: IT,
    prev: int32_t,
}

impl<IT> RefImplEncodedChunkIter<IT>
where
    IT: Iterator<Item = char>,
{
    pub fn new(input: IT) -> RefImplEncodedChunkIter<IT> {
        RefImplEncodedChunkIter {
            input: input,
            prev: 0x40,
        }
    }
}

impl<IT> Iterator for RefImplEncodedChunkIter<IT>
where
    IT: Iterator<Item = char>,
{
    type Item = EncodedChunk;
    fn next(self: &mut Self) -> Option<EncodedChunk> {
        match self.input.next() {
            None => None,
            Some(ch) => {
                let packed = unsafe { encodeBocu1(&self.prev, ch as int32_t) };
                Some(packed_to_chunk(packed))
            }
        }
    }
}

pub type RefImplEncodeIter<IT> = DrainEncodedChunkIter<RefImplEncodedChunkIter<IT>>;

pub trait RefImplEncodeBOCU1 {
    type IT: Iterator<Item = char>;
    fn refimpl_encode_bocu1(self: &Self) -> RefImplEncodeIter<Self::IT>;
}

impl<'a> RefImplEncodeBOCU1 for &'a str {
    type IT = ::std::str::Chars<'a>;
    fn refimpl_encode_bocu1(self: &Self) -> RefImplEncodeIter<Self::IT> {
        let inner = RefImplEncodedChunkIter::new(self.chars());
        DrainEncodedChunkIter::new(inner)
    }
}

struct RefImplDecoder {
    rx: Bocu1Rx,
}

#[cfg(test)]
mod tests;
