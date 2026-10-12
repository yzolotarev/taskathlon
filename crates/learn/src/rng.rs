//! Fixed-width SplitMix64: the same seed gives the same exercise on desktop and wasm.

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x2545_f491_4f6c_dd1d)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }
    /// 0..n (0 when n is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    /// lo..=hi.
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        lo.saturating_add((self.next_u64() % (hi - lo + 1) as u64) as i64)
    }
    /// A round business amount: lo..=hi in multiples of `step`.
    pub fn amount(&mut self, lo: i64, hi: i64, step: i64) -> f64 {
        let step = step.max(1);
        (self.int(lo / step, hi / step) * step) as f64
    }
    pub fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }
    /// A random item, in the interface language.
    pub fn pick(&mut self, items: &[&'static str]) -> &'static str {
        items.get(self.below(items.len())).copied().map(gridcraft_tr::ts).unwrap_or("")
    }
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
    /// `n` distinct items in random order (fewer if the pool is smaller), in the interface language.
    pub fn sample(&mut self, items: &[&'static str], n: usize) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = items.iter().copied().map(gridcraft_tr::ts).collect();
        self.shuffle(&mut v);
        v.truncate(n);
        v
    }
}
