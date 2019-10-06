//! Part 1: delta encoding
//! ======================
//!
//! Normal stateful delta-encoding involves emitting the sequence of differences
//! between the previous and current values; in BOCU-1 this is slightly modified
//! to _normalize_ each previous value, snapping it to a point that is near but
//! not identical to the actual previous value.
//!
//! The point of this is to embed a little bit of predictive knowledge in the
//! encoding: namely if the current value is in a given script-specific block of
//! Unicode's value space, the next value will very likely also be in the same
//! block. Words don't typically change script blocks in the middle of the word.
//!
//! Within each script-specific block, BOCU-1 doesn't use a statistical model
//! for which values are more likely or anything, so its best guess -- to make
//! the intra-block deltas small -- is to snap to the middle of the code range
//! in the block. In practice (and by design) this means that _many_ sequences
//! that stick to a single script can be coded as a single "big delta" jump into
//! the code block followed by a sequence of single-byte deltas, each measured
//! from the middle of the block.
//!
//! There are a couple exceptions to this logic, see the outer loop below in
//! encode_char which does the delta computation.

use crate::*;

/// Normalize a character (the previous character when delta-coding) to the
/// middle of a script-specific block.
pub fn normalized_prev(curr: char) -> char {
    match curr {
        // Hiragana
        '\u{3040}'..='\u{309F}' => '\u{3070}',

        // Unihan
        '\u{4E00}'..='\u{9FA5}' => '\u{7711}',

        // Hangul
        '\u{AC00}'..='\u{D7A3}' => '\u{C1D1}',

        // Other "small scripts" are handled by the observation that most are
        // situated at multiples of 128 in the Unicode space, so a decent guess
        // is to snap to the previous such boundary plus half-a-block, or 64
        // (0x40) values.
        _ => {
            let cu32 = curr as u32;
            let guess_curr_block_start = cu32 & 0xffff_ff80_u32;
            let guess_curr_block_middle = guess_curr_block_start + 0x40;
            let opt = ::std::char::from_u32(guess_curr_block_middle);
            opt.expect("bug in BOCU1Encoder::normalized_prev")
        }
    }
}

pub struct DeltaCoder {
    prev: char,
}

const INITIAL_PREVIOUS_STATE: char = '\u{40}';
const ASCII_SP: char = '\u{20}';

#[allow(clippy::new_without_default_derive)]
impl DeltaCoder {
    pub fn new() -> Self {
        Self {
            prev: INITIAL_PREVIOUS_STATE,
        }
    }

    /// For the most part, this is a simple delta encoder that just emits the
    /// stream of pairwise differences between characters.
    ///
    /// There are three exceptions to this view (in addition to the downstream
    /// variable-length and byte-avoidance encodings):
    ///
    ///   1. The previous values are normalized, see above in normalized_prev.
    ///
    ///   2. To avoid jumping down to ASCII block and back up to some other
    ///      script block on every space between words, the ASCII SP character
    ///      0x20 is both encoded as itself and does _not_ modify the
    ///      previous-value state.
    ///
    ///   3. To self-synchronize the stream and allow compatible forms of
    ///      paging, line breaking, terminal control and so forth, all the ASCII
    ///      control characters below 0x20 are encoded as themselves _and_ reset
    ///      the previous-value state to its initial state, 0x40.
    ///
    #[inline]
    pub fn encode_char(self: &mut Self, curr: char) -> EncodedChunk {
        trace!("DeltaCoder: char 0x{:x}", curr as u32);
        if curr <= ASCII_SP {
            if curr != ASCII_SP {
                trace!("DeltaCoder: reset prev to 0x40");
                self.prev = INITIAL_PREVIOUS_STATE;
            }
            trace!("DeltaCoder: self-encoding 0x{:x}", curr as u32);
            EncodedChunk::new_single(curr as u8)
        } else {
            let delta: i32 = (curr as i32) - (self.prev as i32);
            self.prev = normalized_prev(curr);
            trace!("DeltaCoder: set prev to 0x{:x}", self.prev as u32);
            trace!("DeltaCoder: encoding delta {}", delta);
            variable_length_code::encode_delta(delta)
        }
    }

    /// The decoder is just the inverse of the above, with some error handling
    /// for malformed inputs.
    #[allow(clippy::cast_sign_loss)]
    pub fn decode_char<'a>(
        self: &mut Self,
        b: &'a [u8],
    ) -> Result<(Option<char>, &'a [u8]), DecodeError> {
        assert!(!b.is_empty());
        let init = b[0];
        if init == variable_length_code::LEAD_BYTE_RESET {
            Ok((None, &b[1..]))
        } else if init <= variable_length_code::LEAD_BYTE_ASCII_SP {
            if init != variable_length_code::LEAD_BYTE_ASCII_SP {
                self.prev = INITIAL_PREVIOUS_STATE;
            }
            Ok((Some(init as char), &b[1..]))
        } else {
            let (delta, rest) = variable_length_code::decode_delta(b)?;
            let candidate = (self.prev as i32) + delta;
            let c = ::std::char::from_u32(candidate as u32);
            match c {
                None => Err(DecodeError::CharDeltaOutOfRange(self.prev, delta)),
                Some(ch) => {
                    self.prev = normalized_prev(ch);
                    Ok((Some(ch), rest))
                }
            }
        }
    }
}
