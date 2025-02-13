#[cfg(any(feature = "alloc", feature = "std", test))]
use alloc::string::String;
use core::fmt;
#[cfg(any(feature = "std", test))]
use std::error;

#[cfg(any(feature = "alloc", feature = "std", test))]
use crate::engine::general_purpose::STANDARD;
use crate::engine::{Config, Engine};
use crate::PAD_BYTE;

/// Encode arbitrary octets as base64 using the [`STANDARD` engine](STANDARD).
///
/// See [Engine::encode].
#[allow(unused)]
#[deprecated(since = "0.21.0", note = "Use Engine::encode")]
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
    STANDARD.encode(input)
}

///Encode arbitrary octets as base64 using the provided `Engine` into a new `String`.
///
/// See [Engine::encode].
#[allow(unused)]
#[deprecated(since = "0.21.0", note = "Use Engine::encode")]
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn encode_engine<E: Engine, T: AsRef<[u8]>>(input: T, engine: &E) -> String {
    engine.encode(input)
}

///Encode arbitrary octets as base64 into a supplied `String`.
///
/// See [Engine::encode_string].
#[allow(unused)]
#[deprecated(since = "0.21.0", note = "Use Engine::encode_string")]
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn encode_engine_string<E: Engine, T: AsRef<[u8]>>(
    input: T,
    output_buf: &mut String,
    engine: &E,
) {
    engine.encode_string(input, output_buf)
}

/// Encode arbitrary octets as base64 into a supplied slice.
///
/// See [Engine::encode_slice].
#[allow(unused)]
#[deprecated(since = "0.21.0", note = "Use Engine::encode_slice")]
pub fn encode_engine_slice<E: Engine, T: AsRef<[u8]>>(
    input: T,
    output_buf: &mut [u8],
    engine: &E,
) -> Result<usize, EncodeSliceError> {
    engine.encode_slice(input, output_buf)
}

/// B64-encode and pad (if configured).
///
/// This helper exists to avoid recalculating encoded_size, which is relatively expensive on short
/// inputs.
///
/// `encoded_size` is the encoded size calculated for `input`.
///
/// `output` must be of size `encoded_size`.
///
/// All bytes in `output` will be written to since it is exactly the size of the output.
pub(crate) fn encode_with_padding<E: Engine + ?Sized>(
    input: &[u8],
    output: &mut [core::mem::MaybeUninit<u8>],
    engine: &E,
    expected_encoded_size: usize,
) {
    debug_assert_eq!(expected_encoded_size, output.len());

    let mut written = engine.internal_encode(input, output);
    let padding = if engine.config().encode_padding() {
        (4 - written % 4) % 4
    } else {
        0
    };
    if padding > 0 {
        debug_assert!(padding <= 2);
        let _ = output[written].write(PAD_BYTE);
        written += padding >> 1;
        let _ = output[written].write(PAD_BYTE);
        written += 1;
    }

    debug_assert_eq!(expected_encoded_size, written);
}

/// Calculate the base64 encoded length for a given input length, optionally including any
/// appropriate padding bytes.
///
/// Returns `None` if the encoded length can't be represented in `usize`. This will happen for
/// input lengths in approximately the top quarter of the range of `usize`.
pub fn encoded_len(bytes_len: usize, padding: bool) -> Option<usize> {
    let rem = bytes_len % 3;

    let complete_input_chunks = bytes_len / 3;
    let complete_chunk_output = complete_input_chunks.checked_mul(4);

    if rem > 0 {
        if padding {
            complete_chunk_output.and_then(|c| c.checked_add(4))
        } else {
            let encoded_rem = match rem {
                1 => 2,
                2 => 3,
                _ => unreachable!("Impossible remainder"),
            };
            complete_chunk_output.and_then(|c| c.checked_add(encoded_rem))
        }
    } else {
        complete_chunk_output
    }
}

/// Errors that can occur while encoding into a slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeSliceError {
    /// The provided slice is too small.
    OutputSliceTooSmall,
}

impl fmt::Display for EncodeSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputSliceTooSmall => write!(f, "Output slice too small"),
        }
    }
}

#[cfg(any(feature = "std", test))]
impl error::Error for EncodeSliceError {
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        alphabet,
        engine::general_purpose::{GeneralPurpose, NO_PAD, STANDARD},
        tests::{assert_encode_sanity, random_config, random_engine},
    };
    use rand::{
        distributions::{Distribution, Uniform},
        Rng, SeedableRng,
    };
    use std::str;

    const URL_SAFE_NO_PAD_ENGINE: GeneralPurpose = GeneralPurpose::new(&alphabet::URL_SAFE, NO_PAD);

    #[test]
    fn encoded_size_correct_standard() {
        assert_encoded_length(0, 0, &STANDARD, true);

        assert_encoded_length(1, 4, &STANDARD, true);
        assert_encoded_length(2, 4, &STANDARD, true);
        assert_encoded_length(3, 4, &STANDARD, true);

        assert_encoded_length(4, 8, &STANDARD, true);
        assert_encoded_length(5, 8, &STANDARD, true);
        assert_encoded_length(6, 8, &STANDARD, true);

        assert_encoded_length(7, 12, &STANDARD, true);
        assert_encoded_length(8, 12, &STANDARD, true);
        assert_encoded_length(9, 12, &STANDARD, true);

        assert_encoded_length(54, 72, &STANDARD, true);

        assert_encoded_length(55, 76, &STANDARD, true);
        assert_encoded_length(56, 76, &STANDARD, true);
        assert_encoded_length(57, 76, &STANDARD, true);

        assert_encoded_length(58, 80, &STANDARD, true);
    }

    #[test]
    fn encoded_size_correct_no_pad() {
        assert_encoded_length(0, 0, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(1, 2, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(2, 3, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(3, 4, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(4, 6, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(5, 7, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(6, 8, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(7, 10, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(8, 11, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(9, 12, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(54, 72, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(55, 74, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(56, 75, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(57, 76, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(58, 78, &URL_SAFE_NO_PAD_ENGINE, false);
    }

    #[test]
    fn encoded_size_overflow() {
        assert_eq!(None, encoded_len(usize::MAX, true));
    }

    #[test]
    fn encode_engine_string_into_nonempty_buffer_doesnt_clobber_prefix() {
        let mut orig_data = Vec::new();
        let mut prefix = String::new();
        let mut encoded_data_no_prefix = String::new();
        let mut encoded_data_with_prefix = String::new();
        let mut decoded = Vec::new();

        let prefix_len_range = Uniform::new(0, 1000);
        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            prefix.clear();
            encoded_data_no_prefix.clear();
            encoded_data_with_prefix.clear();
            decoded.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            let prefix_len = prefix_len_range.sample(&mut rng);
            for _ in 0..prefix_len {
                // getting convenient random single-byte printable chars that aren't base64 is
                // annoying
                prefix.push('#');
            }
            encoded_data_with_prefix.push_str(&prefix);

            let engine = random_engine(&mut rng);
            engine.encode_string(&orig_data, &mut encoded_data_no_prefix);
            engine.encode_string(&orig_data, &mut encoded_data_with_prefix);

            assert_eq!(
                encoded_data_no_prefix.len() + prefix_len,
                encoded_data_with_prefix.len()
            );
            assert_encode_sanity(
                &encoded_data_no_prefix,
                engine.config().encode_padding(),
                input_len,
            );
            assert_encode_sanity(
                &encoded_data_with_prefix[prefix_len..],
                engine.config().encode_padding(),
                input_len,
            );

            // append plain encode onto prefix
            prefix.push_str(&encoded_data_no_prefix);

            assert_eq!(prefix, encoded_data_with_prefix);

            engine
                .decode_vec(&encoded_data_no_prefix, &mut decoded)
                .unwrap();
            assert_eq!(orig_data, decoded);
        }
    }

    #[test]
    fn encode_engine_slice_into_nonempty_buffer_doesnt_clobber_suffix() {
        let mut orig_data = Vec::new();
        let mut encoded_data = Vec::new();
        let mut encoded_data_original_state = Vec::new();
        let mut decoded = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            encoded_data.clear();
            encoded_data_original_state.clear();
            decoded.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            // plenty of existing garbage in the encoded buffer
            for _ in 0..10 * input_len {
                encoded_data.push(rng.gen());
            }

            encoded_data_original_state.extend_from_slice(&encoded_data);

            let engine = random_engine(&mut rng);

            let encoded_size = encoded_len(input_len, engine.config().encode_padding()).unwrap();

            assert_eq!(
                encoded_size,
                engine.encode_slice(&orig_data, &mut encoded_data).unwrap()
            );

            assert_encode_sanity(
                str::from_utf8(&encoded_data[0..encoded_size]).unwrap(),
                engine.config().encode_padding(),
                input_len,
            );

            assert_eq!(
                &encoded_data[encoded_size..],
                &encoded_data_original_state[encoded_size..]
            );

            engine
                .decode_vec(&encoded_data[0..encoded_size], &mut decoded)
                .unwrap();
            assert_eq!(orig_data, decoded);
        }
    }

    #[test]
    fn encode_to_slice_random_valid_utf8() {
        let mut input = Vec::new();
        let mut output = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            input.clear();
            output.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                input.push(rng.gen());
            }

            let config = random_config(&mut rng);
            let engine = random_engine(&mut rng);

            // fill up the output buffer with garbage
            let encoded_size = encoded_len(input_len, config.encode_padding()).unwrap();
            for _ in 0..encoded_size {
                output.push(rng.gen());
            }

            let orig_output_buf = output.clone();

            let bytes_written = engine.internal_encode(&input, as_uninit(&mut output));

            // make sure the part beyond bytes_written is the same garbage it was before
            assert_eq!(orig_output_buf[bytes_written..], output[bytes_written..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..bytes_written]).unwrap();
        }
    }

    #[test]
    fn encode_with_padding_random_valid_utf8() {
        let mut input = Vec::new();
        let mut output = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            input.clear();
            output.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                input.push(rng.gen());
            }

            let engine = random_engine(&mut rng);

            // fill up the output buffer with garbage
            let encoded_size = encoded_len(input_len, engine.config().encode_padding()).unwrap();
            for _ in 0..encoded_size + 1000 {
                output.push(rng.gen());
            }

            let orig_output_buf = output.clone();

            encode_with_padding(
                &input,
                as_uninit(&mut output[0..encoded_size]),
                &engine,
                encoded_size,
            );

            // make sure the part beyond b64 is the same garbage it was before
            assert_eq!(orig_output_buf[encoded_size..], output[encoded_size..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..encoded_size]).unwrap();
        }
    }

    fn assert_encoded_length<E: Engine>(
        input_len: usize,
        enc_len: usize,
        engine: &E,
        padded: bool,
    ) {
        assert_eq!(enc_len, encoded_len(input_len, padded).unwrap());

        let mut bytes: Vec<u8> = Vec::new();
        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..input_len {
            bytes.push(rng.gen());
        }

        let encoded = engine.encode(&bytes);
        assert_encode_sanity(&encoded, padded, input_len);

        assert_eq!(enc_len, encoded.len());
    }

    #[test]
    fn encode_imap() {
        assert_eq!(
            &GeneralPurpose::new(&alphabet::IMAP_MUTF7, NO_PAD).encode(b"\xFB\xFF"),
            &GeneralPurpose::new(&alphabet::STANDARD, NO_PAD)
                .encode(b"\xFB\xFF")
                .replace('/', ",")
        );
    }

    /// Casts `[u8]` slice to `[MaybeUninit<u8>]`.
    fn as_uninit(slice: &mut [u8]) -> &mut [core::mem::MaybeUninit<u8>] {
        unsafe { core::mem::transmute(slice) }
    }
}
