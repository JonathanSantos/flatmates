//! Gerador determinístico pequeno (SplitMix64), sem dependências.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    /// Semente própria por elemento: editar um não muda os outros.
    pub fn derive(seed: u64, key: &str) -> Rng {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        for b in key.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        let mut r = Rng(h);
        r.next();
        r
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Valor em [0, 1).
    pub fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.f() * n as f64) as usize % n.max(1)
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[self.below(xs.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_same_sequence() {
        let mut a = Rng::derive(7, "person:0");
        let mut b = Rng::derive(7, "person:0");
        let mut c = Rng::derive(7, "person:1");
        let (x, y, z) = (a.f(), b.f(), c.f());
        assert_eq!(x, y);
        assert_ne!(x, z);
        assert!((0.0..1.0).contains(&x));
    }
}
