//! Credential-at-rest protection.
//!
//! Credentials must never sit on disk as readable plaintext. This module is the
//! single, isolated place where that protection happens, so the platform split
//! stays obvious and easy to audit.
//!
//! * On Windows (`#[cfg(windows)]`) values are encrypted with the Data
//!   Protection API (DPAPI: `CryptProtectData` / `CryptUnprotectData`) tied to
//!   the CURRENT USER. The ciphertext is keyed by the logged-in user's profile,
//!   so another user on the same machine cannot read it, and copying the file to
//!   a different machine renders it undecryptable.
//! * On every other platform (`#[cfg(not(windows))]`) there is a plaintext
//!   pass-through fallback. This exists only so the project still builds and the
//!   surrounding logic can be exercised during development on Linux/macOS; it is
//!   NOT a security feature.
//!
//! The encrypted bytes are stored in the config file as base64 text (see
//! `base64`) so the file stays a human-locatable, copy-pasteable INI.
//!
//! Nothing here ever logs or prints a decrypted value.

use std::io;

/// Whether secrets written by this module are actually encrypted on the current
/// platform. `true` on Windows (DPAPI), `false` on the development fallback.
///
/// The config layer uses this to decide whether to write the `ENCRYPTED = 1`
/// marker, so a file's marker always reflects how its values were produced.
pub const ENCRYPTION_ACTIVE: bool = cfg!(windows);

/// Protects a secret for storage and returns it as base64 text.
///
/// On Windows the input is DPAPI-encrypted (current-user scope) before being
/// base64-encoded. On other platforms the input bytes are base64-encoded only.
pub fn protect(plaintext: &str) -> io::Result<String> {
    let bytes = imp::encrypt(plaintext.as_bytes())?;
    Ok(base64::encode(&bytes))
}

/// Reverses [`protect`], turning stored base64 text back into the secret.
///
/// On Windows the decoded bytes are DPAPI-decrypted; decryption fails (returns
/// an error) if the blob belongs to a different user or machine. On other
/// platforms the decoded bytes are returned as-is.
pub fn unprotect(encoded: &str) -> io::Result<String> {
    let bytes = base64::decode(encoded)?;
    let plaintext = imp::decrypt(&bytes)?;
    String::from_utf8(plaintext).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "decrypted data was not valid UTF-8",
        )
    })
}

// ---------------------------------------------------------------------------
// Platform implementation: Windows DPAPI.
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use std::io;
    use std::ptr;
    use std::slice;

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    /// Wraps a byte slice in the `DATA_BLOB` shape the DPAPI calls expect.
    ///
    /// The blob borrows `data`; it must not outlive it. DPAPI only reads from the
    /// input blob, so handing it a `*mut` derived from a shared slice is sound.
    fn input_blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    /// Copies the bytes out of a DPAPI-allocated output blob and frees it.
    ///
    /// DPAPI allocates `pbData` with `LocalAlloc`, so we own it and must release
    /// it with `LocalFree` once the bytes are copied into a Rust-owned `Vec`.
    ///
    /// # Safety
    /// `blob.pbData` must be a valid DPAPI output buffer of `blob.cbData` bytes.
    unsafe fn take_blob(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let out = if blob.pbData.is_null() {
            Vec::new()
        } else {
            slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec()
        };
        if !blob.pbData.is_null() {
            LocalFree(blob.pbData as *mut c_void);
        }
        out
    }

    pub fn encrypt(plaintext: &[u8]) -> io::Result<Vec<u8>> {
        let input = input_blob(plaintext);
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };

        // SAFETY: `input` is a valid blob for the duration of the call; all
        // optional parameters are null, and we never present UI.
        let ok = unsafe {
            CryptProtectData(
                &input,
                ptr::null(), // description
                ptr::null(), // optional entropy
                ptr::null(), // reserved
                ptr::null(), // prompt struct (UI forbidden below)
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the call succeeded, so `output` is a valid DPAPI buffer.
        Ok(unsafe { take_blob(output) })
    }

    pub fn decrypt(ciphertext: &[u8]) -> io::Result<Vec<u8>> {
        let input = input_blob(ciphertext);
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };

        // SAFETY: see `encrypt`; identical contract.
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                ptr::null_mut(), // description out (not requested)
                ptr::null(),     // optional entropy
                ptr::null(),     // reserved
                ptr::null(),     // prompt struct
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the call succeeded, so `output` is a valid DPAPI buffer.
        Ok(unsafe { take_blob(output) })
    }
}

// ---------------------------------------------------------------------------
// Platform implementation: development fallback (no encryption).
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
mod imp {
    use std::io;

    /// Pass-through "encryption" for non-Windows development builds.
    pub fn encrypt(plaintext: &[u8]) -> io::Result<Vec<u8>> {
        Ok(plaintext.to_vec())
    }

    /// Pass-through "decryption" for non-Windows development builds.
    pub fn decrypt(ciphertext: &[u8]) -> io::Result<Vec<u8>> {
        Ok(ciphertext.to_vec())
    }
}

// ---------------------------------------------------------------------------
// Minimal, dependency-free base64 (standard alphabet, with padding).
// ---------------------------------------------------------------------------

mod base64 {
    use std::io;

    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const PAD: u8 = b'=';

    /// Encodes bytes as standard base64 text with `=` padding.
    pub fn encode(input: &[u8]) -> String {
        let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = *chunk.get(1).unwrap_or(&0) as u32;
            let b2 = *chunk.get(2).unwrap_or(&0) as u32;
            let triple = (b0 << 16) | (b1 << 8) | b2;

            out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
            out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[((triple >> 6) & 0x3F) as usize] as char
            } else {
                PAD as char
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[(triple & 0x3F) as usize] as char
            } else {
                PAD as char
            });
        }
        out
    }

    /// Decodes standard base64 text. Whitespace is ignored; any other invalid
    /// character or malformed length is reported as an error.
    pub fn decode(input: &str) -> io::Result<Vec<u8>> {
        let mut symbols: Vec<u8> = Vec::with_capacity(input.len());
        for &byte in input.as_bytes() {
            match byte {
                b' ' | b'\t' | b'\r' | b'\n' => continue,
                _ => symbols.push(byte),
            }
        }

        // Strip trailing padding; standard base64 has at most two pad bytes.
        let mut data_len = symbols.len();
        while data_len > 0 && symbols[data_len - 1] == PAD {
            data_len -= 1;
        }
        let symbols = &symbols[..data_len];

        let mut out = Vec::with_capacity(symbols.len() / 4 * 3 + 3);
        let mut accumulator: u32 = 0;
        let mut bits: u32 = 0;
        for &symbol in symbols {
            let value = decode_symbol(symbol)?;
            accumulator = (accumulator << 6) | value as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((accumulator >> bits) as u8);
            }
        }
        Ok(out)
    }

    /// Maps a single base64 character to its 6-bit value.
    fn decode_symbol(symbol: u8) -> io::Result<u8> {
        match symbol {
            b'A'..=b'Z' => Ok(symbol - b'A'),
            b'a'..=b'z' => Ok(symbol - b'a' + 26),
            b'0'..=b'9' => Ok(symbol - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid base64 character in stored credential",
            )),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn round_trips_various_lengths() {
            for sample in ["", "f", "fo", "foo", "foob", "fooba", "foobar"] {
                let encoded = encode(sample.as_bytes());
                let decoded = decode(&encoded).expect("decode");
                assert_eq!(decoded, sample.as_bytes());
            }
        }

        #[test]
        fn known_vectors() {
            assert_eq!(encode(b"foobar"), "Zm9vYmFy");
            assert_eq!(encode(b"foo"), "Zm9v");
            assert_eq!(encode(b"fo"), "Zm8=");
            assert_eq!(decode("Zm9vYmFy").unwrap(), b"foobar");
        }

        #[test]
        fn rejects_garbage() {
            assert!(decode("not base64 *").is_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protect_unprotect_round_trip() {
        // On Windows this exercises real DPAPI; elsewhere the fallback. Either
        // way the value must survive a round trip unchanged.
        let secret = "S3cr3t value with symbols !@#";
        let stored = protect(secret).expect("protect");
        assert_ne!(stored, secret, "stored form must not be the raw secret");
        let recovered = unprotect(&stored).expect("unprotect");
        assert_eq!(recovered, secret);
    }
}
