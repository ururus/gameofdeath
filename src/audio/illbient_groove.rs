use std::time::{Duration, Instant};
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use rand;
use crate::audio::ddsp_engine::GameStateFeatures;
// (No Bevy types needed in this module; accessed via NonSendMut from outside)

/// Simple kick drum oscillator: decaying sine with pitch drop.
struct KickOsc {
    sample_rate: u32,
    total_samples: usize,
    generated: usize,
    phase: f32,
}

impl KickOsc {
    fn new() -> Self {
        let sample_rate = 44100;
        let dur = 0.5; // 500 ms
        Self {
            sample_rate,
            total_samples: (dur * sample_rate as f32) as usize,
            generated: 0,
            phase: 0.0,
        }
    }
}

impl Iterator for KickOsc {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.generated >= self.total_samples { return None; }
        let t = self.generated as f32 / self.sample_rate as f32;
        // Exponential pitch drop from 120 Hz → 40 Hz over 100 ms
        let freq = 120.0 * (0.25f32).powf((t / 0.1).min(1.0));
        self.phase += freq / self.sample_rate as f32;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        // Exponential decay envelope
        let amp = (-(6.0 * t)).exp();
        self.generated += 1;
        Some((self.phase * 2.0 * std::f32::consts::PI).sin() * amp * 0.8)
    }
}

impl Source for KickOsc {
    fn current_frame_len(&self) -> Option<usize> { Some(self.total_samples - self.generated) }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { Some(Duration::from_secs_f32(self.total_samples as f32 / self.sample_rate as f32)) }
}

/// Hi-hat: burst of white noise with fast decay & band-pass (approximated by simple envelope).
struct HatOsc {
    sample_rate: u32,
    total_samples: usize,
    generated: usize,
}

impl HatOsc { fn new() -> Self { let sr=44100; let dur=0.15; Self{sample_rate:sr,total_samples:(dur*sr as f32) as usize,generated:0} } }

impl Iterator for HatOsc {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.generated >= self.total_samples {
            return None;
        }
        let t = self.generated as f32 / self.sample_rate as f32;
        let env = (1.0 - t * 8.0).max(0.0).powf(3.0);
        // Simple band-pass via ring-mod with 8 kHz carrier
        let carrier_phase = t * 8000.0 * 2.0 * std::f32::consts::PI;
        let modulator = carrier_phase.sin();
        let raw = (rand::random::<f32>() * 2.0 - 1.0) * modulator;
        let sample = raw * env * 0.4;
        self.generated += 1;
        Some(sample)
    }
}

impl Source for HatOsc { fn current_frame_len(&self)->Option<usize>{Some(self.total_samples-self.generated)} fn channels(&self)->u16{1} fn sample_rate(&self)->u32{self.sample_rate} fn total_duration(&self)->Option<Duration>{Some(Duration::from_secs_f32(self.total_samples as f32/self.sample_rate as f32))} }

/// Deep sine sub-bass
struct BassOsc {
    sample_rate: u32,
    total_samples: usize,
    generated: usize,
    phase: f32,
    freq: f32,
}

impl BassOsc {
    fn new(freq: f32) -> Self {
        let sample_rate = 44100;
        let dur = 0.7; // 700 ms decay
        Self {
            sample_rate,
            total_samples: (dur * sample_rate as f32) as usize,
            generated: 0,
            phase: 0.0,
            freq,
        }
    }
}

impl Iterator for BassOsc {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.generated >= self.total_samples { return None; }
        let t = self.generated as f32 / self.sample_rate as f32;
        let env = (-(3.0 * t)).exp();
        self.phase += self.freq / self.sample_rate as f32;
        if self.phase >= 1.0 { self.phase -= 1.0; }
        self.generated += 1;
        Some((self.phase * 2.0 * std::f32::consts::PI).sin() * env * 0.8)
    }
}

impl Source for BassOsc {
    fn current_frame_len(&self) -> Option<usize> { Some(self.total_samples - self.generated) }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { Some(Duration::from_secs_f32(self.total_samples as f32 / self.sample_rate as f32)) }
}

/// Runtime state managed as a Bevy resource.
pub struct IllbientGroove {
    stream: OutputStream,
    handle: OutputStreamHandle,
    bpm: f32,
    next_beat: Instant,
    step: u8,
}

impl IllbientGroove {
    pub fn new(bpm: f32) -> Self {
        let (stream, handle) = OutputStream::try_default().expect("audio device");
        let now = Instant::now();
        Self { stream, handle, bpm, next_beat: now, step: 0 }
    }

    fn beat_duration(&self) -> Duration { Duration::from_secs_f32(60.0 / self.bpm as f32) }

    pub fn update(&mut self, features: &GameStateFeatures, root_hz: Option<f32>) {
        let now = Instant::now();
        while now >= self.next_beat {
            self.trigger_step(features, root_hz);
            self.next_beat += self.beat_duration();
            self.step = self.step.wrapping_add(1);
        }
    }

    fn trigger_step(&mut self, features: &GameStateFeatures, root_hz: Option<f32>) {
        // Feature-driven pattern: activity controls hat density; chaos adds syncopation.
        let activity = features.activity;
        let chaos = features.chaos;
        // Kick on steps 0 & 8 always
        if self.step % 8 == 0 { self.play_kick(); }
        // Extra kick when activity high (>0.2) on off-beat
        if activity > 0.2 && self.step % 8 == 4 { self.play_kick(); }
        // Hi-hat probabilistic
        // Density and centroid add subtle swing (more hats on right side of board)
        let density = features.density;
        let centroid_x = features.centroid_x; // -1..1
        let hat_prob = 0.25 + activity*0.4 + chaos*0.2 + density*0.15 + centroid_x.abs()*0.1; // 0.25-1.1
        if rand::random::<f32>() < hat_prob { self.play_hat(); }

        // Sub-bass follows kicks and scale root
        if self.step % 8 == 0 {
            if let Some(root) = root_hz {
                self.play_bass(root * 0.5); // sub-octave of root
            }
        }
    }

    fn play_kick(&self) {
        if let Ok(sink) = Sink::try_new(&self.handle) { sink.append(KickOsc::new()); sink.detach(); }
    }
    fn play_hat(&self) { if let Ok(sink)=Sink::try_new(&self.handle){ sink.append(HatOsc::new()); sink.detach(); } }
    fn play_bass(&self, freq: f32) { if let Ok(sink)=Sink::try_new(&self.handle){ sink.append(BassOsc::new(freq)); sink.detach(); } }
}

// (Groove resource is inserted in `main.rs` via `insert_non_send_resource`; update is called from the audio system.)

// pub fn update_illbient_groove(
//     mut groove: ResMut<IllbientGroove>,
//     features: Res<crate::audio::hybrid_dungeon_synth::GameFeatureCache>,
// ) {
//     groove.update(&features.to_array());
// } 