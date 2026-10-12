use std::error::Error;

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::volume::Volume;

const ICON_SIZE: u32 = 32;
// macOS はテンプレート画像として alpha だけを使い、メニューバーの明暗に合わせて着色される。
// それ以外の OS は色がそのまま出るため、暗いタスクバーで見える明るい色にする。
const ICON_RGB: [u8; 3] = if cfg!(target_os = "macos") {
    [0, 0, 0]
} else {
    [0xE8, 0xE8, 0xE8]
};
const ICON_ALPHA_PLAYING: u8 = 0xFF;
const ICON_ALPHA_STOPPED: u8 = 0x60;

pub enum Action {
    TogglePlay,
    SetVolume(Volume),
    Louder,
    Quieter,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    toggle: MenuItem,
    volume_label: MenuItem,
    presets: Vec<(CheckMenuItem, Volume)>,
    louder: MenuItem,
    quieter: MenuItem,
    quit: MenuItem,
}

impl Tray {
    pub fn new(playing: bool, volume: Volume) -> Result<Self, Box<dyn Error>> {
        let toggle = MenuItem::new("", true, None);
        let volume_label = MenuItem::new("", false, None);
        let presets: Vec<_> = Volume::PRESETS
            .iter()
            .map(|&v| {
                let item = CheckMenuItem::new(format!("{}%", v.percent()), true, false, None);
                (item, v)
            })
            .collect();
        let louder = MenuItem::new("音量 +10%", true, None);
        let quieter = MenuItem::new("音量 −10%", true, None);
        let quit = MenuItem::new("終了", true, None);

        let menu = Menu::new();
        menu.append_items(&[&toggle, &PredefinedMenuItem::separator(), &volume_label])?;
        for (item, _) in &presets {
            menu.append(item)?;
        }
        menu.append_items(&[&louder, &quieter, &PredefinedMenuItem::separator(), &quit])?;

        let icon = TrayIconBuilder::new().with_menu(Box::new(menu)).build()?;

        let tray = Self {
            icon,
            toggle,
            volume_label,
            presets,
            louder,
            quieter,
            quit,
        };
        tray.update(playing, volume);
        Ok(tray)
    }

    pub fn action_for(&self, id: &MenuId) -> Option<Action> {
        if id == self.toggle.id() {
            Some(Action::TogglePlay)
        } else if id == self.louder.id() {
            Some(Action::Louder)
        } else if id == self.quieter.id() {
            Some(Action::Quieter)
        } else if id == self.quit.id() {
            Some(Action::Quit)
        } else {
            self.presets
                .iter()
                .find(|(item, _)| item.id() == id)
                .map(|&(_, v)| Action::SetVolume(v))
        }
    }

    /// メニューの表示を状態に合わせる。CheckMenuItem はクリックで勝手に切り替わるので毎回上書きする。
    pub fn update(&self, playing: bool, volume: Volume) {
        self.toggle
            .set_text(if playing { "⏸ 停止" } else { "▶ 再生" });
        self.volume_label
            .set_text(format!("音量 {}%", volume.percent()));
        for (item, v) in &self.presets {
            item.set_checked(*v == volume);
        }
        self.louder.set_enabled(!volume.is_max());
        self.quieter.set_enabled(!volume.is_min());

        let state = if playing { "再生中" } else { "停止中" };
        let _ = self
            .icon
            .set_tooltip(Some(format!("Brown Noise — {state} {}%", volume.percent())));
        let icon = render_icon(playing);
        #[cfg(target_os = "macos")]
        let result = self.icon.set_icon_templated(Some(icon));
        #[cfg(not(target_os = "macos"))]
        let result = self.icon.set_icon(Some(icon));
        if let Err(err) = result {
            eprintln!("failed to set tray icon: {err}");
        }
    }
}

/// 低域ほど高い棒を並べ、ブラウンノイズのスペクトルを模したアイコンを描く。
fn render_icon(playing: bool) -> Icon {
    const BARS: [f32; 5] = [0.9, 0.72, 0.55, 0.4, 0.28];
    const BAR_WIDTH: u32 = 4;
    const GAP: u32 = 2;
    let alpha = if playing {
        ICON_ALPHA_PLAYING
    } else {
        ICON_ALPHA_STOPPED
    };
    let total_width = BARS.len() as u32 * (BAR_WIDTH + GAP) - GAP;
    let left = (ICON_SIZE - total_width) / 2;

    let mut rgba = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    for (i, height) in BARS.iter().enumerate() {
        let x0 = left + i as u32 * (BAR_WIDTH + GAP);
        let bar_height = (ICON_SIZE as f32 * height).round() as u32;
        let top = (ICON_SIZE - bar_height) / 2;
        for y in top..top + bar_height {
            for x in x0..x0 + BAR_WIDTH {
                let p = ((y * ICON_SIZE + x) * 4) as usize;
                rgba[p..p + 3].copy_from_slice(&ICON_RGB);
                rgba[p + 3] = alpha;
            }
        }
    }
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("icon buffer matches its dimensions")
}
