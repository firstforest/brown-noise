use std::error::Error;

use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::volume::Volume;

const ICON_SIZE: u32 = 32;
const ICON_ALPHA_PLAYING: u8 = 0xFF;
const ICON_ALPHA_STOPPED: u8 = 0x60;

// macOS はテンプレート画像として alpha だけを使い、メニューバーの明暗に合わせて着色される。
// それ以外の OS は色がそのまま出て背景の明暗も分からないため、明るい塗りに暗い縁を付ける。
#[cfg(target_os = "macos")]
mod style {
    pub const FILL: [u8; 3] = [0, 0, 0];
    pub const OUTLINE: Option<[u8; 3]> = None;
    pub const BAR_WIDTH: u32 = 4;
    pub const GAP: u32 = 2;
}
#[cfg(not(target_os = "macos"))]
mod style {
    pub const FILL: [u8; 3] = [0xF0, 0xF0, 0xF0];
    pub const OUTLINE: Option<[u8; 3]> = Some([0x20, 0x20, 0x20]);
    pub const BAR_WIDTH: u32 = 5;
    pub const GAP: u32 = 1;
}

pub enum Action {
    TogglePlay,
    SetVolume(Volume),
    Louder,
    Quieter,
    TogglePlayOnLaunch,
    ToggleAutostart,
    Quit,
}

/// メニューに映す状態。
pub struct View {
    pub playing: bool,
    pub volume: Volume,
    pub play_on_launch: bool,
    /// 自動起動を扱えない環境では None になり、項目を無効にする。
    pub autostart: Option<bool>,
}

pub struct Tray {
    icon: TrayIcon,
    toggle: MenuItem,
    volume_label: MenuItem,
    presets: Vec<(CheckMenuItem, Volume)>,
    louder: MenuItem,
    quieter: MenuItem,
    play_on_launch: CheckMenuItem,
    autostart: CheckMenuItem,
    quit: MenuItem,
}

impl Tray {
    pub fn new(view: &View) -> Result<Self, Box<dyn Error>> {
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
        let play_on_launch = CheckMenuItem::new("起動時に再生", true, false, None);
        let autostart = CheckMenuItem::new("ログイン時に起動", true, false, None);
        let quit = MenuItem::new("終了", true, None);

        let menu = Menu::new();
        menu.append_items(&[&toggle, &PredefinedMenuItem::separator(), &volume_label])?;
        for (item, _) in &presets {
            menu.append(item)?;
        }
        menu.append_items(&[
            &louder,
            &quieter,
            &PredefinedMenuItem::separator(),
            &play_on_launch,
            &autostart,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;

        let icon = TrayIconBuilder::new().with_menu(Box::new(menu)).build()?;

        let tray = Self {
            icon,
            toggle,
            volume_label,
            presets,
            louder,
            quieter,
            play_on_launch,
            autostart,
            quit,
        };
        tray.update(view);
        Ok(tray)
    }

    pub fn action_for(&self, id: &MenuId) -> Option<Action> {
        if id == self.toggle.id() {
            Some(Action::TogglePlay)
        } else if id == self.louder.id() {
            Some(Action::Louder)
        } else if id == self.quieter.id() {
            Some(Action::Quieter)
        } else if id == self.play_on_launch.id() {
            Some(Action::TogglePlayOnLaunch)
        } else if id == self.autostart.id() {
            Some(Action::ToggleAutostart)
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
    pub fn update(&self, view: &View) {
        self.toggle.set_text(if view.playing {
            "⏸ 停止"
        } else {
            "▶ 再生"
        });
        self.volume_label
            .set_text(format!("音量 {}%", view.volume.percent()));
        for (item, v) in &self.presets {
            item.set_checked(*v == view.volume);
        }
        self.louder.set_enabled(!view.volume.is_max());
        self.quieter.set_enabled(!view.volume.is_min());
        self.play_on_launch.set_checked(view.play_on_launch);
        self.autostart.set_checked(view.autostart == Some(true));
        self.autostart.set_enabled(view.autostart.is_some());

        let state = if view.playing {
            "再生中"
        } else {
            "停止中"
        };
        let _ = self.icon.set_tooltip(Some(format!(
            "Brown Noise — {state} {}%",
            view.volume.percent()
        )));
        let icon = render_icon(view.playing);
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
    let alpha = if playing {
        ICON_ALPHA_PLAYING
    } else {
        ICON_ALPHA_STOPPED
    };
    let total_width = BARS.len() as u32 * (style::BAR_WIDTH + style::GAP) - style::GAP;
    let left = (ICON_SIZE - total_width) / 2;

    let mut rgba = vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    for (i, height) in BARS.iter().enumerate() {
        let x0 = left + i as u32 * (style::BAR_WIDTH + style::GAP);
        let x1 = x0 + style::BAR_WIDTH;
        let bar_height = (ICON_SIZE as f32 * height).round() as u32;
        let y0 = (ICON_SIZE - bar_height) / 2;
        let y1 = y0 + bar_height;
        for y in y0..y1 {
            for x in x0..x1 {
                let edge = x == x0 || x == x1 - 1 || y == y0 || y == y1 - 1;
                let rgb = match style::OUTLINE {
                    Some(outline) if edge => outline,
                    _ => style::FILL,
                };
                let p = ((y * ICON_SIZE + x) * 4) as usize;
                rgba[p..p + 3].copy_from_slice(&rgb);
                rgba[p + 3] = alpha;
            }
        }
    }
    Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE).expect("icon buffer matches its dimensions")
}
