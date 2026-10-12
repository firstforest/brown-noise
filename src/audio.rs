use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BufferSize, Device, Error, ErrorKind, FromSample, SampleFormat, SizedSample, Stream,
    StreamConfig, SupportedBufferSize,
};

use crate::noise::BrownNoise;

// コールバックの起床回数を減らすため大きめに取る。遅延は用途上問題にならない。
const PREFERRED_BUFFER_FRAMES: u32 = 4096;
// 音量変化をなめらかにする一次ローパスの時定数。
const GAIN_TIME_CONSTANT: f32 = 0.02;
// 時定数の 6 倍待てばゲインは 0.25% まで下がり、切断してもクリック音にならない。
const FADE_OUT: Duration = Duration::from_millis(120);

/// 音声コールバックと共有する状態。コールバック内でロックしないよう atomic だけで持つ。
struct Shared {
    gain: AtomicU32,
    fading_out: AtomicBool,
}

impl Shared {
    fn target_gain(&self) -> f32 {
        if self.fading_out.load(Ordering::Relaxed) {
            0.0
        } else {
            f32::from_bits(self.gain.load(Ordering::Relaxed))
        }
    }
}

/// 停止中はストリームを保持しない。デバイスを開いたままにするとスリープを妨げるため。
pub struct Player {
    shared: Arc<Shared>,
    stream: Option<Stream>,
    fade_deadline: Option<Instant>,
}

impl Player {
    pub fn new(gain: f32) -> Self {
        Self {
            shared: Arc::new(Shared {
                gain: AtomicU32::new(gain.clamp(0.0, 1.0).to_bits()),
                fading_out: AtomicBool::new(false),
            }),
            stream: None,
            fade_deadline: None,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.stream.is_some() && self.fade_deadline.is_none()
    }

    pub fn set_gain(&self, gain: f32) {
        self.shared
            .gain
            .store(gain.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn play(&mut self) -> Result<(), Error> {
        self.fade_deadline = None;
        self.shared.fading_out.store(false, Ordering::Relaxed);
        if self.stream.is_some() {
            // フェードアウト途中なら同じストリームのままフェードインに戻る
            return Ok(());
        }

        let device = cpal::default_host()
            .default_output_device()
            .ok_or(Error::from(ErrorKind::DeviceNotAvailable))?;
        let supported = device.default_output_config()?;
        let config = StreamConfig {
            buffer_size: match supported.buffer_size() {
                SupportedBufferSize::Range { min, max } => {
                    BufferSize::Fixed(PREFERRED_BUFFER_FRAMES.clamp(*min, *max))
                }
                SupportedBufferSize::Unknown => BufferSize::Default,
            },
            ..supported.config()
        };

        let stream = match supported.sample_format() {
            SampleFormat::F32 => self.build::<f32>(&device, config),
            SampleFormat::F64 => self.build::<f64>(&device, config),
            SampleFormat::I16 => self.build::<i16>(&device, config),
            SampleFormat::I32 => self.build::<i32>(&device, config),
            SampleFormat::U16 => self.build::<u16>(&device, config),
            _ => Err(ErrorKind::UnsupportedConfig.into()),
        }?;
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    /// フェードアウトを始める。ストリームの解放は期限を過ぎてから `tick` で行う。
    pub fn stop(&mut self) {
        if !self.is_playing() {
            return;
        }
        self.shared.fading_out.store(true, Ordering::Relaxed);
        self.fade_deadline = Some(Instant::now() + FADE_OUT);
    }

    /// 期限を過ぎたフェードアウトを完了させる。まだ待つ必要があればその期限を返す。
    pub fn tick(&mut self, now: Instant) -> Option<Instant> {
        match self.fade_deadline {
            Some(deadline) if now >= deadline => {
                self.stream = None;
                self.fade_deadline = None;
                None
            }
            pending => pending,
        }
    }

    fn build<T>(&self, device: &Device, config: StreamConfig) -> Result<Stream, Error>
    where
        T: SizedSample + FromSample<f32>,
    {
        let shared = Arc::clone(&self.shared);
        let channels = config.channels as usize;
        let coef = 1.0 - (-1.0 / (GAIN_TIME_CONSTANT * config.sample_rate as f32)).exp();
        let mut noise = BrownNoise::new(seed());
        // 0 から始めることで再生開始がフェードインになる
        let mut gain = 0.0f32;

        device.build_output_stream(
            config,
            move |data: &mut [T], _| {
                let target = shared.target_gain();
                for frame in data.chunks_mut(channels) {
                    gain += (target - gain) * coef;
                    let sample = (noise.next_sample() * gain).clamp(-1.0, 1.0);
                    frame.fill(T::from_sample(sample));
                }
            },
            |err| eprintln!("audio stream error: {err}"),
            None,
        )
    }
}

fn seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1)
}
