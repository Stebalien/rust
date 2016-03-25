// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Character manipulation.
//!
//! For more details, see ::rustc_unicode::char (a.k.a. std::char)

#![allow(non_snake_case)]
#![stable(feature = "core_char", since = "1.2.0")]

use str;
use ops::Deref;
use iter::Iterator;
use mem::transmute;
use option::Option::{None, Some};
use option::Option;
use slice::SliceExt;

// UTF-8 ranges and tags for encoding characters
const TAG_CONT: u8    = 0b1000_0000;
const TAG_TWO_B: u8   = 0b1100_0000;
const TAG_THREE_B: u8 = 0b1110_0000;
const TAG_FOUR_B: u8  = 0b1111_0000;
const MAX_ONE_B: u32   =     0x80;
const MAX_TWO_B: u32   =    0x800;
const MAX_THREE_B: u32 =  0x10000;

/*
    Lu  Uppercase_Letter        an uppercase letter
    Ll  Lowercase_Letter        a lowercase letter
    Lt  Titlecase_Letter        a digraphic character, with first part uppercase
    Lm  Modifier_Letter         a modifier letter
    Lo  Other_Letter            other letters, including syllables and ideographs
    Mn  Nonspacing_Mark         a nonspacing combining mark (zero advance width)
    Mc  Spacing_Mark            a spacing combining mark (positive advance width)
    Me  Enclosing_Mark          an enclosing combining mark
    Nd  Decimal_Number          a decimal digit
    Nl  Letter_Number           a letterlike numeric character
    No  Other_Number            a numeric character of other type
    Pc  Connector_Punctuation   a connecting punctuation mark, like a tie
    Pd  Dash_Punctuation        a dash or hyphen punctuation mark
    Ps  Open_Punctuation        an opening punctuation mark (of a pair)
    Pe  Close_Punctuation       a closing punctuation mark (of a pair)
    Pi  Initial_Punctuation     an initial quotation mark
    Pf  Final_Punctuation       a final quotation mark
    Po  Other_Punctuation       a punctuation mark of other type
    Sm  Math_Symbol             a symbol of primarily mathematical use
    Sc  Currency_Symbol         a currency sign
    Sk  Modifier_Symbol         a non-letterlike modifier symbol
    So  Other_Symbol            a symbol of other type
    Zs  Space_Separator         a space character (of various non-zero widths)
    Zl  Line_Separator          U+2028 LINE SEPARATOR only
    Zp  Paragraph_Separator     U+2029 PARAGRAPH SEPARATOR only
    Cc  Control                 a C0 or C1 control code
    Cf  Format                  a format control character
    Cs  Surrogate               a surrogate code point
    Co  Private_Use             a private-use character
    Cn  Unassigned              a reserved unassigned code point or a noncharacter
*/

/// The highest valid code point a `char` can have.
///
/// A [`char`] is a [Unicode Scalar Value], which means that it is a [Code
/// Point], but only ones within a certain range. `MAX` is the highest valid
/// code point that's a valid [Unicode Scalar Value].
///
/// [`char`]: ../../std/primitive.char.html
/// [Unicode Scalar Value]: http://www.unicode.org/glossary/#unicode_scalar_value
/// [Code Point]: http://www.unicode.org/glossary/#code_point
#[stable(feature = "rust1", since = "1.0.0")]
pub const MAX: char = '\u{10ffff}';

/// Converts a `u32` to a `char`.
///
/// Note that all [`char`]s are valid [`u32`]s, and can be casted to one with
/// [`as`]:
///
/// ```
/// let c = '💯';
/// let i = c as u32;
///
/// assert_eq!(128175, i);
/// ```
///
/// However, the reverse is not true: not all valid [`u32`]s are valid
/// [`char`]s. `from_u32()` will return `None` if the input is not a valid value
/// for a [`char`].
///
/// [`char`]: ../../std/primitive.char.html
/// [`u32`]: ../../std/primitive.u32.html
/// [`as`]: ../../book/casting-between-types.html#as
///
/// For an unsafe version of this function which ignores these checks, see
/// [`from_u32_unchecked()`].
///
/// [`from_u32_unchecked()`]: fn.from_u32_unchecked.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::char;
///
/// let c = char::from_u32(0x2764);
///
/// assert_eq!(Some('❤'), c);
/// ```
///
/// Returning `None` when the input is not a valid [`char`]:
///
/// ```
/// use std::char;
///
/// let c = char::from_u32(0x110000);
///
/// assert_eq!(None, c);
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub fn from_u32(i: u32) -> Option<char> {
    // catch out-of-bounds and surrogates
    if (i > MAX as u32) || (i >= 0xD800 && i <= 0xDFFF) {
        None
    } else {
        Some(unsafe { from_u32_unchecked(i) })
    }
}

/// Converts a `u32` to a `char`, ignoring validity.
///
/// Note that all [`char`]s are valid [`u32`]s, and can be casted to one with
/// [`as`]:
///
/// ```
/// let c = '💯';
/// let i = c as u32;
///
/// assert_eq!(128175, i);
/// ```
///
/// However, the reverse is not true: not all valid [`u32`]s are valid
/// [`char`]s. `from_u32_unchecked()` will ignore this, and blindly cast to
/// [`char`], possibly creating an invalid one.
///
/// [`char`]: ../../std/primitive.char.html
/// [`u32`]: ../../std/primitive.u32.html
/// [`as`]: ../../book/casting-between-types.html#as
///
/// # Safety
///
/// This function is unsafe, as it may construct invalid `char` values.
///
/// For a safe version of this function, see the [`from_u32()`] function.
///
/// [`from_u32()`]: fn.from_u32.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::char;
///
/// let c = unsafe { char::from_u32_unchecked(0x2764) };
///
/// assert_eq!('❤', c);
/// ```
#[inline]
#[stable(feature = "char_from_unchecked", since = "1.5.0")]
pub unsafe fn from_u32_unchecked(i: u32) -> char {
    transmute(i)
}

/// Converts a digit in the given radix to a `char`.
///
/// A 'radix' here is sometimes also called a 'base'. A radix of two
/// indicates a binary number, a radix of ten, decimal, and a radix of
/// sixteen, hexadecimal, to give some common values. Arbitrary
/// radicum are supported.
///
/// `from_digit()` will return `None` if the input is not a digit in
/// the given radix.
///
/// # Panics
///
/// Panics if given a radix larger than 36.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::char;
///
/// let c = char::from_digit(4, 10);
///
/// assert_eq!(Some('4'), c);
///
/// // Decimal 11 is a single digit in base 16
/// let c = char::from_digit(11, 16);
///
/// assert_eq!(Some('b'), c);
/// ```
///
/// Returning `None` when the input is not a digit:
///
/// ```
/// use std::char;
///
/// let c = char::from_digit(20, 10);
///
/// assert_eq!(None, c);
/// ```
///
/// Passing a large radix, causing a panic:
///
/// ```
/// use std::thread;
/// use std::char;
///
/// let result = thread::spawn(|| {
///     // this panics
///     let c = char::from_digit(1, 37);
/// }).join();
///
/// assert!(result.is_err());
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub fn from_digit(num: u32, radix: u32) -> Option<char> {
    if radix > 36 {
        panic!("from_digit: radix is too high (maximum 36)");
    }
    if num < radix {
        let num = num as u8;
        if num < 10 {
            Some((b'0' + num) as char)
        } else {
            Some((b'a' + num - 10) as char)
        }
    } else {
        None
    }
}

// NB: the stabilization and documentation for this trait is in
// unicode/char.rs, not here
#[allow(missing_docs)] // docs in libunicode/u_char.rs
#[doc(hidden)]
#[unstable(feature = "core_char_ext",
           reason = "the stable interface is `impl char` in later crate",
           issue = "32110")]
pub trait CharExt {
    #[stable(feature = "core", since = "1.6.0")]
    fn is_digit(self, radix: u32) -> bool;
    #[stable(feature = "core", since = "1.6.0")]
    fn to_digit(self, radix: u32) -> Option<u32>;
    #[stable(feature = "core", since = "1.6.0")]
    fn escape_unicode(self) -> EscapeUnicode;
    #[stable(feature = "core", since = "1.6.0")]
    fn escape_default(self) -> EscapeDefault;
    #[stable(feature = "core", since = "1.6.0")]
    fn len_utf8(self) -> usize;
    #[stable(feature = "core", since = "1.6.0")]
    fn len_utf16(self) -> usize;
    #[unstable(feature = "unicode", issue = "27784")]
    fn encode_utf8(self) -> Utf8Char;
    #[unstable(feature = "unicode", issue = "27784")]
    fn encode_utf16(self) -> Utf16Char;
}

#[stable(feature = "core", since = "1.6.0")]
impl CharExt for char {
    #[inline]
    fn is_digit(self, radix: u32) -> bool {
        self.to_digit(radix).is_some()
    }

    #[inline]
    fn to_digit(self, radix: u32) -> Option<u32> {
        if radix > 36 {
            panic!("to_digit: radix is too high (maximum 36)");
        }
        let val = match self {
          '0' ... '9' => self as u32 - '0' as u32,
          'a' ... 'z' => self as u32 - 'a' as u32 + 10,
          'A' ... 'Z' => self as u32 - 'A' as u32 + 10,
          _ => return None,
        };
        if val < radix { Some(val) }
        else { None }
    }

    #[inline]
    fn escape_unicode(self) -> EscapeUnicode {
        EscapeUnicode { c: self, state: EscapeUnicodeState::Backslash }
    }

    #[inline]
    fn escape_default(self) -> EscapeDefault {
        let init_state = match self {
            '\t' => EscapeDefaultState::Backslash('t'),
            '\r' => EscapeDefaultState::Backslash('r'),
            '\n' => EscapeDefaultState::Backslash('n'),
            '\\' | '\'' | '"' => EscapeDefaultState::Backslash(self),
            '\x20' ... '\x7e' => EscapeDefaultState::Char(self),
            _ => EscapeDefaultState::Unicode(self.escape_unicode())
        };
        EscapeDefault { state: init_state }
    }

    #[inline]
    fn len_utf8(self) -> usize {
        let code = self as u32;
        if code < MAX_ONE_B {
            1
        } else if code < MAX_TWO_B {
            2
        } else if code < MAX_THREE_B {
            3
        } else {
            4
        }
    }

    #[inline]
    fn len_utf16(self) -> usize {
        let ch = self as u32;
        if (ch & 0xFFFF) == ch { 1 } else { 2 }
    }

    #[inline]
    fn encode_utf8(self) -> Utf8Char {
        let code = self as u32;
        let mut buf = [0; 4];
        let len = if code < MAX_ONE_B {
            buf[0] = code as u8;
            1
        } else if code < MAX_TWO_B {
            buf[0] = (code >> 6 & 0x1F) as u8 | TAG_TWO_B;
            buf[1] = (code & 0x3F) as u8 | TAG_CONT;
            2
        } else if code < MAX_THREE_B {
            buf[0] = (code >> 12 & 0x0F) as u8 | TAG_THREE_B;
            buf[1] = (code >>  6 & 0x3F) as u8 | TAG_CONT;
            buf[2] = (code & 0x3F) as u8 | TAG_CONT;
            3
        } else {
            buf[0] = (code >> 18 & 0x07) as u8 | TAG_FOUR_B;
            buf[1] = (code >> 12 & 0x3F) as u8 | TAG_CONT;
            buf[2] = (code >>  6 & 0x3F) as u8 | TAG_CONT;
            buf[3] = (code & 0x3F) as u8 | TAG_CONT;
            4
        };
        Utf8Char { buf: buf, len: len }
    }

    #[inline]
    fn encode_utf16(self) -> Utf16Char {
        let mut buf = [0; 2];
        let mut code = self as u32;
        let len = if (code & 0xFFFF) == code {
            // The BMP falls through (assuming non-surrogate, as it should)
            buf[0] = code as u16;
            1
        } else {
            // Supplementary planes break into surrogates.
            code -= 0x1_0000;
            buf[0] = 0xD800 | ((code >> 10) as u16);
            buf[1] = 0xDC00 | ((code as u16) & 0x3FF);
            2
        };
        Utf16Char { buf: buf, len: len }
    }
}

/// Returns an iterator that yields the hexadecimal Unicode escape of a
/// character, as `char`s.
///
/// This `struct` is created by the [`escape_unicode()`] method on [`char`]. See
/// its documentation for more.
///
/// [`escape_unicode()`]: ../../std/primitive.char.html#method.escape_unicode
/// [`char`]: ../../std/primitive.char.html
#[derive(Clone, Debug)]
#[stable(feature = "rust1", since = "1.0.0")]
pub struct EscapeUnicode {
    c: char,
    state: EscapeUnicodeState
}

#[derive(Clone, Debug)]
enum EscapeUnicodeState {
    Backslash,
    Type,
    LeftBrace,
    Value(usize),
    RightBrace,
    Done,
}

#[stable(feature = "rust1", since = "1.0.0")]
impl Iterator for EscapeUnicode {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeUnicodeState::Backslash => {
                self.state = EscapeUnicodeState::Type;
                Some('\\')
            }
            EscapeUnicodeState::Type => {
                self.state = EscapeUnicodeState::LeftBrace;
                Some('u')
            }
            EscapeUnicodeState::LeftBrace => {
                let mut n = 0;
                while (self.c as u32) >> (4 * (n + 1)) != 0 {
                    n += 1;
                }
                self.state = EscapeUnicodeState::Value(n);
                Some('{')
            }
            EscapeUnicodeState::Value(offset) => {
                let c = from_digit(((self.c as u32) >> (offset * 4)) & 0xf, 16).unwrap();
                if offset == 0 {
                    self.state = EscapeUnicodeState::RightBrace;
                } else {
                    self.state = EscapeUnicodeState::Value(offset - 1);
                }
                Some(c)
            }
            EscapeUnicodeState::RightBrace => {
                self.state = EscapeUnicodeState::Done;
                Some('}')
            }
            EscapeUnicodeState::Done => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut n = 0;
        while (self.c as usize) >> (4 * (n + 1)) != 0 {
            n += 1;
        }
        let n = match self.state {
            EscapeUnicodeState::Backslash => n + 5,
            EscapeUnicodeState::Type => n + 4,
            EscapeUnicodeState::LeftBrace => n + 3,
            EscapeUnicodeState::Value(offset) => offset + 2,
            EscapeUnicodeState::RightBrace => 1,
            EscapeUnicodeState::Done => 0,
        };
        (n, Some(n))
    }
}

/// An iterator that yields the literal escape code of a `char`.
///
/// This `struct` is created by the [`escape_default()`] method on [`char`]. See
/// its documentation for more.
///
/// [`escape_default()`]: ../../std/primitive.char.html#method.escape_default
/// [`char`]: ../../std/primitive.char.html
#[derive(Clone, Debug)]
#[stable(feature = "rust1", since = "1.0.0")]
pub struct EscapeDefault {
    state: EscapeDefaultState
}

#[derive(Clone, Debug)]
enum EscapeDefaultState {
    Backslash(char),
    Char(char),
    Done,
    Unicode(EscapeUnicode),
}

#[stable(feature = "rust1", since = "1.0.0")]
impl Iterator for EscapeDefault {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match self.state {
            EscapeDefaultState::Backslash(c) => {
                self.state = EscapeDefaultState::Char(c);
                Some('\\')
            }
            EscapeDefaultState::Char(c) => {
                self.state = EscapeDefaultState::Done;
                Some(c)
            }
            EscapeDefaultState::Done => None,
            EscapeDefaultState::Unicode(ref mut iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.state {
            EscapeDefaultState::Char(_) => (1, Some(1)),
            EscapeDefaultState::Backslash(_) => (2, Some(2)),
            EscapeDefaultState::Unicode(ref iter) => iter.size_hint(),
            EscapeDefaultState::Done => (0, Some(0)),
        }
    }

    fn count(self) -> usize {
        match self.state {
            EscapeDefaultState::Char(_) => 1,
            EscapeDefaultState::Unicode(iter) => iter.count(),
            EscapeDefaultState::Done => 0,
            EscapeDefaultState::Backslash(_) => 2,
        }
    }

    fn nth(&mut self, n: usize) -> Option<char> {
        match self.state {
            EscapeDefaultState::Backslash(c) if n == 0 => {
                self.state = EscapeDefaultState::Char(c);
                Some('\\')
            },
            EscapeDefaultState::Backslash(c) if n == 1 => {
                self.state = EscapeDefaultState::Done;
                Some(c)
            },
            EscapeDefaultState::Backslash(_) => {
                self.state = EscapeDefaultState::Done;
                None
            },
            EscapeDefaultState::Char(c) => {
                self.state = EscapeDefaultState::Done;

                if n == 0 {
                    Some(c)
                } else {
                    None
                }
            },
            EscapeDefaultState::Done => return None,
            EscapeDefaultState::Unicode(ref mut i) => return i.nth(n),
        }
    }

    fn last(self) -> Option<char> {
        match self.state {
            EscapeDefaultState::Unicode(iter) => iter.last(),
            EscapeDefaultState::Done => None,
            EscapeDefaultState::Backslash(c) | EscapeDefaultState::Char(c) => Some(c),
        }
    }
}

/// A container that derefs to an `str` representing the UTF-8 encoding of a
/// `char` value.
///
/// Constructed via the `.encode_utf8()` method on `char`.
#[unstable(feature = "unicode", issue = "27784")]
#[derive(Debug, Copy, Clone)]
pub struct Utf8Char {
    buf: [u8; 4],
    len: usize,
}

#[unstable(feature = "unicode", issue = "27784")]
impl Deref for Utf8Char {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_ref()
    }
}

#[unstable(feature = "unicode", issue = "27784")]
impl AsRef<str> for Utf8Char {
    fn as_ref(&self) -> &str {
        unsafe {
            str::from_utf8_unchecked(self.as_ref())
        }
    }
}

#[unstable(feature = "unicode", issue = "27784")]
impl AsRef<[u8]> for Utf8Char {
    fn as_ref(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

/// A container that derefs to a slice of `u16` entries representing the UTF-16
/// encoding of a `char` value.
///
/// Constructed via the `.encode_utf16()` method on `char`.
#[unstable(feature = "unicode", issue = "27784")]
#[derive(Debug, Copy, Clone)]
pub struct Utf16Char {
    buf: [u16; 2],
    len: usize,
}

#[unstable(feature = "unicode", issue = "27784")]
impl Deref for Utf16Char {
    type Target = [u16];
    fn deref(&self) -> &[u16] {
        self.as_ref()
    }
}

#[unstable(feature = "unicode", issue = "27784")]
impl AsRef<[u16]> for Utf16Char {
    fn as_ref(&self) -> &[u16] {
        &self.buf[..self.len]
    }
}
