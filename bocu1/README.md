# bocu1

 This crate serves two purposes:

   1. To provide a usable implementation of BOCU-1 to people who are looking
      for one, as well as some utilities related to working directly with
      BOCU-1 strings and "small strings" packed into 64 or 128-bit integers.

   2. To serve as a well-documented elucidation / reference for people
      trying to piece together how the encoding even works, from the
      multiple, partial bits of documentation that exist elsewhere online.

 It thus has more redundancies and explanations than necessary, and has been
 "split apart" into as many layers as possible, in order to maximize
 clarity, motives, design choices, etc. It is probably too slow as a result.
 But it's not a fast encoding in any case: the speed win comes from encoding
 text once and working with it in its encoded (smaller!) form.

 I've also included a bunch of test vectors and a randomizing validator for
 it connected to IBM's previous reference implementation.


 What even is BOCU-1
 ===================

 [BOCU-1](https://en.wikipedia.org/wiki/Binary_Ordered_Compression_for_Unicode)
 is a relatively nice Unicode encoding, like UTF-8 or the like, with
 somewhat different tradeoffs. It is not useful for all (or even many) cases
 and in general most applications should lean on UTF-8, but BOCU-1 gets you
 a few things that you might want in certain contexts:

   1. Generally small. The 'C' in 'BOCU-1' stands for compression, and the
      principal way to look at this encoding is as a very simple
      text-focused compression format. BOCU-1 strings are smaller than those
      in most other common Unicode encodings. In particular it is small
      enough to be quite useful for storing small strings "packed" in
      multibyte machine words (64 or 128 bits).

   2. Minimizes linguistic penalties. Latin-script languages are not encoded
      significantly more efficiently than non-Latin. Indeed, most scripts
      get an encoding nearly as good as they got in the pre-Unicode "code
      page" days.

   3. Minimal fixed compression overhead. The compression kicks in on the
      second character encoded, without any dictionaries or encoded
      metadata.

   4. Codepoint-order preserving. You can memcmp() BOCU-1 strings, or
      integer-compare them if small strings are packed into integers, and
      the comparison will obey the lexicographic Unicode codepoint order of
      the strings. This is probably the main use of the form: you can build
      a really fast ordered dictionary type or database key range using
      BOCU-1 small strings packed into integers (if you're ok with codepoint
      order).

   5. Reasonably (though not perfectly) compatible with existing contexts:
      self-synchronizing at line boundaries, avoids ASCII control characters
      that would mess up a terminal, permits some crude forms of byte-level
      word and line breaking, etc.

   6. Small, simple, deterministic code. Single representation for each
      input.

 It has several downsides of course, which should be admitted up front:

   1. Stateful encoding. Period. The encoder state resets quite often, but
      you can't decode a single Unicode scalar inside a run without decoding
      from the most recent reset-point. This means memchr-like byte-based
      string-search is somewhat compromised (though you can byte-search for
      candidates sharing the second-and-beyond characters in a query, and
      then fully decode and filter those candidates, so it's not completely
      hopeless).

   2. Less CPU-efficient than UTF-8. It uses integer divide and modulus
      operations rather than just bitwise operations.

   3. Less compatible. It reuses many printable ASCII codes for other
      things, and does not code most values in the ASCII space as themselves
      (aside from the C0 control characters <= 0x20).

   4. Formerly patented in the US (US patent 6737994, filed 2002, expired
      November 2022). I actually wrote this crate 4 years before publishing
      it, tried to contact IBM to get the promised "royalty-free license"
      they were going to offer conforming implementations, found out that it
      was sold to Google in 2011, contacted them and heard back they would
      be willing to include it in their "open patent non-assertion pledge",
      but then they stopped responding to emails, the trail went cold and
      nothing ever happened so I just waited. The patent is now expired, so
      I think I'm in the clear, but that is why this package is MIT licensed
      rather than MIT+ASL2: I have no idea what the ASL2 patent clauses mean
      with respect to a formerly-patented thing.


 Encoding overview
 =================

 BOCU-1 consists of 3 parts:

   1. A stateful delta-encoder with a previous-char value that is normalized
     by some script- and block-specific rules.

   2. A variable-length code (1..4 code values) for the deltas generated in
     part 1, that is both lexical-order and delta-magnitude preserving. This
     code has separately-treated leading and trailing values.

   3. A final mapping from each trailing code value in the variable-length
     code of part 2 to output bytes, using a small set of range-offsets that
     thereby avoids bytes used for common ASCII control codes, provides
     synchronization bytes, and allows codepoints below 0x20 to be
     self-encoded.

 Part of what makes existing documentation and code for BOCU-1 look complex
 is that these three parts appear quite intertwined (for performance), when
 in fact they can be understood almost completely in isolation. This crate
 is therefore organized into 3 main sub-modules (plus some helpers), one for
 each such part, exposing only the bits that need to be shared between them.


License: MIT
