//! オリジナル合成効果音。圧縮音声デコーダや外部アセットは使用しない。
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Source};

pub struct Audio {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sounds: Vec<SamplesBuffer<f32>>,
    volume: f32,
}
#[derive(Clone, Copy)]
pub enum Sound {
    Fire,
    Pistol,
    Sniper,
    Reload,
    Hit,
    Head,
    Kill,
    Footstep,
    Land,
}
impl Audio {
    pub fn new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default().map_err(|e| e.to_string())?;
        let sounds = [
            (110., 0.075, 0.7),
            (240., 0.06, 0.65),
            (75., 0.18, 0.8),
            (350., 0.13, 0.4),
            (800., 0.05, 0.0),
            (1400., 0.07, 0.0),
            (520., 0.16, 0.0),
            (92., 0.085, 0.72),
            (58., 0.16, 0.78),
        ]
        .into_iter()
        .map(|(frequency, duration, noise)| {
            let count = (duration * 44100.) as usize;
            let mut seed = 17u32;
            let samples: Vec<f32> = (0..count)
                .map(|i| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    let t = i as f32 / 44100.;
                    let envelope = (1. - i as f32 / count as f32).powi(3) * (t / 0.002).min(1.);
                    let random = (seed >> 8) as f32 / 8388608. - 1.;
                    let tone = (std::f32::consts::TAU * frequency * t).sin();
                    0.18 * envelope * (tone * (1. - noise) + random * noise)
                })
                .collect();
            SamplesBuffer::new(1, 44100, samples)
        })
        .collect();
        Ok(Self {
            _stream: stream,
            handle,
            sounds,
            volume: 1.0,
        })
    }
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }
    pub fn play(&self, sound: Sound) {
        if let Err(e) = self
            .handle
            .play_raw(self.sounds[sound as usize].clone().amplify(self.volume))
        {
            eprintln!("audio playback: {e}");
        }
    }
}
