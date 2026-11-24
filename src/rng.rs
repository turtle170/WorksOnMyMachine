//! A tiny deterministic PRNG (SplitMix64). Every "random" thing womm does is
//! seeded, so the same inputs on the same machine always behave the same way.

use sha2::{Digest, Sha512};

#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn from_seed(seed: &[u8]) -> Rng {
        let d = Sha512::digest(seed);
        Rng(u64::from_le_bytes(d[..8].try_into().unwrap()))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn bytes8(&mut self) -> [u8; 8] {
        self.next_u64().to_le_bytes()
    }
}

/// Anything that can make a choice. NoNet does it with a neural network; a
/// plain `Rng` does it the boring way (used where results must not depend on
/// the local machine, e.g. `womm duel`).
pub trait Chooser {
    fn pick(&mut self, n: usize) -> usize;
    fn one_in(&mut self, n: usize) -> bool {
        self.pick(n) == 0
    }
}

impl Chooser for Rng {
    fn pick(&mut self, n: usize) -> usize {
        self.below(n)
    }
}
