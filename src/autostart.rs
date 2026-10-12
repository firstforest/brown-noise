use auto_launch::{AutoLaunch, AutoLaunchBuilder, MacOSLaunchMode, WindowsEnableMode};

const APP_NAME: &str = "brown-noise";

/// ログイン時の自動起動。登録するのは今動いている実行ファイルのパス。
pub struct Autostart(Option<AutoLaunch>);

impl Autostart {
    pub fn new() -> Self {
        let launcher = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.to_str().map(str::to_owned))
            .and_then(|path| {
                AutoLaunchBuilder::new()
                    .set_app_name(APP_NAME)
                    .set_app_path(&path)
                    // .app に包まずに動かしても使えるよう、ログイン項目ではなく LaunchAgent で登録する
                    .set_macos_launch_mode(MacOSLaunchMode::LaunchAgent)
                    // 管理者権限で動いていても他のユーザーのログインには影響させない
                    .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
                    .build()
                    .map_err(|err| eprintln!("autostart unavailable: {err}"))
                    .ok()
            });
        Self(launcher)
    }

    /// 自動起動を扱えない環境、または状態を読めない場合は None。
    pub fn is_enabled(&self) -> Option<bool> {
        self.0.as_ref()?.is_enabled().ok()
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<(), auto_launch::Error> {
        let Some(launcher) = &self.0 else {
            return Ok(());
        };
        if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        }
    }
}
