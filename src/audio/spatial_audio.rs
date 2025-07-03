use std::cell::RefCell;

use std::time::{Duration, SystemTime};
use rodio::{OutputStream, Sink, Source};
use rand;

/// Complex oscillator with spatial positioning
pub struct SpatialOscillator {
    frequency: f32,
    amplitude: f32,
    sample_rate: u32,
    samples_generated: usize,
    total_samples: usize,
    phase: f32,
    fade_in_samples: usize,
    fade_out_samples: usize,
}

impl SpatialOscillator {
    pub fn new(frequency: f32, amplitude: f32, duration: f32) -> Self {
        let sample_rate = 44100;
        let total_samples = (sample_rate as f32 * duration) as usize;
        let fade_samples = (sample_rate as f32 * 0.05) as usize; // 50ms fade
        
        Self {
            frequency,
            amplitude,
            sample_rate,
            samples_generated: 0,
            total_samples,
            phase: 0.0,
            fade_in_samples: fade_samples,
            fade_out_samples: fade_samples,
        }
    }
}

impl Iterator for SpatialOscillator {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_generated >= self.total_samples {
            return None;
        }

        // Generate sine wave
        let sample = (self.phase * 2.0 * std::f32::consts::PI).sin();
        
        // Apply envelope (fade in/out)
        let envelope = if self.samples_generated < self.fade_in_samples {
            self.samples_generated as f32 / self.fade_in_samples as f32
        } else if self.samples_generated > self.total_samples - self.fade_out_samples {
            let remaining = self.total_samples - self.samples_generated;
            remaining as f32 / self.fade_out_samples as f32
        } else {
            1.0
        };
        
        let final_sample = sample * self.amplitude * envelope;
        
        // Update phase
        self.phase += self.frequency / self.sample_rate as f32;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        self.samples_generated += 1;
        Some(final_sample)
    }
}

impl Source for SpatialOscillator {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.total_samples - self.samples_generated)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.total_samples as f32 / self.sample_rate as f32))
    }
}

/// Continuous drone oscillator that never ends
pub struct DroneOscillator {
    frequency: f32,
    amplitude: f32,
    sample_rate: u32,
    phase: f32,
    time: f32,
}

impl DroneOscillator {
    pub fn new(frequency: f32, amplitude: f32) -> Self {
        Self {
            frequency,
            amplitude,
            sample_rate: 44100,
            phase: 0.0,
            time: 0.0,
        }
    }
}

impl Iterator for DroneOscillator {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Generate base sine wave
        let base_wave = (self.phase * 2.0 * std::f32::consts::PI).sin();
        
        // Add subtle modulation for more interesting drone
        let mod_freq = 0.1; // Very slow modulation
        let modulation = (self.time * mod_freq * 2.0 * std::f32::consts::PI).sin() * 0.1;
        
        // Combine base wave with modulation
        let sample = base_wave * (1.0 + modulation) * self.amplitude;
        
        // Update phase and time
        self.phase += self.frequency / self.sample_rate as f32;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        self.time += 1.0 / self.sample_rate as f32;
        
        Some(sample)
    }
}

impl Source for DroneOscillator {
    fn current_frame_len(&self) -> Option<usize> {
        None // Infinite duration
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Infinite duration
    }
}

/// Spatial audio mapping configuration
#[derive(Clone)]
pub struct SpatialConfig {
    pub min_frequency: f32,
    pub max_frequency: f32,
    pub min_amplitude: f32,
    pub max_amplitude: f32,
    pub voice_duration: f32,
    pub max_voices: usize,
    pub grid_range: i32, // How far from center to map
    pub drone_frequency: f32,
    pub drone_amplitude: f32,
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            min_frequency: 200.0,
            max_frequency: 2000.0,
            min_amplitude: 0.02,
            max_amplitude: 0.08,
            voice_duration: 1.5,
            max_voices: 20,
            grid_range: 50,
            drone_frequency: 65.0, // Low bass drone
            drone_amplitude: 0.03,
        }
    }
}

/// Individual voice with its own sink for true polyphony
struct Voice {
    sink: Sink,
    _frequency: f32,
    _amplitude: f32,
    start_time: SystemTime,
    cell_position: (i32, i32),
}

/// Advanced spatial audio system with true polyphony and background drone
struct SpatialAudioState {
    voices: Vec<Voice>,
    drone_sink: Option<Sink>,
    _output_stream: Option<OutputStream>,
    output_handle: Option<rodio::OutputStreamHandle>,
    enabled: bool,
    config: SpatialConfig,
    last_update: SystemTime,
    population: usize,
    milestone_cooldown: SystemTime,
    camera_x: f32,
    camera_y: f32,
}

impl SpatialAudioState {
    fn new() -> Self {
        let (_output_stream, output_handle) = OutputStream::try_default().ok().unzip();
        
        let has_audio = output_handle.is_some();
        if has_audio {
            println!("🎵 Polyphonic spatial audio system with background drone initialized successfully!");
        } else {
            println!("⚠️  Spatial audio system failed to initialize - no sound will be played");
        }
        
        Self {
            voices: Vec::new(),
            drone_sink: None,
            _output_stream,
            output_handle,
            enabled: has_audio,
            config: SpatialConfig::default(),
            last_update: SystemTime::now(),
            population: 0,
            milestone_cooldown: SystemTime::now(),
            camera_x: 0.0,
            camera_y: 0.0,
        }
    }
    
    fn start_background_drone(&mut self) {
        if !self.enabled || self.output_handle.is_none() {
            return;
        }
        
        if let Some(ref output_handle) = self.output_handle {
            if let Ok(sink) = Sink::try_new(output_handle) {
                let drone = DroneOscillator::new(
                    self.config.drone_frequency, 
                    self.config.drone_amplitude
                );
                sink.append(drone);
                self.drone_sink = Some(sink);
                println!("🎵 Background drone started at {:.1}Hz", self.config.drone_frequency);
                println!("🎵 Drone sink created and audio source added");
            }
        }
    }
    
    fn stop_background_drone(&mut self) {
        if let Some(drone_sink) = self.drone_sink.take() {
            drone_sink.stop();
            println!("🎵 Background drone stopped");
        }
    }
    
    fn coord_to_frequency(&self, x: i32, _y: i32) -> f32 {
        // Map X coordinate to frequency (relative to camera)
        let relative_x = x as f32 - self.camera_x;
        let normalized_x = (relative_x / self.config.grid_range as f32).clamp(-1.0, 1.0);
        
        // Convert to 0-1 range
        let freq_factor = (normalized_x + 1.0) / 2.0;
        
        self.config.min_frequency + freq_factor * (self.config.max_frequency - self.config.min_frequency)
    }
    
    fn coord_to_amplitude(&self, _x: i32, y: i32) -> f32 {
        // Map Y coordinate to amplitude (relative to camera)
        let relative_y = y as f32 - self.camera_y;
        let normalized_y = (relative_y / self.config.grid_range as f32).clamp(-1.0, 1.0);
        
        // Convert to 0-1 range (invert so bottom = louder)
        let amp_factor = (-normalized_y + 1.0) / 2.0;
        
        self.config.min_amplitude + amp_factor * (self.config.max_amplitude - self.config.min_amplitude)
    }
    
    fn update_camera_position(&mut self, camera_x: f32, camera_y: f32) {
        self.camera_x = camera_x;
        self.camera_y = camera_y;
    }
    
    fn process_cells(&mut self, alive_cells: &[(i32, i32)]) {
        let now = SystemTime::now();
        
        // Throttle updates to prevent audio overload
        if now.duration_since(self.last_update).unwrap_or(Duration::ZERO) < Duration::from_millis(100) {
            return;
        }
        self.last_update = now;
        
        if !self.enabled || self.output_handle.is_none() {
            return;
        }
        
        // Start drone if not already playing
        if self.drone_sink.is_none() {
            self.start_background_drone();
        } else {
            // Check if drone is still playing
            if let Some(ref drone_sink) = self.drone_sink {
                if drone_sink.empty() {
                    println!("🎵 Drone stopped, restarting...");
                    self.start_background_drone();
                }
            }
        }
        
        // Clean up finished voices
        self.cleanup_finished_voices();
        
        // Limit the number of cells we process for performance
        let cells_to_process: Vec<_> = alive_cells.iter()
            .filter(|(x, y)| {
                // Only process cells near the camera
                let dx = (*x as f32 - self.camera_x).abs();
                let dy = (*y as f32 - self.camera_y).abs();
                dx <= self.config.grid_range as f32 && dy <= self.config.grid_range as f32
            })
            .take(self.config.max_voices)
            .collect();
        
        let mut new_voices = 0;
        
        for &(x, y) in cells_to_process {
            // Skip if we already have a recent voice for this cell
            if self.voices.iter().any(|voice| {
                voice.cell_position == (x, y) && 
                now.duration_since(voice.start_time).unwrap_or(Duration::ZERO) 
                < Duration::from_secs_f32(self.config.voice_duration * 0.5)
            }) {
                continue;
            }
            
            // Don't create too many voices at once
            if self.voices.len() >= self.config.max_voices {
                break;
            }
            
            // Create new voice for this cell
            let frequency = self.coord_to_frequency(x, y);
            let amplitude = self.coord_to_amplitude(x, y);
            
            // Map X distance to stereo pan (-1.0 = left, 1.0 = right)
            let dx = (x as f32 - self.camera_x).clamp(-self.config.grid_range as f32, self.config.grid_range as f32);
            let pan = dx / self.config.grid_range as f32; // Normalised [-1,1]

            if let Some(ref output_handle) = self.output_handle {
                if let Ok(sink) = Sink::try_new(output_handle) {
                    // Add subtle detune per voice for illbient flavour
                    let detune = (rand::random::<f32>() - 0.5) * 15.0; // ±7.5 Hz variance
                    let oscillator = PanOscillator::new(frequency + detune, amplitude, self.config.voice_duration, pan);
                    sink.append(oscillator);
                    
                    let voice = Voice {
                        sink,
                        _frequency: frequency,
                        _amplitude: amplitude,
                        start_time: now,
                        cell_position: (x, y),
                    };
                    
                    self.voices.push(voice);
                    new_voices += 1;
                    
                    // Limit new voices per update
                    if new_voices >= 5 {
                        break;
                    }
                }
            }
        }
        
        if new_voices > 0 {
            println!("🎵 Created {} polyphonic voices (total active: {})", new_voices, self.voices.len());
        }
    }
    
    fn cleanup_finished_voices(&mut self) {
        let now = SystemTime::now();
        let voice_lifetime = Duration::from_secs_f32(self.config.voice_duration + 0.5);
        
        self.voices.retain(|voice| {
            let is_alive = now.duration_since(voice.start_time).unwrap_or(Duration::ZERO) < voice_lifetime;
            let is_playing = !voice.sink.empty();
            
            is_alive && is_playing
        });
    }
    
    fn update_population(&mut self, new_population: usize) {
        let old_population = self.population;
        self.population = new_population;
        
        // Play milestone sounds
        if self.should_play_milestone(old_population, new_population) {
            let now = SystemTime::now();
            if now.duration_since(self.milestone_cooldown).unwrap_or(Duration::ZERO) > Duration::from_secs(2) {
                self.milestone_cooldown = now;
                
                let frequency = 600.0 + (new_population as f32).log2() * 100.0;
                let frequency = frequency.clamp(600.0, 1200.0);
                
                if let Some(ref output_handle) = self.output_handle {
                    if let Ok(milestone_sink) = Sink::try_new(output_handle) {
                        let milestone_sound = SpatialOscillator::new(frequency, 0.15, 0.4);
                        milestone_sink.append(milestone_sound);
                        milestone_sink.detach(); // Let it play independently
                        println!("🔊 Population milestone: {} cells -> {:.0}Hz", new_population, frequency);
                    }
                }
            }
        }
    }
    
    fn should_play_milestone(&self, old_pop: usize, new_pop: usize) -> bool {
        if new_pop < 16 || old_pop >= new_pop {
            return false;
        }
        
        // Check if we crossed a power of 2 threshold
        let old_threshold = if old_pop == 0 { 0 } else { self.prev_power_of_2(old_pop) };
        let new_threshold = self.prev_power_of_2(new_pop);
        
        new_threshold > old_threshold && new_threshold >= 16
    }
    
    fn prev_power_of_2(&self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        1 << (63 - n.leading_zeros() - 1)
    }
    
    fn play_cell_sound(&self, frequency: f32, duration: f32, amplitude: f32) {
        if !self.enabled {
            return;
        }
        
        if let Some(ref output_handle) = self.output_handle {
            if let Ok(sink) = Sink::try_new(output_handle) {
                let oscillator = PanOscillator::new(frequency, amplitude, duration, 0.0);
                sink.append(oscillator);
                sink.detach();
            }
        }
    }
    
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled && self.output_handle.is_some();
        if !self.enabled {
            // Stop all voices and drone
            self.voices.clear();
            self.stop_background_drone();
        } else {
            // Restart drone when re-enabled
            self.start_background_drone();
        }
        println!("🔊 Polyphonic spatial audio {}", if self.enabled { "enabled" } else { "disabled" });
    }
    
    fn has_audio(&self) -> bool {
        self.output_handle.is_some()
    }
    
    fn get_voice_count(&self) -> usize {
        self.voices.len()
    }
}

// Thread-local spatial audio manager
thread_local! {
    static SPATIAL_AUDIO_STATE: RefCell<Option<SpatialAudioState>> = RefCell::new(None);
}

/// Initialize the spatial audio system
pub fn init_spatial_audio() {
    SPATIAL_AUDIO_STATE.with(|state| {
        *state.borrow_mut() = Some(SpatialAudioState::new());
    });
}

/// Update camera position for spatial mapping
pub fn update_camera_position(camera_x: f32, camera_y: f32) {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref mut audio_state) = *state.borrow_mut() {
            audio_state.update_camera_position(camera_x, camera_y);
        }
    });
}

/// Process all alive cells for spatial audio
pub fn process_spatial_audio(alive_cells: &[(i32, i32)]) {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref mut audio_state) = *state.borrow_mut() {
            audio_state.process_cells(alive_cells);
        }
    });
}

/// Update population for milestone detection
pub fn update_spatial_population(population: usize) {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref mut audio_state) = *state.borrow_mut() {
            audio_state.update_population(population);
        }
    });
}

/// Play individual cell sounds (birth/death)
pub fn play_spatial_cell_birth() {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref audio_state) = *state.borrow() {
            audio_state.play_cell_sound(800.0, 0.08, 0.06);
        }
    });
}

pub fn play_spatial_cell_death() {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref audio_state) = *state.borrow() {
            audio_state.play_cell_sound(300.0, 0.05, 0.04);
        }
    });
}

/// Toggle spatial audio enabled/disabled
pub fn toggle_spatial_audio() -> bool {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref mut audio_state) = *state.borrow_mut() {
            let current_enabled = audio_state.enabled;
            audio_state.set_enabled(!current_enabled);
            return !current_enabled;
        }
        false
    })
}

/// Check if spatial audio is available
pub fn has_spatial_audio() -> bool {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref audio_state) = *state.borrow() {
            return audio_state.has_audio();
        }
        false
    })
}

/// Get current number of active voices (for debugging)
pub fn get_active_voice_count() -> usize {
    SPATIAL_AUDIO_STATE.with(|state| {
        if let Some(ref audio_state) = *state.borrow() {
            return audio_state.get_voice_count();
        }
        0
    })
}

/// Simple stereo oscillator with constant pan
pub struct PanOscillator {
    inner: SpatialOscillator,
    pan: f32, // -1.0 (left) .. 0.0 (center) .. 1.0 (right)
}

impl PanOscillator {
    pub fn new(freq: f32, amp: f32, dur: f32, pan: f32) -> Self {
        Self {
            inner: SpatialOscillator::new(freq, amp, dur),
            pan: pan.clamp(-1.0, 1.0),
        }
    }
}

impl Iterator for PanOscillator {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Emit queued right-channel sample first (interleaved stereo)
        if let Some(sample) = THREAD_LOCAL_RIGHT_SAMPLE.with(|cell| cell.borrow_mut().take()) {
            return Some(sample);
        }

        // Generate a new mono sample and split into L/R
        if let Some(sample) = self.inner.next() {
            let pan = (self.pan + 1.0) * 0.5; // 0..1
            let left_gain = (1.0 - pan).sqrt();
            let right_gain = pan.sqrt();

            THREAD_LOCAL_RIGHT_SAMPLE.with(|cell| cell.replace(Some(sample * right_gain)));
            Some(sample * left_gain)
        } else {
            None
        }
    }
}

// Thread-local buffer to hold the second channel sample between iterator calls
thread_local! {
    static THREAD_LOCAL_RIGHT_SAMPLE: RefCell<Option<f32>> = RefCell::new(None);
}

impl Source for PanOscillator {
    fn current_frame_len(&self) -> Option<usize> {
        // Each frame is 2 samples; inner knows remaining mono samples
        self.inner.current_frame_len().map(|m| m * 2)
    }

    fn channels(&self) -> u16 { 2 }

    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }

    fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
} 