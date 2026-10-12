#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod noise;
mod tray;
mod volume;

use std::time::Instant;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;

use audio::Player;
use tray::{Action, Tray};
use volume::Volume;

enum UserEvent {
    Menu(MenuEvent),
}

struct App {
    player: Player,
    volume: Volume,
    quitting: bool,
}

impl App {
    fn handle(&mut self, action: Action) {
        match action {
            Action::TogglePlay if self.player.is_playing() => self.player.stop(),
            Action::TogglePlay => {
                if let Err(err) = self.player.play() {
                    eprintln!("failed to start playback: {err}");
                }
            }
            Action::SetVolume(v) => self.set_volume(v),
            Action::Louder => self.set_volume(self.volume.louder()),
            Action::Quieter => self.set_volume(self.volume.quieter()),
            Action::Quit => {
                self.player.stop();
                self.quitting = true;
            }
        }
    }

    fn set_volume(&mut self, volume: Volume) {
        self.volume = volume;
        self.player.set_gain(volume.gain());
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

    let mut app = App {
        player: Player::new(Volume::DEFAULT.gain()),
        volume: Volume::DEFAULT,
        quitting: false,
    };
    let mut tray: Option<Tray> = None;

    event_loop.run(move |event, _, control_flow| {
        match event {
            // macOS ではイベントループ開始後でないとメニューバーに項目を作れない
            Event::NewEvents(StartCause::Init) => match Tray::new(false, app.volume) {
                Ok(t) => tray = Some(t),
                Err(err) => {
                    eprintln!("failed to create tray icon: {err}");
                    *control_flow = ControlFlow::ExitWithCode(1);
                    return;
                }
            },
            Event::UserEvent(UserEvent::Menu(event)) => {
                if let Some(action) = tray.as_ref().and_then(|t| t.action_for(&event.id)) {
                    app.handle(action);
                }
                if let Some(t) = &tray {
                    t.update(app.player.is_playing(), app.volume);
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
