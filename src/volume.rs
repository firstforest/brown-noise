/// メニューで扱う音量。パーセントで持ち、出力ゲインへは聴感に合わせて変換する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Volume(u8);

impl Volume {
    pub const DEFAULT: Volume = Volume(50);
    pub const PRESETS: [Volume; 4] = [Volume(25), Volume(50), Volume(75), Volume(100)];
    const STEP: u8 = 10;
    // 0 にすると「再生中なのに無音」になるため下限を設ける。
    const MIN: u8 = 5;
    const MAX: u8 = 100;

    pub fn percent(self) -> u8 {
        self.0
    }

    pub fn louder(self) -> Self {
        Volume((self.0 + Self::STEP).min(Self::MAX))
    }

    pub fn quieter(self) -> Self {
        Volume(self.0.saturating_sub(Self::STEP).max(Self::MIN))
    }

    pub fn is_max(self) -> bool {
        self.0 >= Self::MAX
    }

    pub fn is_min(self) -> bool {
        self.0 <= Self::MIN
    }

    /// 人の音量感覚は対数的なので、線形のままだと上半分の変化が感じ取りにくい。
    pub fn gain(self) -> f32 {
        let x = f32::from(self.0) / 100.0;
        x * x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn louder_stops_at_max() {
        assert_eq!(Volume(95).louder(), Volume(100));
        assert_eq!(Volume(100).louder(), Volume(100));
        assert!(Volume(100).is_max());
    }

    #[test]
    fn quieter_stops_at_min() {
        assert_eq!(Volume(25).quieter(), Volume(15));
        assert_eq!(Volume(15).quieter(), Volume(5));
        assert_eq!(Volume(5).quieter(), Volume(5));
        assert!(Volume(5).is_min());
    }

    #[test]
    fn gain_is_monotonic_and_bounded() {
        let gains: Vec<f32> = (Volume::MIN..=Volume::MAX)
            .map(|p| Volume(p).gain())
            .collect();
        assert!(gains.windows(2).all(|w| w[0] < w[1]));
        assert!(gains[0] > 0.0);
        assert_eq!(Volume(100).gain(), 1.0);
    }

    #[test]
    fn default_is_a_preset() {
        assert!(Volume::PRESETS.contains(&Volume::DEFAULT));
    }
}
