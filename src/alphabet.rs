//! Provides [Alphabet] and constants for alphabets commonly used in the wild.

use crate::PAD_BYTE;
use core::{convert, fmt};
#[cfg(any(feature = "std", test))]
use std::error;

const ALPHABET_SIZE: usize = 64;

/// An alphabet defines the 64 ASCII characters (symbols) used for base64.
///
/// Common alphabets are provided as constants, and custom alphabets
/// can be made via `from_str` or the `TryFrom<str>` implementation.
///
/// ```
/// let custom = base64::alphabet::Alphabet::from_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/").unwrap();
///
/// let engine = base64::engine::GeneralPurpose::from(
///     &custom,
///     base64::engine::general_purpose::PAD);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Alphabet {
    pub(crate) symbols: [u8; ALPHABET_SIZE],
}

impl Alphabet {
    /// Performs no checks so that it can be const.
    /// Used only for known-valid strings.
    const fn from_str_unchecked(alphabet: &str) -> Self {
        let mut symbols = [0_u8; ALPHABET_SIZE];
        let source_bytes = alphabet.as_bytes();

        // a way to copy that's allowed in const fn
        let mut index = 0;
        while index < ALPHABET_SIZE {
            symbols[index] = source_bytes[index];
            index += 1;
        }

        Self { symbols }
    }

    /// Create an `Alphabet` from a string of 64 unique printable ASCII bytes.
    ///
    /// The `=` byte is not allowed as it is used for padding.
    pub const fn from_str(alphabet: &str) -> Result<Self, ParseAlphabetError> {
        let bytes = alphabet.as_bytes();
        if bytes.len() != ALPHABET_SIZE {
            return Err(ParseAlphabetError::InvalidLength);
        }

        {
            let mut index = 0;
            while index < ALPHABET_SIZE {
                let byte = bytes[index];

                // must be ascii printable. 127 (DEL) is commonly considered printable
                // for some reason but clearly unsuitable for base64.
                if !(byte >= 32_u8 && byte <= 126_u8) {
                    return Err(ParseAlphabetError::UnprintableByte(byte));
                }
                // = is assumed to be padding, so cannot be used as a symbol
                if byte == PAD_BYTE {
                    return Err(ParseAlphabetError::ReservedByte(byte));
                }

                // Check for duplicates while staying within what const allows.
                // It's n^2, but only over 64 hot bytes, and only once, so it's likely in the single digit
                // microsecond range.

                let mut probe_index = 0;
                while probe_index < ALPHABET_SIZE {
                    if probe_index == index {
                        probe_index += 1;
                        continue;
                    }

                    let probe_byte = bytes[probe_index];

                    if byte == probe_byte {
                        return Err(ParseAlphabetError::DuplicatedByte(byte));
                    }

                    probe_index += 1;
                }

                index += 1;
            }
        }

        Ok(Self::from_str_unchecked(alphabet))
    }
}

impl convert::TryFrom<&str> for Alphabet {
    type Error = ParseAlphabetError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

/// Possible errors when constructing an [Alphabet] from a `str`.
#[derive(Debug, Eq, PartialEq)]
pub enum ParseAlphabetError {
    /// Alphabets must be 64 ASCII bytes
    InvalidLength,
    /// All bytes must be unique
    DuplicatedByte(u8),
    /// All bytes must be printable (in the range `[32, 126]`).
    UnprintableByte(u8),
    /// `=` cannot be used
    ReservedByte(u8),
}

impl fmt::Display for ParseAlphabetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => write!(f, "Invalid length - must be 64 bytes"),
            Self::DuplicatedByte(b) => write!(f, "Duplicated byte: {:#04x}", b),
            Self::UnprintableByte(b) => write!(f, "Unprintable byte: {:#04x}", b),
            Self::ReservedByte(b) => write!(f, "Reserved byte: {:#04x}", b),
        }
    }
}

#[cfg(any(feature = "std", test))]
impl error::Error for ParseAlphabetError {}

/// The standard alphabet (uses `+` and `/`).
///
/// See [RFC 3548](https://tools.ietf.org/html/rfc3548#section-3).
pub const STANDARD: Alphabet = Alphabet::from_str_unchecked(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
);

/// The URL safe alphabet (uses `-` and `_`).
///
/// See [RFC 3548](https://tools.ietf.org/html/rfc3548#section-4).
pub const URL_SAFE: Alphabet = Alphabet::from_str_unchecked(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
);

/// The `crypt(3)` alphabet (uses `.` and `/` as the first two values).
///
/// Not standardized, but folk wisdom on the net asserts that this alphabet is what crypt uses.
pub const CRYPT: Alphabet = Alphabet::from_str_unchecked(
    "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
);

/// The bcrypt alphabet.
pub const BCRYPT: Alphabet = Alphabet::from_str_unchecked(
    "./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
);

/// The alphabet used in IMAP-modified UTF-7 (uses `+` and `,`).
///
/// See [RFC 3501](https://tools.ietf.org/html/rfc3501#section-5.1.3)
pub const IMAP_MUTF7: Alphabet = Alphabet::from_str_unchecked(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,",
);

/// The alphabet used in BinHex 4.0 files.
///
/// See [BinHex 4.0 Definition](http://files.stairways.com/other/binhex-40-specs-info.txt)
pub const BIN_HEX: Alphabet = Alphabet::from_str_unchecked(
    "!\"#$%&'()*+,-0123456789@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdehijklmpqr",
);

#[cfg(test)]
mod tests {
    use crate::alphabet::*;
    use std::convert::TryFrom as _;

    #[test]
    fn detects_duplicate_start() {
        assert_eq!(
            ParseAlphabetError::DuplicatedByte(b'A'),
            Alphabet::from_str("AACDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_duplicate_end() {
        assert_eq!(
            ParseAlphabetError::DuplicatedByte(b'/'),
            Alphabet::from_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789//")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_duplicate_middle() {
        assert_eq!(
            ParseAlphabetError::DuplicatedByte(b'Z'),
            Alphabet::from_str("ABCDEFGHIJKLMNOPQRSTUVWXYZZbcdefghijklmnopqrstuvwxyz0123456789+/")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_length() {
        assert_eq!(
            ParseAlphabetError::InvalidLength,
            Alphabet::from_str(
                "xxxxxxxxxABCDEFGHIJKLMNOPQRSTUVWXYZZbcdefghijklmnopqrstuvwxyz0123456789+/",
            )
            .unwrap_err()
        );
    }

    #[test]
    fn detects_padding() {
        assert_eq!(
            ParseAlphabetError::ReservedByte(b'='),
            Alphabet::from_str("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+=")
                .unwrap_err()
        );
    }

    #[test]
    fn detects_unprintable() {
        // form feed
        assert_eq!(
            ParseAlphabetError::UnprintableByte(0xc),
            Alphabet::from_str(
                "\x0cBCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            )
            .unwrap_err()
        );
    }

    #[test]
    fn same_as_unchecked() {
        assert_eq!(
            STANDARD,
            Alphabet::try_from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/")
                .unwrap()
        );
    }
}
