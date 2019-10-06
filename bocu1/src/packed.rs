//! This module presents a secondary representation of _small_ BOCU-1 strings as
//! single packed integer-type values. For example, rather than storing an
//! encoded string "\xAA\xBB\xCC" in a [u8] byte buffer, this module presents
//! mechanisms to pack it into (say) a single u32 scalar value: 0xAABBCC00
//!
//! Why do this? Simple: your computer has registers that can hold such scalars
//! directly! No need to allocate separate memory even in the stack. If you have
//! access to 128-bit scalars, you can pack all 16-byte small strings in there,
//! which covers quite a lot of the strings in most programs.
//!
//! Moreover this packed representation is designed so that the first byte of
//! the string is the high byte in the packed scalar, meaning it preserves the
//! nice BOCU-1 property of "lexicographical unicode codepoint order": doing a
//! single scalar compare on packed strings in this form is sufficient to
//! compare their unicode string values (at least at the crude codepoint level
//! -- no UCA or CLDR logic at this level).

use crate::delta_encoding;
use crate::DecodeError;
use crate::EncodeBOCU1;
use num_integer::Integer;
use std::mem;
use std::ops::{BitAnd, BitOrAssign, ShlAssign, ShrAssign};
use try_from::TryInto;

pub fn pack<IT, N>(i: &IT) -> Option<N>
where
    IT: EncodeBOCU1,
    N: Copy + Integer + ShlAssign<usize> + BitOrAssign<N> + From<u8>,
{
    let mut tmp: N = N::zero();
    let mut n: usize = mem::size_of::<N>();
    for c in i.encode_bocu1() {
        if n == 0 {
            return None;
        }
        tmp <<= 8;
        tmp |= N::from(c);
        n -= 1;
    }
    while n != 0 {
        tmp <<= 8;
        n -= 1;
    }
    Some(tmp)
}

pub struct DecodePackedResultIter {
    state: delta_encoding::DeltaCoder,
    // At present, no scalar types are more than 16 bytes. This could be generic
    // over fixed-size arrays at some point after const generics, though I don't
    // see a lot of benefit.
    buf: [u8; 16],
    rem: usize,
}

impl DecodePackedResultIter {
    pub fn pos(rem: usize) -> usize {
        assert!(rem <= 16);
        16 - rem
    }
    pub fn range(rem: usize) -> std::ops::Range<usize> {
        assert!(rem <= 16);
        (16 - rem)..16
    }
    pub fn new<N>(mut n: N) -> Self
    where
        N: Copy + Integer + ShrAssign<usize> + BitAnd<N, Output = N> + From<u8> + TryInto<u8>,
    {
        // A packed value is always right-shifted to make the encoded string's
        // first byte be the most-significant byte. So for example the encoded
        // string "\xAA\xBB\xCC" will be packed into a u32 as: 0xAABBCC00.
        //
        // That is, every packed value will have 0 or more low 00 NUL bytes and
        // then a sequence of non-NUL high bytes representing the original
        // string. Packed strings do not allow or represent internal /
        // non-terminal NUL bytes.
        //
        // To unpack this back to a byte buffer, we walk down from end to start
        // depositing bytes at the _end_ of the buffer, and counting the non-NUL
        // bytes to form a "bytes remaining" count.
        //
        // This somewhat awkward arrangement means that we can incrementally
        // decode bytes _from_ this remaining-byte supply back to unicode
        // scalars one by one, while only tracking the one "remaining bytes"
        // value, not a start/end pair. I don't much like this but I tried a few
        // other approaches and they read even more awkwardly. Feel free to
        // propose something else!
        let mask = N::from(0xff_u8);
        let mut buf: [u8; 16] = [0; 16];
        let mut rem: usize = 0;
        for _ in 0..mem::size_of::<N>() {
            let byte: u8 = (n & mask).try_into().unwrap_or(0);
            if byte != 0 {
                rem += 1;
                buf[Self::pos(rem)] = byte
            }
            n >>= 8;
        }
        Self {
            state: delta_encoding::DeltaCoder::new(),
            buf: buf,
            rem: rem,
        }
    }
}

impl Iterator for DecodePackedResultIter {
    type Item = Result<char, DecodeError>;
    fn next(self: &mut Self) -> Option<Result<char, DecodeError>> {
        loop {
            if self.rem == 0 {
                return None;
            }
            match self.state.decode_char(&self.buf[Self::range(self.rem)]) {
                Ok((None, rest)) => self.rem = rest.len(),
                Ok((Some(c), rest)) => {
                    self.rem = rest.len();
                    return Some(Ok(c));
                }
                Err(e) => {
                    return Some(Err(e));
                }
            }
        }
    }
}

pub struct DecodePackedIter {
    inner: DecodePackedResultIter,
}

impl DecodePackedIter {
    pub fn new<N>(n: N) -> Self
    where
        N: Copy + Integer + ShrAssign<usize> + BitAnd<N, Output = N> + From<u8> + TryInto<u8>,
    {
        Self {
            inner: DecodePackedResultIter::new(n),
        }
    }
}

impl Iterator for DecodePackedIter {
    type Item = char;
    fn next(self: &mut Self) -> Option<char> {
        match self.inner.next() {
            None | Some(Err(_)) => None,
            Some(Ok(c)) => Some(c),
        }
    }
}

pub trait DecodePackedBOCU1 {
    fn decode_packed_bocu1(self: &Self) -> DecodePackedIter;
}

impl<T> DecodePackedBOCU1 for T
where
    T: Copy + Integer + ShrAssign<usize> + BitAnd<T, Output = T> + From<u8> + TryInto<u8>,
{
    fn decode_packed_bocu1(self: &Self) -> DecodePackedIter {
        DecodePackedIter::new(*self)
    }
}
