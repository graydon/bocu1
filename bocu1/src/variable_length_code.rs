//! Part 2: variable-length code
//! ============================
//!
//! The second phase of BOCU-1 maps the linear space of deltas into
//! variable-length small-value ("almost byte") sequences. These coding
//! sequences are chosen to achieve two goals simultaneously.
//!
//!  1. They preserve the order of deltas in lexicographical binary order of
//!     their bytes. In other words: given two i32 deltas d1 and d2, with d1 <
//!     d2, we get encoded(d1) as less-than encoded(d2) in the memcmp()
//!     lexicographic order.
//!
//!  2. They reflect (as much as possible) the absolute value of the delta in
//!     terms of the length of the value sequence. In other words: given two i32
//!     deltas d1 and d2, with abs(d1) <= abs(d2), we get len(encoded(d1)) <=
//!     len(encoded(d2)). In particular: small deltas within an encoding block
//!     will tend to be single value encodings in alphabetic scripts, or two
//!     values in logosyllabic scripts.
//!
//! The encode_delta and decode_delta functions in this module are driven by
//! some seemingly magic values which map specific delta ranges to leading bytes
//! (and code lengths). The magic values are actually derived quite
//! systematically, but their rationale is subtle.
//!
//! Choice of leading bytes
//! -----------------------
//!
//! Note that each variable-length code starts with a leading byte, and these
//! leading bytes are chosen to _not_ collide with the ASCII C0 control range
//! either -- just like the trailing bytes -- so they range from 0x21 .. 0xFE
//! (avoiding 0xFF -- see LEAD_BYTE_RESET). So that means there are 0xFE - 0x20
//! = 222 leading bytes to go around.
//!
//! The intent is to allocate them to deltas in such a way that 128 of them will
//! be used for single-byte encoding of small deltas (+/- 64) within a small
//! script block. Assigning them _outwards from the middle_ (middle=0x90) of the
//! range available we therefore assign those 128 deltas to the leading bytes
//! 0x50..0xD0, a.k.a. 0x90-0x40 .. 0x90+0x40. In those codes there is no
//! trailing byte, just a leading byte that indicates a delta entirely.
//!
//! After the 128 single-byte codes starting in the middle of the leading-byte
//! range, there are a set of 43 positive and 43 negative codes that are leading
//! bytes of 2-byte sequences. Why 43? This threshold was chosen such that
//! deltas within the relatively large main block of "Unihan 1.0.1" from
//! U+4E00..U+9FA5 (a range of 20,901) fits in a 2-byte code. Since there are
//! 243 possible _trailing_ bytes (after accounting for excluded trailing byte
//! values), each 2-byte code will be one leading and one trailing byte, so with
//! 43 leading codes the positive and negative two-byte codes with can each
//! encode 43 * 243 = 10,449 delta values, which (added to the middle 128) gives
//! us 10,449 * 2 + 128 = 21,026 deltas: just enough for Unihan 1.0.1.
//!
//! The remaining leading bytes were allocated to favor the 3-byte codes (3
//! positive and 3 negative leading bytes, coding 354,294 more deltas), with
//! only one positive and one negative remaining leading byte for the 4-byte
//! codes (coding the remaining 28,697,814 possible large deltas, which is far
//! more than needed to jump across the whole Unicode range of 1,114,111
//! values).

const N_LEAD_BYTES_1: i32 = 64;
const N_LEAD_BYTES_2: i32 = 43;
const N_LEAD_BYTES_3: i32 = 3;

const LO_1BYTE_DELTA: i32 = -N_LEAD_BYTES_1;
const HI_1BYTE_DELTA: i32 = N_LEAD_BYTES_1 - 1;
const_assert_eq!(assert_L1D; LO_1BYTE_DELTA, -0x0000_0040);
const_assert_eq!(assert_H1D; HI_1BYTE_DELTA,  0x0000_003F);

const RANGE_2BYTE: i32 = N_LEAD_BYTES_2 * N_TRAIL_VALUES;
const LO_2BYTE_DELTA: i32 = LO_1BYTE_DELTA - RANGE_2BYTE;
const HI_2BYTE_DELTA: i32 = HI_1BYTE_DELTA + RANGE_2BYTE;
const_assert_eq!(assert_L2D; LO_2BYTE_DELTA, -0x0000_2911);
const_assert_eq!(assert_H2D; HI_2BYTE_DELTA,  0x0000_2910);

const RANGE_3BYTE: i32 = N_LEAD_BYTES_3 * N_TRAIL_VALUES * N_TRAIL_VALUES;
const LO_3BYTE_DELTA: i32 = LO_2BYTE_DELTA - RANGE_3BYTE;
const HI_3BYTE_DELTA: i32 = HI_2BYTE_DELTA + RANGE_3BYTE;
const_assert_eq!(assert_L3D; LO_3BYTE_DELTA, -0x0002_DD0C);
const_assert_eq!(assert_H3D; HI_3BYTE_DELTA,  0x0002_DD0B);

use crate::trailing_byte_selection;
use crate::trailing_byte_selection::N_TRAIL_VALUES;
use crate::util::Euc;
use crate::{DecodeError, EncodedChunk};

#[inline]
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
pub fn encode_delta(delta: i32) -> EncodedChunk {
    let (offset, lead, len): (i32, u8, usize) = match delta {
        -0x0010_FF9F..=-0x0002_DD0D => (-0x0002_DD0C, 0x22, 4),
        -0x0002_DD0C..=-0x0000_2912 => (-0x0000_2911, 0x25, 3),
        -0x0000_2911..=-0x0000_0041 => (-0x0000_0040, 0x50, 2),
        -0x0000_0040..=0x0000_003F => (0x0000_0000, 0x90, 1),
        0x0000_0040..=0x0000_2910 => (0x0000_0040, 0xD0, 2),
        0x0000_2911..=0x0002_DD0B => (0x0000_2911, 0xFB, 3),
        0x0002_DD0C..=0x0010_FFBF => (0x0002_DD0C, 0xFE, 4),
        _ => panic!("bug in VariableLengthCode::encode_delta"),
    };
    trace!(
        "VariableLengthCode: delta {} (= 0x{:x}) gets \
         {}-value code, lead byte 0x{:x}",
        delta,
        delta,
        len,
        lead
    );

    // Buffer to store the sequence.
    let mut buf: [u8; 4] = [lead, 0x0, 0x0, 0x0];

    // Value to encode base-243 digits of, in the target window.
    let mut d: i32 = delta - offset;

    // Select the trailing bytes, from least to greatest.
    let divisor: i32 = N_TRAIL_VALUES;
    for i in (1..len).rev() {
        let m: i32 = Euc::mod_euc(d, divisor);
        d = Euc::div_euc(d, divisor);
        trace!(
            "VariableLengthCode: byte {}: adding trail \
             modulus {} to buffer value 0x{:x}",
            i,
            m,
            buf[i]
        );
        assert!(0 <= m && m <= 0xff);
        buf[i] = trailing_byte_selection::trail_to_byte(m as u8);
    }

    // Adjust in the leading byte.
    trace!(
        "VariableLengthCode: byte 0: adding lead \
         divisor {} to buffer lead-byte 0x{:x}",
        d,
        buf[0]
    );
    let init: i32 = i32::from(buf[0]) + d;
    assert!(0 < init && init <= 0xff);
    buf[0] = init as u8;

    trace!(
        "VariableLengthCode: final code for delta {} is {:?}",
        delta,
        &buf[0..len]
    );
    EncodedChunk {
        bytes: buf,
        count: len,
    }
}

// The leading byte 0xFF is reserved as a non-coding delta-state-reset byte
// that applications can inject to get more self-syncronization in the code
// stream, if they're not seeing enough naturally occurring from C0 codes).
pub const LEAD_BYTE_RESET: u8 = 0xff;

// Lead-bytes are supposed to be greater than this bytes; bytes at-or-below
// 0x20 (SP) are self-encoded at an upper level.
pub const LEAD_BYTE_ASCII_SP: u8 = 0x20;

#[inline]
#[allow(clippy::needless_range_loop)] // The loop is not "needless" here!
pub fn decode_delta(b: &[u8]) -> Result<(i32, &[u8]), DecodeError> {
    assert!(!b.is_empty());

    let lead: u8 = b[0];

    // Lead bytes 0xFF or below 0x21 are not deltas and should have been
    // handled in our caller.
    assert!(lead > LEAD_BYTE_ASCII_SP);
    assert!(lead != LEAD_BYTE_RESET);

    let (offset, base, len): (i32, u8, usize) = match lead {
        | 0x21 ..= 0x21 /*   1 code  */ => (-0x0002_DD0C, 0x22, 4),
        | 0x22 ..= 0x24 /*   3 codes */ => (-0x0000_2911, 0x25, 3),
        | 0x25 ..= 0x4F /*  43 codes */ => (-0x0000_0040, 0x50, 2),
        | 0x50 ..= 0xCF /* 128 codes */ => ( 0x0000_0000, 0x90, 1),
        | 0xD0 ..= 0xFA /*  43 codes */ => ( 0x0000_0040, 0xD0, 2),
        | 0xFB ..= 0xFD /*   3 codes */ => ( 0x0000_2911, 0xFB, 3),
        | 0xFE ..= 0xFE /*   1 code  */ => ( 0x0002_DD0C, 0xFE, 4),
        | _ => panic!("bug in VariableLengthCode::decode_delta")
    };

    if b.len() < len {
        return Err(DecodeError::TruncatedInput);
    }

    let mut delta: i32 = i32::from(lead) - i32::from(base);
    for i in 1..len {
        delta *= N_TRAIL_VALUES;
        delta += i32::from(trailing_byte_selection::byte_to_trail(b[i])?);
    }
    delta += offset;
    Ok((delta, &b[len..]))
}
