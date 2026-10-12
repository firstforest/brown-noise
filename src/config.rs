use std::fs;
use std::io;
use std::path::PathBuf;

use crate::volume::Volume;

const APP_DIR: &str = "brown-noise";
const FILE_NAME: &str = "settings.conf";

/// 再起動をまたいで保持する設定。`key = value` の行だけの形式で保存する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub volume: Volume,
    pub play_on_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: Volume::DEFAULT,
            play_on_launch: false,
        }
    }
}

impl Settings {
    /// 読めない・壊れている場合は既定値で起動する。設定ファイルが原因で常駐できないのは避けたい。
    pub fn load() -> Self {
        path()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        let path = path().ok_or_else(|| io::Error::other("config directory not found"))?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        // 書き込み途中で落ちても既存の設定を壊さないよう、別名で書いてから置き換える
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, self.serialize())?;
        fs::rename(tmp, path)
    }

    fn parse(text: &str) -> Self {
        let mut settings = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match (key.trim(), value.trim()) {
                ("volume", v) => {
                    if let Ok(percent) = v.parse() {
                        settings.volume = Volume::from_percent(percent);
                    }
                }
                ("play_on_launch", v) => {
                    if let Ok(flag) = v.parse() {
                        settings.play_on_launch = flag;
                    }
                }
                _ => {}
            }
        }
        settings
    }

    fn serialize(&self) -> String {
        format!(
            "volume = {}\nplay_on_launch = {}\n",
            self.volume.percent(),
            self.play_on_launch
        )
    }
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(APP_DIR).join(FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let settings = Settings {
            volume: Volume::from_percent(35),
            play_on_launch: true,
        };
        assert_eq!(Settings::parse(&settings.serialize()), settings);
    }

    #[test]
    fn falls_back_to_defaults_for_bad_values() {
        let settings = Settings::parse("volume = loud\nplay_on_launch = maybe\n");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn ignores_unknown_keys_and_junk_lines() {
        let settings = Settings::parse("# comment\ntheme = dark\n\nvolume=80\n");
        assert_eq!(settings.volume, Volume::from_percent(80));
        assert!(!settings.play_on_launch);
    }

    #[test]
    fn clamps_out_of_range_volume() {
        assert_eq!(
            Settings::parse("volume = 0").volume,
            Volume::from_percent(5)
        );
    }
}
