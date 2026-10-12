mod audio;
mod noise;

use std::io::{self, BufRead};
use std::process::ExitCode;

use audio::Player;

const DEFAULT_VOLUME: f32 = 0.5;

fn main() -> ExitCode {
    let volume = match std::env::args().nth(1) {
        None => DEFAULT_VOLUME,
        Some(arg) => match arg.parse::<f32>() {
            Ok(v) if (0.0..=1.0).contains(&v) => v,
            _ => {
                eprintln!("usage: brown-noise [volume 0.0-1.0]");
                return ExitCode::FAILURE;
            }
        },
    };

    let mut player = Player::new(volume);
    if let Err(err) = player.play() {
        eprintln!("failed to start playback: {err}");
        return ExitCode::FAILURE;
    }

    println!(
        "playing brown noise (volume {:.0}%). press Enter to stop.",
        volume * 100.0
    );
    let _ = io::stdin().lock().lines().next();

    player.stop();
    ExitCode::SUCCESS
}
