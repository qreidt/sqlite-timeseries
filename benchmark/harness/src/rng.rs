//! Small deterministic PRNG (SplitMix64), so query parameters never change with a crate upgrade.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform integer in `[0, n)`, without modulo bias.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0, "Rng::below(0)");
        let zone = u64::MAX - (u64::MAX % n);
        loop {
            let x = self.next_u64();
            if x < zone {
                return x % n;
            }
        }
    }

    /// `k` distinct integers from `[0, n)`, in random order.
    pub fn sample_distinct(&mut self, k: u64, n: u64) -> Vec<u64> {
        assert!(k <= n);
        let mut picked = std::collections::HashSet::with_capacity(k as usize);
        let mut out = Vec::with_capacity(k as usize);
        while (out.len() as u64) < k {
            let x = self.below(n);
            if picked.insert(x) {
                out.push(x);
            }
        }
        out
    }
}

/// Derive a per-label seed (FNV-1a over the label, mixed with the base seed).
pub fn seed_for(base: u64, label: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in label.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h ^ base.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_distinct() {
        let mut a = Rng::new(seed_for(123, "x"));
        let mut b = Rng::new(seed_for(123, "x"));
        assert_eq!(a.next_u64(), b.next_u64());
        let s = a.sample_distinct(8, 10);
        let set: std::collections::HashSet<_> = s.iter().collect();
        assert_eq!(set.len(), 8);
        assert!(s.iter().all(|&x| x < 10));
    }
}
