/*
* Iterators for transcoding BOCU-1 to and from various normal Rust types.
*
* This pile of code has nothing to do with BOCU-1 as such, it's just sequence
* adaptors and management of the flow of data in and out of the BOCU-1
* {en,de}coder routines in the other modules.
*/

#![allow(clippy::stutter)]

use crate::delta_encoding;
use std::io;

// There are two levels of encoding iterator: one that returns chunks of
// encoded bytes (one chunk per input character), and another that drains
// chunks sequentially, producing an encoded byte stream.

pub struct EncodedChunk {
    pub bytes: [u8; 4],
    pub count: usize,
}

impl EncodedChunk {
    pub fn new_single(lead: u8) -> Self {
        Self {
            bytes: [lead, 0, 0, 0],
            count: 1,
        }
    }
    pub fn as_slice(self: &Self) -> &[u8] {
        &self.bytes[0..self.count]
    }
}

pub struct EncodedChunkIter<IT>
where
    IT: Iterator<Item = char>,
{
    input: IT,
    coder: delta_encoding::DeltaCoder,
}

impl<IT> EncodedChunkIter<IT>
where
    IT: Iterator<Item = char>,
{
    pub fn new(input: IT) -> Self {
        Self {
            input: input,
            coder: delta_encoding::DeltaCoder::new(),
        }
    }
}

impl<IT> Iterator for EncodedChunkIter<IT>
where
    IT: Iterator<Item = char>,
{
    type Item = EncodedChunk;
    fn next(self: &mut Self) -> Option<EncodedChunk> {
        match self.input.next() {
            None => None,
            Some(ch) => Some(self.coder.encode_char(ch)),
        }
    }
}

pub struct DrainEncodedChunkIter<IT>
where
    IT: Iterator<Item = EncodedChunk>,
{
    inner: IT,
    drain: Option<EncodedChunk>,
    index: usize,
}

impl<IT> DrainEncodedChunkIter<IT>
where
    IT: Iterator<Item = EncodedChunk>,
{
    pub fn new(inner: IT) -> Self {
        Self {
            inner: inner,
            drain: None,
            index: 0,
        }
    }
}

impl<IT> Iterator for DrainEncodedChunkIter<IT>
where
    IT: Iterator<Item = EncodedChunk>,
{
    type Item = u8;
    fn next(self: &mut Self) -> Option<u8> {
        if self.drain.is_none() {
            if let Some(enc) = self.inner.next() {
                self.drain = Some(enc);
                self.index = 0;
            }
        }
        let (ret, reset) = match &self.drain {
            None => (None, false),
            Some(enc) => {
                assert!(self.index < enc.count);
                let ret = Some(enc.bytes[self.index]);
                self.index += 1;
                let reset = self.index == enc.count;
                (ret, reset)
            }
        };
        if reset {
            self.drain = None;
        }
        ret
    }
}

// Convenience methods for building composite encode-and-drain-chunk
// iterators off of str and vec[char]. Probably someone who knows the
// iterator protocols better will laugh at this but I am a novice.

pub type EncodeIter<IT> = DrainEncodedChunkIter<EncodedChunkIter<IT>>;

pub trait EncodeBOCU1 {
    type IT: Iterator<Item = char>;
    fn encode_bocu1(self: &Self) -> EncodeIter<Self::IT>;
}

impl<'a> EncodeBOCU1 for &'a str {
    type IT = ::std::str::Chars<'a>;
    fn encode_bocu1(self: &Self) -> EncodeIter<Self::IT> {
        let inner = EncodedChunkIter::new(self.chars());
        DrainEncodedChunkIter::new(inner)
    }
}
impl<'a> EncodeBOCU1 for &'a [char] {
    type IT = ::std::iter::Cloned<::std::slice::Iter<'a, char>>;
    fn encode_bocu1(self: &Self) -> EncodeIter<Self::IT> {
        let inner = EncodedChunkIter::new(self.iter().cloned());
        DrainEncodedChunkIter::new(inner)
    }
}

impl<'a> EncodeBOCU1 for ::std::slice::Iter<'a, char> {
    type IT = ::std::iter::Cloned<Self>;
    fn encode_bocu1(self: &Self) -> EncodeIter<Self::IT> {
        let inner = EncodedChunkIter::new(self.clone().cloned());
        DrainEncodedChunkIter::new(inner)
    }
}

pub fn write_encoded_chars<W>(s: &str, out: &mut W) -> io::Result<usize>
where
    W: io::Write,
{
    let mut total = 0;
    let mut e = delta_encoding::DeltaCoder::new();
    for c in s.chars() {
        let enc = e.encode_char(c);
        total += out.write(enc.as_slice())?;
    }
    Ok(total)
}

// The most straightforward way to decode is just to call .decode_bocu1()
// on the encoded bytes and collect the resulting characters. It will only
// return the error-free prefix though; if you want a more-detailed view
// that accounts for errors, you need to use DecodeResultIter.

pub enum DecodeError {
    TruncatedInput,
    TrailByteOutOfRange(u8),
    CharDeltaOutOfRange(char, i32),
}

pub struct DecodeIter<'a> {
    inner: DecodeResultIter<'a>,
}

impl<'a> DecodeIter<'a> {
    pub fn new(s: &'a [u8]) -> DecodeIter<'a> {
        DecodeIter {
            inner: DecodeResultIter::new(s),
        }
    }
}

impl<'a> Iterator for DecodeIter<'a> {
    type Item = char;
    fn next(self: &mut Self) -> Option<char> {
        match self.inner.next() {
            None | Some(Err(_)) => None,
            Some(Ok(c)) => Some(c),
        }
    }
}

pub trait DecodeBOCU1 {
    fn decode_bocu1(self: &Self) -> DecodeIter;
}

impl<'a> DecodeBOCU1 for &'a [u8] {
    fn decode_bocu1(self: &Self) -> DecodeIter {
        DecodeIter::new(self)
    }
}

pub struct DecodeResultIter<'a> {
    state: delta_encoding::DeltaCoder,
    slice: &'a [u8],
}

impl<'a> DecodeResultIter<'a> {
    pub fn new(s: &'a [u8]) -> DecodeResultIter<'a> {
        DecodeResultIter {
            state: delta_encoding::DeltaCoder::new(),
            slice: s,
        }
    }
}

impl<'a> Iterator for DecodeResultIter<'a> {
    type Item = Result<char, DecodeError>;
    fn next(self: &mut Self) -> Option<Result<char, DecodeError>> {
        loop {
            if self.slice.is_empty() {
                return None;
            }
            match self.state.decode_char(self.slice) {
                Ok((None, rest)) => self.slice = rest,
                Ok((Some(c), rest)) => {
                    self.slice = rest;
                    return Some(Ok(c));
                }
                Err(e) => {
                    return Some(Err(e));
                }
            }
        }
    }
}
