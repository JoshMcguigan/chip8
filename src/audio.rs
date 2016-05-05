use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::Sdl;
use std::time::{Duration, Instant};

/// Represents a square wave.
pub struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = match self.phase {
                0.0...0.5 => self.volume,
                _ => -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

/// Represents a device for making a 'beep' noise.
pub struct Beeper {
    pub device: AudioDevice<SquareWave>,
    duration: Duration,
    start: Instant,
}

impl Beeper {
    /// Constructs a Beeper using the give SDL context.
    pub fn new(context: &Sdl, duration: Duration) -> Self {
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };
        let sub = context.audio().unwrap();
        let device = sub.open_playback(None, &desired_spec, |spec| {
            // Show obtained AudioSpec
            debug!("{:?}", spec);

            // initialize the audio callback
            SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25
            }
        }).unwrap();

        Beeper {
            device: device,
            duration: duration,
            start: Instant::now(),
        }
    }

    /// Starts the beep if necessary, and stops the beep if the beep duration
    /// has passed.
    pub fn set_beep(&mut self, enable: bool) {
        if enable {
            self.start = Instant::now();
            self.device.resume();
        } else {
            if self.duration <= Instant::now().duration_since(self.start) {
                self.device.pause();
            }
        }
    }
}
