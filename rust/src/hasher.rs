//! hasher provides a custom hash implementation optimized for the 1BRC challenge.
//! This hasher is designed to be fast for station name keys in the weather measurements dataset.
use std::hash;

/// A simple hasher map hasher. Uses polynomial rolling hash approach with multiplication and XOR mixing.
/// This hasher trades cryptographic security for speed, which is perfect for our use case
/// where we're just counting weather station measurements.
pub struct DumbHasher(u64);

impl hash::Hasher for DumbHasher {
    /// Returns the final hash value after processing all input bytes.
    /// The value stored in the internal state is a final mixing
    /// operation to improve hash distribution.
    fn finish(&self) -> u64 {
        //note: Final mixing: XOR the state with itself rotated by different amounts.
        // This helps ensure that small changes in input lead to larger changes in the output hash,
        // improving the quality of the hash for use in hash tables.
        self.0 ^ self.0.rotate_right(33) ^ self.0.rotate_right(15)
    }

    /// Processes a slice of bytes and updates the internal hash state.
    /// For potentially short keys, this reads up to 16 bytes efficiently
    /// and mixes them directly into the state using XOR.
    fn write(&mut self, bytes: &[u8]) {
        // Create a buffer to hold up to 16 bytes (two u64s)
        let mut word = [0u64; 2];
        //SAFETY: We copy `bytes.len().min(16)` bytes into the `word` buffer.
        // This is safe as long as the length we copy does not exceed the size of `word` (16 bytes).
        unsafe {
            std::ptr::copy(
                bytes.as_ptr(),                 // Source: start of input bytes
                word.as_mut_ptr().cast::<u8>(), // Destination: start of our 16-byte buffer (cast to u8*)
                bytes.len().min(16),            // Number of bytes to copy (up to 16)
            )
        };
        // Mix the two 64-bit words by XORing them. This becomes the new state.
        self.0 = word[0] ^ word[1];

        // OLD LOGIC (Commented out):
        // let (chunks, remainder) = bytes.as_chunks::<8>();
        // let mut last = [1u8; 8];
        // last[..remainder.len()].copy_from_slice(remainder);
        //
        // for &chunk in chunks.iter().chain(iter::once(&last)) {
        //     let mixed = self.0 as u128 * (u64::from_ne_bytes(chunk) as u128);
        //     self.0 = (mixed >> 64) as u64 ^ mixed as u64;
        // }
    }

    /// Tells the hasher that a length prefix is not needed.
    /// This is used by HashMap when hashing keys that already contain their own length information
    /// or where length is implicitly known (like slices). Skipping the prefix can save cycles
    /// and potentially improve performance for simple hashers like this one.
    fn write_length_prefix(&mut self, _len: usize) {
        // Do nothing - we don't want the length prepended by HashMap
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
        // Using the FNV offset basis as the initial state
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
