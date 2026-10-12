/// ホワイトノイズをリーク積分してブラウンノイズを生成する。
pub struct BrownNoise {
    rng: u32,
    last: f32,
}

impl BrownNoise {
    // 積分値が直流成分に張り付かないよう少しずつ 0 に戻す係数。
    const LEAK: f32 = 1.02;
    const STEP: f32 = 0.02;
    // 積分でほぼ ±0.3 に収まる振幅を ±1 付近まで持ち上げる。
    const GAIN: f32 = 3.5;

    pub fn new(seed: u32) -> Self {
        Self {
            // xorshift は状態 0 から抜け出せない
            rng: seed.max(1),
            last: 0.0,
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        self.last = (self.last + Self::STEP * self.white()) / Self::LEAK;
        self.last * Self::GAIN
    }

    fn white(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(n: usize) -> Vec<f32> {
        let mut noise = BrownNoise::new(12345);
        (0..n).map(|_| noise.next_sample()).collect()
    }

    fn variance(xs: &[f32]) -> f32 {
        let mean = xs.iter().sum::<f32>() / xs.len() as f32;
        xs.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / xs.len() as f32
    }

    #[test]
    fn stays_within_audible_range() {
        let xs = samples(48_000 * 60);
        let peak = xs.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak < 1.5, "peak = {peak}");
        assert!(peak > 0.3, "peak = {peak}");
    }

    #[test]
    fn has_no_dc_offset() {
        let xs = samples(48_000 * 60);
        let mean = xs.iter().sum::<f32>() / xs.len() as f32;
        assert!(mean.abs() < 0.05, "mean = {mean}");
    }

    #[test]
    fn energy_is_concentrated_in_low_frequencies() {
        // 隣接サンプルの差分は高域成分を表す。ブラウンノイズなら信号全体よりずっと小さい。
        let xs = samples(48_000 * 10);
        let diffs: Vec<f32> = xs.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(variance(&diffs) < variance(&xs) * 0.1);
    }

    #[test]
    fn zero_seed_still_produces_noise() {
        let mut noise = BrownNoise::new(0);
        let xs: Vec<f32> = (0..1000).map(|_| noise.next_sample()).collect();
        assert!(xs.iter().any(|x| *x != 0.0));
    }
}
