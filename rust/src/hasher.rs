//! hasher provides a custom hash implementation optimized for the 1BRC challenge.
//! This hasher is designed to be fast for station name keys in the weather measurements dataset.
use std::{hash, iter};

/// A simple hasher map hasher. Uses polynomial rolling hash approach with multiplication and XOR mixing.
/// This hasher trades cryptographic security for speed, which is perfect for our use case
/// where we're just counting weather station measurements.
pub struct DumbHasher(u64);

impl hash::Hasher for DumbHasher {
    /// Returns the final hash value after processing all input bytes.
    /// The value stored in the internal state is returned as-is.
    fn finish(&self) -> u64 {
        self.0
    }

    /// Processes a slice of bytes and updates the internal hash state.
    /// Chunks the input into 8-byte segments and applies polynomial mixing.
    /// The last chunk (or partial chunk) is padded with 1's to ensure consistent behavior.
    fn write(&mut self, bytes: &[u8]) {
        // if bytes.len() > 16 {
        let (chunks, remainder) = bytes.as_chunks::<8>();
        let mut last = [1u8; 8];
        last[..remainder.len()].copy_from_slice(remainder);

        for &chunk in chunks.iter().chain(iter::once(&last)) {
            let mixed = self.0 as u128 * (u64::from_ne_bytes(chunk) as u128);
            self.0 = (mixed >> 64) as u64 ^ mixed as u64;
        }
        // } else {
        //     let mut last = [0u8; 16];
        //     last[..bytes.len()].copy_from_slice(bytes);
        //     let left = i64::from_ne_bytes([
        //         last[0], last[1], last[2], last[3], last[4], last[5], last[6], last[7],
        //     ]);
        //     let right = i64::from_ne_bytes([
        //         last[8], last[9], last[10], last[11], last[12], last[13], last[14], last[15],
        //     ]);
        //     let mut h = (left ^ right) * -7046029254386353131;
        //     h ^= h >> 35;
        //     self.0 = h as u64;
        // }
    }
}

/// Builder for creating instances of DumbHasher.
/// Implements BuildHasher trait to work seamlessly with Rust's HashMap.
pub struct DumbHasherBuilder;

impl hash::BuildHasher for DumbHasherBuilder {
    type Hasher = DumbHasher;

    /// Creates a new DumbHasher instance with an initial seed value.
    /// The seed 0xcbf29ce484222325 is the FNV offset basis, providing good initial entropy.
    fn build_hasher(&self) -> Self::Hasher {
        DumbHasher(0xcbf29ce484222325)
    }
}

#[cfg(test)]
mod hasher_tests {
    use super::*;
    use std::hash::{BuildHasher, Hasher};

    #[test]
    fn test_dumb_hasher_basic() {
        let mut hasher = DumbHasherBuilder.build_hasher();
        hasher.write(b"test");
        let result = hasher.finish();
        assert_ne!(result, 0); // Should produce non-zero hash
    }

    #[test]
    fn test_dumb_hasher_consistency() {
        let mut hasher1 = DumbHasherBuilder.build_hasher();
        let mut hasher2 = DumbHasherBuilder.build_hasher();

        hasher1.write(b"hello world");
        hasher2.write(b"hello world");

        assert_eq!(hasher1.finish(), hasher2.finish()); // Same inputs should produce same hash
    }
}
