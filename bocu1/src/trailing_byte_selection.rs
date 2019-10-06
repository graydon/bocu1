//! Part 3: trailing-byte selection
//! ===============================
//!
//! This is the third and final phase of encoding, in which the linear range of
//! the _trailing_ values emitted by the variable-length encoder is mapped to a
//! few disjoint spans of encoded byte values.
//!
//! The purpose of this mapping is to avoid emitting bytes (NUL, CR, LF, SP, a
//! few other ASCII control codes) into the output stream that might be
//! meaningful in "text contexts" such as ASCII processing tools or MIME emails,
//! as well as avoiding the DOS EOF byte 0x1A, the ASCII ESC byte 0x1B used for
//! extended terminal control sequences, and the ASCII SP byte 0x20. All of
//! these are "self-encoded" in BOCU-1, meaning the byte values only occur in
//! the output byte stream when the same-numbered unicode scalars were present
//! in the input text.
//!
use crate::DecodeError;

// BOCU-1 avoids using 13 values for trailing bytes in a multibyte code
// unit, leaving 256 - 13 = 243 values.
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cast_possible_truncation)]
pub const N_TRAIL_VALUES: i32 = 256 - (N_EXCLUDED_CODES as i32);
const_assert_eq!(assert0; N_TRAIL_VALUES, 243);

const N_EXCLUDED_CODES: usize = 13;
const EXCLUDED_CODE_BYTES: [u8; N_EXCLUDED_CODES] = [
    // NUL
    0x00, // Various common ASCII C0 control codes (CR, LF, HT, etc).
    0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    // ASCII _SUB_stitute, used as EOF in DOS/Win/OS/2 systems.
    0x1A, // ASCII _ESC_ape, often used for extended control sequences.
    0x1B, // ASCII _SP_ace.
    0x20,
];

/// Dodge the 13 avoided ASCII-encoding-bytes by shifting byte ranges up.
#[inline]
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
pub fn trail_to_byte(b: u8) -> u8 {
    assert!(b < (N_TRAIL_VALUES as u8));
    let v = match b {
        0x00..=0x05 => b + 1,         // NUL
        0x06..=0x0F => b + 1 + 9,     // NUL + C0
        0x10..=0x13 => b + 1 + 9 + 2, // NUL + C0 + SUB/ESC
        _ => b + 1 + 9 + 2 + 1,       // NUL + C0 + SUB/ESC + SP
    };
    trace!(
        "TrailingByteSelection:trail_to_byte(0x{:x}) => 0x{:x}",
        b,
        v
    );
    assert!(!EXCLUDED_CODE_BYTES.contains(&v));
    v
}

/// Inverse of the mapping in trail_to_byte above, returning None for
/// inputs that are outside the output range of trail_to_byte.
#[inline]
pub fn byte_to_trail(b: u8) -> Result<u8, DecodeError> {
    let v = match b {
        0x01..=0x06 => Ok(b - 1),
        0x10..=0x19 => Ok((b - 1) - 9),
        0x1C..=0x1F => Ok(((b - 1) - 9) - 2),
        0x21..=0xFF => Ok((((b - 1) - 9) - 2) - 1),
        _ => Err(DecodeError::TrailByteOutOfRange(b)),
    };
    match v {
        Err(_) => trace!("TrailingByteSelection:byte_to_trail(0x{:x}) => Err", b),
        Ok(x) => trace!(
            "TrailingByteSelection:byte_to_trail(0x{:x}) => 0x{:x}",
            b,
            x
        ),
    }
    v
}
