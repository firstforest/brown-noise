#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod autostart;
mod config;
mod noise;
mod tray;
mod volume;

use std::time::Instant;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;

use audio::Player;
use autostart::Autostart;
use config::Settings;
use tray::{Action, Tray, View};
use volume::Volume;

enum UserEvent {
    Menu(MenuEvent),
    StreamLost(u64),
}

struct App {
    player: Player,
    settings: Settings,
    autostart: Autostart,
    quitting: bool,
}

impl App {
    fn handle(&mut self, action: Action) {
        match action {
            Action::TogglePlay if self.player.is_playing() => self.player.stop(),
            Action::TogglePlay => self.play(),
            Action::SetVolume(v) => self.set_volume(v),
            Action::Louder => self.set_volume(self.settings.volume.louder()),
            Action::Quieter => self.set_volume(self.settings.volume.quieter()),
            Action::TogglePlayOnLaunch => {
                self.settings.play_on_launch = !self.settings.play_on_launch;
                self.save_settings();
            }
            Action::ToggleAutostart => {
                let enable = self.autostart.is_enabled() != Some(true);
                if let Err(err) = self.autostart.set_enabled(enable) {
                    eprintln!("failed to change autostart: {err}");
                }
            }
            Action::Quit => {
                self.player.stop();
                self.quitting = true;
            }
        }
    }

    fn play(&mut self) {
        if let Err(err) = self.player.play() {
            eprintln!("failed to start playback: {err}");
        }
    }

    fn set_volume(&mut self, volume: Volume) {
        self.settings.volume = volume;
        self.player.set_gain(volume.gain());
        self.save_settings();
    }

    fn save_settings(&self) {
        if let Err(err) = self.settings.save() {
            eprintln!("failed to save settings: {err}");
        }
    }

    fn view(&self) -> View {
        View {
            playing: self.player.is_playing(),
            volume: self.settings.volume,
            play_on_launch: self.settings.play_on_launch,
            autostart: self.autostart.is_enabled(),
        }
    }
}

fn main() {
    #[allow(unused_mut)]
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        // Dock とアプリ切り替えに出さず、メニューバーだけに常駐する
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let settings = Settings::load();
    let proxy = event_loop.create_proxy();
    let mut app = App {
        player: Player::new(settings.volume.gain(), move |generation| {
            let _ = proxy.send_event(UserEvent::StreamLost(generation));
        }),
        settings,
        autostart: Autostart::new(),
        quitting: false,
    };
    let mut tray: Option<Tray> = None;

    event_loop.run(move |event, _, control_flow| {
        match event {
            // macOS ではイベントループ開始後でないとメニューバーに項目を作れない
            Event::NewEvents(StartCause::Init) => {
                if app.settings.play_on_launch {
                    app.play();
                }
                match Tray::new(&app.view()) {
                    Ok(t) => tray = Some(t),
                    Err(err) => {
                        eprintln!("failed to create tray icon: {err}");
                        *control_flow = ControlFlow::ExitWithCode(1);
                        return;
                    }
                }
            }
            Event::UserEvent(UserEvent::Menu(event)) => {
                if let Some(action) = tray.as_ref().and_then(|t| t.action_for(&event.id)) {
                    app.handle(action);
                }
                if let Some(t) = &tray {
                    t.update(&app.view());
                }
            }
            Event::UserEvent(UserEvent::StreamLost(generation)) => {
                if let Err(err) = app.player.recover(generation) {
                    eprintln!("failed to recover playback: {err}");
                }
                if let Some(t) = &tray {
                    t.update(&app.view());
                }
            }
            _ => {}
        }

        let fade_deadline = app.player.tick(Instant::now());
        *control_flow = match fade_deadline {
            Some(deadline) => ControlFlow::WaitUntil(deadline),
            None if app.quitting => {
                // Windows では明示的に破棄しないと通知領域にアイコンが残る
                tray = None;
                ControlFlow::Exit
            }
            None => ControlFlow::Wait,
        };
    });
}
