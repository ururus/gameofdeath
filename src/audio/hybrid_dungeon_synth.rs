use std::f32::consts::PI;

use rodio::{Source, OutputStream, Sink};
use std::sync::Mutex;
use std::collections::HashMap;
// use std::time::{Duration, Instant}; // Not needed

/// Optimized cell matrix for efficient processing of large populations
#[derive(Clone)]
struct CellMatrix {
    grid: HashMap<(i32, i32), f32>, // Cell intensity map
    regions: [(f32, f32, f32); 16],  // 4x4 regions with (density, activity, complexity)
    total_cells: usize,
    max_cells_per_region: usize,
}

impl CellMatrix {
    fn new() -> Self {
        Self {
            grid: HashMap::new(),
            regions: [(0.0, 0.0, 0.0); 16],
            total_cells: 0,
            max_cells_per_region: 50, // Limit processing per region
        }
    }
    
    /// Update matrix from cell data with optimization for large populations
    fn update_from_cells(&mut self, cells: &[(i32, i32)], camera_x: f32, camera_y: f32, viewport_size: f32) {
        self.grid.clear();
        self.regions = [(0.0, 0.0, 0.0); 16];
        self.total_cells = cells.len();
        
        // For large populations, use spatial downsampling
        if cells.len() > 200 {
            self.update_with_spatial_grouping(cells, camera_x, camera_y, viewport_size);
        } else {
            self.update_direct(cells, camera_x, camera_y, viewport_size);
        }
    }
    
    fn update_direct(&mut self, cells: &[(i32, i32)], camera_x: f32, camera_y: f32, viewport_size: f32) {
        let region_size = viewport_size / 4.0; // 4x4 grid of regions
        
        for &(x, y) in cells {
            // Calculate intensity based on local density
            let intensity = self.calculate_local_intensity(x, y, cells);
            self.grid.insert((x, y), intensity);
            
            // Update region data
            let region_x = ((x as f32 - camera_x + viewport_size * 2.0) / region_size).floor() as usize;
            let region_y = ((y as f32 - camera_y + viewport_size * 2.0) / region_size).floor() as usize;
            
            if region_x < 4 && region_y < 4 {
                let region_idx = region_y * 4 + region_x;
                self.regions[region_idx].0 += 1.0; // density count
                self.regions[region_idx].1 += intensity; // activity
                self.regions[region_idx].2 += self.calculate_local_complexity(x, y, cells); // complexity
            }
        }
        
        // Normalize region values
        for region in &mut self.regions {
            region.0 /= self.max_cells_per_region as f32; // normalize density
            region.1 /= region.0.max(1.0); // normalize activity by density
            region.2 /= region.0.max(1.0); // normalize complexity by density
        }
    }
    
    fn update_with_spatial_grouping(&mut self, cells: &[(i32, i32)], camera_x: f32, camera_y: f32, viewport_size: f32) {
        let region_size = viewport_size / 4.0;
        let mut region_cells: [Vec<(i32, i32)>; 16] = Default::default();
        
        // Group cells by region
        for &(x, y) in cells {
            let region_x = ((x as f32 - camera_x + viewport_size * 2.0) / region_size).floor() as usize;
            let region_y = ((y as f32 - camera_y + viewport_size * 2.0) / region_size).floor() as usize;
            
            if region_x < 4 && region_y < 4 {
                let region_idx = region_y * 4 + region_x;
                region_cells[region_idx].push((x, y));
            }
        }
        
        // Process each region with cell limit
        for (idx, region_cell_list) in region_cells.iter().enumerate() {
            if region_cell_list.is_empty() {
                continue;
            }
            
            let cell_count = region_cell_list.len().min(self.max_cells_per_region);
            let sample_step = if region_cell_list.len() > self.max_cells_per_region {
                region_cell_list.len() / self.max_cells_per_region
            } else {
                1
            };
            
            let mut total_intensity = 0.0;
            let mut total_complexity = 0.0;
            
            // Sample cells from this region
            for i in (0..region_cell_list.len()).step_by(sample_step).take(cell_count) {
                let (x, y) = region_cell_list[i];
                let intensity = self.calculate_local_intensity(x, y, region_cell_list);
                let complexity = self.calculate_local_complexity(x, y, region_cell_list);
                
                self.grid.insert((x, y), intensity);
                total_intensity += intensity;
                total_complexity += complexity;
            }
            
            // Update region summary
            self.regions[idx] = (
                cell_count as f32 / self.max_cells_per_region as f32, // density
                total_intensity / cell_count as f32, // avg activity
                total_complexity / cell_count as f32, // avg complexity
            );
        }
    }
    
    fn calculate_local_intensity(&self, x: i32, y: i32, cells: &[(i32, i32)]) -> f32 {
        let mut neighbor_count = 0;
        for &(cx, cy) in cells {
            let dist_sq = (cx - x).pow(2) + (cy - y).pow(2);
            if dist_sq <= 9 { // 3x3 neighborhood
                neighbor_count += 1;
            }
        }
        (neighbor_count as f32 / 9.0).min(1.0)
    }
    
    fn calculate_local_complexity(&self, x: i32, y: i32, cells: &[(i32, i32)]) -> f32 {
        let mut pattern_score = 0.0;
        let mut neighbor_positions = Vec::new();
        
        // Collect nearby neighbors
        for &(cx, cy) in cells {
            let dist_sq = (cx - x).pow(2) + (cy - y).pow(2);
            if dist_sq <= 25 { // 5x5 neighborhood
                neighbor_positions.push((cx - x, cy - y));
            }
        }
        
        // Analyze pattern complexity
        if neighbor_positions.len() > 2 {
            // Check for regular patterns vs irregular distributions
            let mut regularity_score = 0.0;
            for &(dx, dy) in &neighbor_positions {
                for &(dx2, dy2) in &neighbor_positions {
                    if (dx, dy) != (dx2, dy2) {
                        let dist = ((dx - dx2).pow(2) + (dy - dy2).pow(2)) as f32;
                        if dist > 0.0 {
                            regularity_score += 1.0 / dist.sqrt();
                        }
                    }
                }
            }
            pattern_score = (regularity_score / neighbor_positions.len() as f32).min(1.0);
        }
        
        pattern_score
    }
}

/// Hybrid Dungeon Synth Engine
/// Combines procedural synthesis, sample-based elements, and neural parameter modulation
pub struct HybridDungeonSynthEngine {
    // Core synthesis layers
    cathedral_drone: CathedralDroneLayer,
    medieval_samples: MedievalSampleBank,
    neural_modulator: SimpleNeuralModulator,
    crypt_reverb: CryptReverb,
    tape_saturation: TapeSaturation,
    
    // State management
    sample_rate: f32,
    current_features: [f32; 8],
    update_counter: usize,
    
    // Optimized cell processing
    cell_matrix: CellMatrix,
    
    // Continuous evolution system
    pattern_memory: [f32; 16],        // Remember recent patterns
    evolution_phase: f32,             // Slow-changing phase for variation
    micro_variation_timer: f32,       // Fast micro-variations
    harmonic_drift: [f32; 4],         // Slowly drifting harmonic content
    scale_evolution: f32,             // Gradually shifts between musical scales
    temporal_complexity: f32,         // Tracks pattern complexity over time
    
    // Enhanced modulation system
    _bass_pattern_memory: [f32; 8],    // Memory for bass pattern variations
    _rhythm_phase: f32,                // For rhythmic bass variations
    _spatial_modulation: [f32; 16],    // Modulation based on cell regions
    
    // Hybrid control parameters
    synthesis_mix: f32,      // 0.0 = all samples, 1.0 = all synthesis
    _neural_influence: f32,   // How much neural modulation affects parameters
    _medieval_authenticity: f32, // Controls how "authentic" vs "atmospheric" the sound is
    
    // Audio optimization
    master_gain: f32,        // Adaptive gain control
    peak_detector: f32,      // Peak level detector for automatic gain
    compression_ratio: f32,  // Dynamic compression
    // --- Phase-2 additions ---
    scale_notes: [f32; 7],      // Current diatonic scale (Hz)
    last_milestone_generation: u64, // For 100-generation bell trigger
}

/// Cathedral Drone Layer - Deep bass foundation with neural modulation
struct CathedralDroneLayer {
    oscillators: [DroneOscillator; 4], // Added 4th oscillator for sub-bass
    base_frequencies: [f32; 4],       // Root frequencies derived from current musical scale
    modulation_phase: f32,
    resonance_filter: ResonanceFilter,
    // Enhanced bass modulation
    bass_modulation_patterns: [f32; 8], // Different bass patterns
    current_pattern: usize,
    pattern_transition_timer: f32,
    _sub_bass_phase: f32,              // Deep sub-bass oscillator
    rhythm_trigger: f32,              // Rhythmic bass variations
}

#[derive(Copy, Clone)]
struct DroneOscillator {
    phase: f32,
    frequency: f32,
    amplitude: f32,
    harmonic_weights: [f32; 8],
}

struct ResonanceFilter {
    state: [f32; 4],
    cutoff: f32,
    resonance: f32,
}

impl CathedralDroneLayer {
    fn new(_sample_rate: f32) -> Self {
        let base_frequencies = [32.7, 65.4, 82.4, 98.0]; // C1, C2, E2, G2 - extended minor triad with sub-bass
        let mut oscillators = [DroneOscillator {
            phase: 0.0,
            frequency: 65.4,
            amplitude: 0.7,
            harmonic_weights: [1.0, 0.8, 0.6, 0.4, 0.3, 0.2, 0.15, 0.1],
        }; 4];
        
        for (i, osc) in oscillators.iter_mut().enumerate() {
            osc.frequency = base_frequencies[i];
            osc.amplitude = match i {
                0 => 0.9, // Sub-bass stronger
                1 => 0.7, // Main bass
                2 => 0.6, // Mid bass
                3 => 0.5, // Upper bass
                _ => 0.4, // Any additional oscillators
            };
        }
        
        Self {
            oscillators,
            modulation_phase: 0.0,
            resonance_filter: ResonanceFilter {
                state: [0.0; 4],
                cutoff: 200.0,
                resonance: 0.7,
            },
            // Initialize bass modulation system
            bass_modulation_patterns: [0.2, 0.5, 0.8, 0.3, 0.7, 0.4, 0.9, 0.6], // Different pattern intensities
            current_pattern: 0,
            pattern_transition_timer: 0.0,
            _sub_bass_phase: 0.0,
            rhythm_trigger: 0.0,
            base_frequencies,
        }
    }
    
    fn process(&mut self, sample_rate: f32) -> f32 {
        let mut output = 0.0;
        
        // Update pattern transition
        self.pattern_transition_timer += 1.0 / sample_rate;
        if self.pattern_transition_timer >= 4.0 { // Change pattern every 4 seconds
            self.current_pattern = (self.current_pattern + 1) % 8;
            self.pattern_transition_timer = 0.0;
        }
        
        // Current pattern modulation
        let pattern_intensity = self.bass_modulation_patterns[self.current_pattern];
        let pattern_transition = (self.pattern_transition_timer / 4.0).min(1.0);
        let next_pattern_intensity = self.bass_modulation_patterns[(self.current_pattern + 1) % 8];
        let current_pattern_mod = pattern_intensity * (1.0 - pattern_transition) + next_pattern_intensity * pattern_transition;
        
        // Rhythmic modulation
        self.rhythm_trigger += 2.5 / sample_rate; // ~2.5 Hz rhythm
        let rhythm_mod = if self.rhythm_trigger >= 1.0 {
            self.rhythm_trigger -= 1.0;
            current_pattern_mod * 0.3 // Rhythmic accent
        } else {
            0.0
        };
        
        // Use externally-set base frequencies for musical adaptability
        let base_frequencies = self.base_frequencies;
        
        for (i, osc) in self.oscillators.iter_mut().enumerate() {
            let mut osc_output = 0.0;
            
            // Different processing for sub-bass vs other oscillators
            if i == 0 { // Sub-bass oscillator
                // Simple but powerful sub-bass
                let sub_fundamental = (osc.phase * 2.0 * PI).sin();
                let sub_harmonic2 = (osc.phase * 4.0 * PI).sin() * 0.3;
                osc_output = (sub_fundamental + sub_harmonic2) * (1.0 + rhythm_mod);
            } else {
                // Rich harmonic content for other oscillators
                for (h, &weight) in osc.harmonic_weights.iter().enumerate() {
                    let _harmonic_freq = osc.frequency * (h + 1) as f32;
                    let harmonic_phase = osc.phase * (h + 1) as f32;
                    let harmonic_content = (harmonic_phase * 2.0 * PI).sin() * weight;
                    
                    // Apply pattern modulation to mid/high harmonics
                    let pattern_influence = if h > 2 { current_pattern_mod } else { 1.0 };
                    osc_output += harmonic_content * pattern_influence;
                }
            }
            
            // Modulation with pattern influence
            let mod_amount = (self.modulation_phase * 2.0 * PI).sin() * 0.05 * current_pattern_mod;
            osc_output *= osc.amplitude * (1.0 + mod_amount);
            
            // Apply soft limiting per oscillator to prevent clipping
            let threshold = 0.7;
            if osc_output.abs() > threshold {
                let sign = osc_output.signum();
                let excess = osc_output.abs() - threshold;
                let compressed = threshold + excess * 0.2; // Gentle compression
                osc_output = sign * compressed.min(0.85);
            }
            output += osc_output;
            
            osc.phase += osc.frequency / sample_rate;
            if osc.phase >= 1.0 {
                osc.phase -= 1.0;
            }
        }
        
        self.modulation_phase += 0.2 / sample_rate;
        if self.modulation_phase >= 1.0 {
            self.modulation_phase -= 1.0;
        }
        
        // Reduced overall gain and better filtering to prevent clipping
        let filtered_output = self.resonance_filter.process(output * 0.15); // Reduced from 0.3
        
        // Apply soft limiting to final output
        let threshold = 0.7;
        if filtered_output.abs() > threshold {
            let sign = filtered_output.signum();
            let excess = filtered_output.abs() - threshold;
            let compressed = threshold + excess * 0.2;
            sign * compressed.min(0.85)
        } else {
            filtered_output
        }
    }
    

    
    fn update_parameters(&mut self, population: f32, darkness: f32, neural_mod: f32, cell_regions: &[(f32, f32, f32); 16]) {
        // Use current scale-derived base tones
        let base_frequencies = self.base_frequencies;
        let base_amp = 0.3 + population * 0.3; // Reduced base amplitude to prevent clipping
        let harmonic_brightness = 0.6 - darkness * 0.5 + neural_mod * 0.3;
        
        // Add breathing effect to the drone
        self.modulation_phase += 0.002;
        let breathing_mod = (self.modulation_phase.sin() * 0.5 + 0.5) * 0.1; // Reduced breathing intensity
        
        // Calculate regional activity influence
        let total_regional_activity: f32 = cell_regions.iter().map(|(d, a, _)| d * a).sum();
        let regional_complexity: f32 = cell_regions.iter().map(|(_, _, c)| *c).sum();
        let regional_mod = (total_regional_activity / 16.0).min(1.0);
        
        // Update bass pattern selection based on cell activity
        if regional_mod > 0.5 {
            // High activity - faster pattern changes
            self.pattern_transition_timer += 0.5 / 44100.0; // Accelerate pattern changes
        }
        
        // Modulate bass patterns based on regional complexity
        for (i, pattern) in self.bass_modulation_patterns.iter_mut().enumerate() {
            let region_influence = if i < 16 { cell_regions[i % 16].2 } else { 0.0 };
            *pattern = (*pattern * 0.9 + region_influence * 0.1).clamp(0.1, 1.0);
        }
        
        for (i, osc) in self.oscillators.iter_mut().enumerate() {
            // Different amplitude scaling for different oscillators
            let osc_amp_base = match i {
                0 => base_amp * 0.8,  // Sub-bass
                1 => base_amp * 0.9,  // Main bass
                2 => base_amp * 0.7,  // Mid
                3 => base_amp * 0.6,  // Upper
                _ => base_amp * 0.5,
            };
            
            // Regional modulation affects amplitude
            let regional_influence = regional_mod * 0.2;
            osc.amplitude = (osc_amp_base + regional_influence) * (1.0 + breathing_mod);
            
            // Cell complexity affects frequency drift
            let complexity_drift = regional_complexity * 0.02; // Subtle frequency drift
            let darkness_offset = darkness * 3.0 * (i as f32 - 1.5); // Centered around 1.5
            osc.frequency = base_frequencies[i] * (1.0 + complexity_drift) + darkness_offset;
            
            // Cell activity affects harmonic content
            for (h, weight) in osc.harmonic_weights.iter_mut().enumerate() {
                let base_weight = 1.0 / (h as f32 + 1.0).sqrt();
                let harmonic_mod = harmonic_brightness * (1.0 + neural_mod * 0.3); // Reduced neural influence
                
                // Regional activity influences harmonic distribution
                let region_harmonic_mod = if h < 16 { cell_regions[h % 16].1 } else { 0.0 };
                let harmonic_emphasis = if h > 3 { 
                    harmonic_brightness * 1.5 + region_harmonic_mod * 0.3 
                } else { 
                    1.0 + region_harmonic_mod * 0.1 
                };
                
                *weight = (base_weight * harmonic_mod * harmonic_emphasis).clamp(0.05, 1.0);
            }
        }
        
        // Cell-responsive filter modulation
        let filter_mod = (neural_mod * 80.0).sin() * 20.0 + regional_mod * 15.0; // Regional influence on filter
        self.resonance_filter.cutoff = (100.0 + (1.0 - darkness) * 180.0 + neural_mod * 60.0 + filter_mod).clamp(50.0, 400.0);
        self.resonance_filter.resonance = (0.2 + darkness * 0.3 + regional_complexity * 0.1).clamp(0.1, 0.7);
    }

    /// Allow external code to change the fundamental notes used by the drone
    pub fn set_base_frequencies(&mut self, base: [f32; 4]) {
        self.base_frequencies = base;
    }
}

impl ResonanceFilter {
    fn process(&mut self, input: f32) -> f32 {
        let f = (self.cutoff * 2.0 * PI / 44100.0).min(0.99);
        let fb = self.resonance + self.resonance / (1.0 - f);
        
        self.state[0] += f * (input - self.state[0] + fb * (self.state[0] - self.state[1]));
        self.state[1] += f * (self.state[0] - self.state[1]);
        self.state[2] += f * (self.state[1] - self.state[2]);
        self.state[3] += f * (self.state[2] - self.state[3]);
        
        self.state[3]
    }
}

/// Medieval Sample Bank - Procedurally generated medieval instrument samples
struct MedievalSampleBank {
    lute_samples: Vec<SampleData>,
    bell_samples: Vec<SampleData>,
    current_voices: Vec<PlayingVoice>,
}

#[derive(Clone)]
struct SampleData {
    data: Vec<f32>,
    base_frequency: f32,
}

struct PlayingVoice {
    sample_data: Vec<f32>,
    position: f32,
    pitch_ratio: f32,
    amplitude: f32,
    decay_rate: f32,
}

impl MedievalSampleBank {
    fn new() -> Self {
        let mut lute_samples = Vec::new();
        let mut bell_samples = Vec::new();
        
        // Generate lute-like samples
        for &freq in &[220.0, 293.66, 369.99, 440.0] {
            lute_samples.push(Self::generate_lute_sample(freq, 2.0));
        }
        
        // Generate bell samples
        for &freq in &[130.81, 164.81, 196.00, 220.00] {
            bell_samples.push(Self::generate_bell_sample(freq, 4.0));
        }
        
        Self {
            lute_samples,
            bell_samples,
            current_voices: Vec::new(),
        }
    }
    
    fn generate_lute_sample(frequency: f32, duration: f32) -> SampleData {
        let sample_rate = 44100.0;
        let length = (sample_rate * duration) as usize;
        let mut data = Vec::with_capacity(length);
        
        for i in 0..length {
            let t = i as f32 / sample_rate;
            let phase = frequency * t * 2.0 * PI;
            
            let attack = (-t * 3.0).exp();
            let sustain = (-t * 0.8).exp();
            
            let fundamental = phase.sin();
            let second = (phase * 2.0).sin() * 0.6;
            let third = (phase * 3.0).sin() * 0.3;
            let fourth = (phase * 4.0).sin() * 0.15;
            
            let sample = (fundamental + second + third + fourth) * attack * sustain * 0.3;
            data.push(sample);
        }
        
        SampleData {
            data,
            base_frequency: frequency,
        }
    }
    
    fn generate_bell_sample(frequency: f32, duration: f32) -> SampleData {
        let sample_rate = 44100.0;
        let length = (sample_rate * duration) as usize;
        let mut data = Vec::with_capacity(length);
        
        let partials = [
            (1.0, 1.0),
            (2.76, 0.6),
            (5.40, 0.25),
            (8.93, 0.12),
            (13.34, 0.06),
        ];
        
        for i in 0..length {
            let t = i as f32 / sample_rate;
            let mut sample = 0.0;
            
            for &(ratio, amplitude) in &partials {
                let partial_freq = frequency * ratio;
                let phase = partial_freq * t * 2.0 * PI;
                let decay = (-t * (0.3 + ratio * 0.1)).exp();
                sample += phase.sin() * amplitude * decay;
            }
            
            let attack = if t < 0.01 { (t / 0.01).powi(2) } else { 1.0 };
            data.push(sample * attack * 0.2);
        }
        
        SampleData {
            data,
            base_frequency: frequency,
        }
    }
    
    fn trigger_lute(&mut self, note: f32, velocity: f32) {
        // Limit maximum concurrent voices to prevent noise
        if self.current_voices.len() >= 6 {
            return; // Don't add more voices if we're at limit
        }
        
        if let Some(sample) = self.find_closest_lute_sample(note) {
            // Add micro-variations for more organic sound
            let pitch_variation = 1.0 + (rand::random::<f32>() - 0.5) * 0.01; // Reduced pitch variation
            let decay_variation = 0.998 + rand::random::<f32>() * 0.002; // Faster decay
            let velocity_variation = velocity * (0.6 + rand::random::<f32>() * 0.3); // Reduced velocity
            
            let voice = PlayingVoice {
                sample_data: sample.data.clone(),
                position: 0.0,
                pitch_ratio: (note / sample.base_frequency) * pitch_variation,
                amplitude: velocity_variation * 0.7, // Reduced amplitude
                decay_rate: decay_variation,
            };
            self.current_voices.push(voice);
        }
    }
    
    fn trigger_bell(&mut self, note: f32, velocity: f32) {
        // Limit maximum concurrent voices to prevent noise
        if self.current_voices.len() >= 6 {
            return; // Don't add more voices if we're at limit
        }
        
        if let Some(sample) = self.find_closest_bell_sample(note) {
            // Bells get more variation for ethereal effect
            let pitch_variation = 1.0 + (rand::random::<f32>() - 0.5) * 0.008; // Reduced pitch variation
            let velocity_variation = velocity * (0.5 + rand::random::<f32>() * 0.4); // Reduced velocity variation
            let decay_variation = 0.9999 + rand::random::<f32>() * 0.0001; // Even faster decay
            
            let voice = PlayingVoice {
                sample_data: sample.data.clone(),
                position: 0.0,
                pitch_ratio: (note / sample.base_frequency) * pitch_variation,
                amplitude: velocity_variation * 0.6, // Reduced amplitude
                decay_rate: decay_variation,
            };
            self.current_voices.push(voice);
        }
    }
    
    fn find_closest_lute_sample(&self, target_freq: f32) -> Option<&SampleData> {
        self.lute_samples.iter()
            .min_by(|a, b| {
                let dist_a = (a.base_frequency - target_freq).abs();
                let dist_b = (b.base_frequency - target_freq).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            })
    }
    
    fn find_closest_bell_sample(&self, target_freq: f32) -> Option<&SampleData> {
        self.bell_samples.iter()
            .min_by(|a, b| {
                let dist_a = (a.base_frequency - target_freq).abs();
                let dist_b = (b.base_frequency - target_freq).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            })
    }
    
    fn process(&mut self) -> f32 {
        let mut output = 0.0;
        let voice_count = self.current_voices.len();
        
        // Dynamic gain scaling based on polyphony to prevent clipping
        let polyphony_gain = if voice_count > 0 {
            1.0 / (1.0 + voice_count as f32 * 0.15) // Gentle scaling
        } else {
            1.0
        };
        
        self.current_voices.retain_mut(|voice| {
            let pos = voice.position as usize;
            if pos >= voice.sample_data.len() {
                return false;
            }
            
            let sample = voice.sample_data[pos];
            output += sample * voice.amplitude * polyphony_gain;
            
            voice.position += voice.pitch_ratio;
            voice.amplitude *= voice.decay_rate;
            
            voice.amplitude > 0.001
        });
        
        // Soft limiting to prevent harsh clipping
        self.soft_limit(output * 0.25) // Reduced base level
    }
    
    fn soft_limit(&self, input: f32) -> f32 {
        let threshold = 0.8;
        if input.abs() > threshold {
            let sign = input.signum();
            let excess = input.abs() - threshold;
            let compressed = threshold + excess * 0.2; // Gentle compression above threshold
            sign * compressed.min(0.95) // Hard limit at 0.95
        } else {
            input
        }
    }
}

/// Simple Neural Modulator - Lightweight neural network for parameter control
struct SimpleNeuralModulator {
    input_weights: [[f32; 8]; 16],
    hidden_weights: [[f32; 16]; 8],
    hidden_bias: [f32; 16],
    output_bias: [f32; 8],
}

impl SimpleNeuralModulator {
    fn new() -> Self {
        let mut modulator = Self {
            input_weights: [[0.0; 8]; 16],
            hidden_weights: [[0.0; 16]; 8],
            hidden_bias: [0.0; 16],
            output_bias: [0.0; 8],
        };
        
        modulator.init_dungeon_synth_weights();
        modulator
    }
    
    fn init_dungeon_synth_weights(&mut self) {
        // Initialize with dungeon synth-optimized weights
        for i in 0..16 {
            for j in 0..8 {
                match i % 4 {
                    0 => self.input_weights[i][j] = if j < 2 { 0.8 } else { 0.2 },
                    1 => self.input_weights[i][j] = if j == 2 || j == 6 { 0.9 } else { 0.1 },
                    2 => self.input_weights[i][j] = if j > 3 { 0.7 } else { 0.3 },
                    3 => self.input_weights[i][j] = 0.5 + (i as f32 - 8.0) * 0.1,
                    _ => unreachable!(),
                }
            }
            self.hidden_bias[i] = -0.5 + (i as f32 / 16.0);
        }
        
        for i in 0..8 {
            for j in 0..16 {
                match i {
                    0 => self.hidden_weights[i][j] = if j < 4 { 0.6 } else { 0.2 },
                    1 => self.hidden_weights[i][j] = if j >= 4 && j < 8 { 0.7 } else { 0.1 },
                    2 => self.hidden_weights[i][j] = if j >= 8 && j < 12 { 0.8 } else { 0.2 },
                    3 => self.hidden_weights[i][j] = if j >= 12 { 0.9 } else { 0.3 },
                    4..=7 => self.hidden_weights[i][j] = 0.4 + ((i + j) % 3) as f32 * 0.2,
                    _ => unreachable!(),
                }
            }
            self.output_bias[i] = 0.0;
        }
    }
    
    fn forward(&self, inputs: &[f32; 8]) -> [f32; 8] {
        let mut hidden = [0.0; 16];
        
        for i in 0..16 {
            let mut sum = self.hidden_bias[i];
            for j in 0..8 {
                sum += inputs[j] * self.input_weights[i][j];
            }
            hidden[i] = Self::tanh_activation(sum);
        }
        
        let mut outputs = [0.0; 8];
        for i in 0..8 {
            let mut sum = self.output_bias[i];
            for j in 0..16 {
                sum += hidden[j] * self.hidden_weights[i][j];
            }
            outputs[i] = Self::tanh_activation(sum);
        }
        
        outputs
    }
    
    fn tanh_activation(x: f32) -> f32 {
        let x2 = x * x;
        x * (27.0 + x2) / (27.0 + 9.0 * x2)
    }
    
    fn get_modulation_values(&self, game_features: &[f32; 8]) -> [f32; 8] {
        let mut normalized_inputs = [0.0; 8];
        for i in 0..8 {
            normalized_inputs[i] = (game_features[i] * 2.0 - 1.0).clamp(-1.0, 1.0);
        }
        
        let outputs = self.forward(&normalized_inputs);
        
        let mut modulation = [0.0; 8];
        for i in 0..8 {
            modulation[i] = (outputs[i] + 1.0) / 2.0;
        }
        
        modulation
    }
}

/// Crypt Reverb with Hadamard feedback matrix
#[derive(Debug)]
struct CryptReverb {
    delay_lines: [DelayLine; 8],
    feedback_matrix: [[f32; 8]; 8],
    wet_amount: f32,
}

#[derive(Debug)]
struct DelayLine {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
}

impl CryptReverb {
    fn new(sample_rate: f32) -> Self {
        let delay_times = [0.023, 0.031, 0.041, 0.053, 0.067, 0.079, 0.097, 0.113];
        let delay_lines: Vec<DelayLine> = delay_times.iter().map(|&time| {
            let size = (sample_rate * time) as usize;
            DelayLine {
                buffer: vec![0.0; size],
                write_pos: 0,
                delay_samples: size,
            }
        }).collect();
        
        let delay_lines: [DelayLine; 8] = delay_lines.try_into().unwrap();
        
        let feedback_matrix = [
            [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
            [0.5, -0.5, 0.5, -0.5, 0.5, -0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5, -0.5, 0.5, 0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5, 0.5, 0.5, -0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5, 0.5, -0.5, -0.5, -0.5, -0.5],
            [0.5, -0.5, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5],
            [0.5, 0.5, -0.5, -0.5, -0.5, -0.5, 0.5, 0.5],
            [0.5, -0.5, -0.5, 0.5, -0.5, 0.5, 0.5, -0.5],
        ];
        
        Self {
            delay_lines,
            feedback_matrix,
            wet_amount: 0.3,
        }
    }
    
    fn process(&mut self, input: f32) -> f32 {
        let mut outputs = [0.0; 8];
        
        for (i, delay_line) in self.delay_lines.iter().enumerate() {
            let read_pos = (delay_line.write_pos + delay_line.delay_samples - delay_line.delay_samples) % delay_line.delay_samples;
            outputs[i] = delay_line.buffer[read_pos];
        }
        
        let mut inputs = [input; 8];
        for i in 0..8 {
            let mut sum = 0.0;
            for j in 0..8 {
                sum += outputs[j] * self.feedback_matrix[i][j] * 0.7;
            }
            inputs[i] += sum;
        }
        
        for (i, delay_line) in self.delay_lines.iter_mut().enumerate() {
            delay_line.buffer[delay_line.write_pos] = inputs[i];
            delay_line.write_pos = (delay_line.write_pos + 1) % delay_line.delay_samples;
        }
        
        let reverb_output = outputs.iter().sum::<f32>() / 8.0;
        input * (1.0 - self.wet_amount) + reverb_output * self.wet_amount
    }
}

/// Tape Saturation for vintage character
struct TapeSaturation {
    drive: f32,
    output_gain: f32,
}

impl TapeSaturation {
    fn new() -> Self {
        Self {
            drive: 1.5,
            output_gain: 0.7,
        }
    }
    
    fn process(&self, input: f32) -> f32 {
        let driven = input * self.drive;
        let saturated = driven.tanh();
        saturated * self.output_gain
    }
}

impl HybridDungeonSynthEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            cathedral_drone: CathedralDroneLayer::new(sample_rate),
            medieval_samples: MedievalSampleBank::new(),
            neural_modulator: SimpleNeuralModulator::new(),
            crypt_reverb: CryptReverb::new(sample_rate),
            tape_saturation: TapeSaturation::new(),
            
            sample_rate,
            current_features: [0.0; 8],
            update_counter: 0,
            
            // Initialize evolution system
            pattern_memory: [0.0; 16],
            evolution_phase: 0.0,
            micro_variation_timer: 0.0,
            harmonic_drift: [1.0, 1.0, 1.0, 1.0],
            scale_evolution: 0.0,
            temporal_complexity: 0.0,
            
            // Initialize cell matrix
            cell_matrix: CellMatrix::new(),
            
            // Initialize enhanced modulation system
            _bass_pattern_memory: [0.0; 8],
            _rhythm_phase: 0.0,
            _spatial_modulation: [0.0; 16],
            
            synthesis_mix: 0.7,
            _neural_influence: 0.5,
            _medieval_authenticity: 0.8,
            
            // Initialize audio optimization
            master_gain: 1.0,
            peak_detector: 0.0,
            compression_ratio: 1.0,
            // Phase-2 init
            scale_notes: [220.0, 246.94, 261.63, 293.66, 329.63, 369.99, 415.30], // A minor by default
            last_milestone_generation: 0,
        }
    }
    
    pub fn process_sample(&mut self, game_features: [f32; 8]) -> (f32, f32) {
        self.current_features = game_features;
        
        // Continuous evolution system - always evolving!
        self.update_evolution_state();
        
        if self.update_counter % 64 == 0 {
            self.update_all_parameters();
        }
        self.update_counter += 1;
        
        // Generate synthesis layers
        let drone = self.cathedral_drone.process(self.sample_rate);
        
        // Generate sample-based layers
        let samples = self.medieval_samples.process();
        
        // Mix synthesis and samples
        let mixed = drone * self.synthesis_mix + samples * (1.0 - self.synthesis_mix);
        
        // Apply effects
        let reverbed = self.crypt_reverb.process(mixed);
        let saturated = self.tape_saturation.process(reverbed);
        
        // Master limiting to prevent clipping
        let limited = self.master_limiter(saturated);
        
        // Create stereo image with subtle differences
        let left = limited;
        let right = limited * 0.96; // Slight channel difference for width
        
        (left * 0.8, right * 0.8) // Increased base mix level for more presence
    }
    
    fn update_all_parameters(&mut self) {
        let neural_mod = self.neural_modulator.get_modulation_values(&self.current_features);
        
        let population = self.current_features[0];
        let density = self.current_features[1];
        let activity = self.current_features[2];
        let cluster_count = self.current_features[3];
        let cluster_avg_size = self.current_features[4];
        let symmetry = self.current_features[5];
        let chaos = self.current_features[6];
        let generation = self.current_features[7];
        
        // -----------------------------
        // 1. Compute musical scale (root & mode)
        // -----------------------------
        let scale_choice = ((symmetry + chaos) * 4.0) as usize % 4; // 0-3

        // Root note shifts with symmetry (smooth) – between ~55 Hz and 110 Hz
        let root_hz = 55.0 * 2f32.powf((symmetry - 0.5) * 1.0);

        let new_scale = Self::build_scale(root_hz, scale_choice);
        if new_scale != self.scale_notes {
            println!("🎼 New scale root {:.1} Hz, mode {}", root_hz, scale_choice);
        }
        self.scale_notes = new_scale;

        // Update drone layer base frequencies (sub-octaves of scale degrees 0,2,4)
        let drone_bases = [
            self.scale_notes[0] * 0.25,
            self.scale_notes[0] * 0.5,
            self.scale_notes[2] * 0.5,
            self.scale_notes[4] * 0.5,
        ];
        self.cathedral_drone.set_base_frequencies(drone_bases);

        // Milestone bell every 100 generations
        let gen_u64 = generation as u64;
        if gen_u64 / 100 > self.last_milestone_generation / 100 {
            self.medieval_samples.trigger_bell(self.scale_notes[0], 1.0);
            self.last_milestone_generation = gen_u64;
        }

        // -----------------------------
        // 2. Existing parameter updates
        // -----------------------------
        // More dynamic cathedral drone modulation
        let drone_intensity = population * (1.0 + neural_mod[0] * 0.5);
        let drone_darkness = (1.0 - symmetry) * (1.0 + chaos * 0.3);
        
        // Pass cell regions for enhanced modulation
        self.cathedral_drone.update_parameters(drone_intensity, drone_darkness, neural_mod[0], &self.cell_matrix.regions);
        
        // More controlled musical variety and responsive triggering
        if activity > 0.05 { // Increased threshold to reduce noise during low activity
            let activity_intensity = (activity * 8.0).min(1.0); // Reduced scaling to prevent overwhelming
            
            // Use freshly-computed scale for melodic content
            let base_frequencies = self.scale_notes;
            
            // Melodic patterns based on activity level - much more controlled
            if activity > 0.15 && self.update_counter % 128 == 0 { // Higher threshold and less frequent triggering
                let note_index = (neural_mod[1] * 7.0) as usize % 7;
                let base_note = base_frequencies[note_index] * (1.0 + neural_mod[1] * 0.1);
                
                // Single note instead of full chord to reduce noise
                self.medieval_samples.trigger_lute(base_note, activity_intensity * 0.6);
                
                // Only add harmony occasionally
                if population > 0.7 && self.update_counter % 256 == 0 {
                    self.medieval_samples.trigger_lute(base_note * 1.25, activity_intensity * 0.4); // Third
                }
            }
            
            // Occasional melodic phrases during very high activity
            if activity > 0.25 && self.update_counter % 512 == 0 { // Much higher threshold and very infrequent
                let note_idx = (neural_mod[2] * 7.0) as usize % 7;
                let run_note = base_frequencies[note_idx] * (1.0 + neural_mod[2] * 0.1);
                self.medieval_samples.trigger_lute(run_note, activity_intensity * 0.5);
            }
            
            // Controlled dissonance during chaos - much more restrained
            if chaos > 0.25 && self.update_counter % 192 == 0 { // Higher threshold and less frequent
                let base_note = base_frequencies[((neural_mod[6] * 7.0) as usize) % 7];
                let chromatic_shift = if neural_mod[6] > 0.5 { 1.059 } else { 0.944 }; // Semitone up/down
                let dissonant_note = base_note * chromatic_shift;
                self.medieval_samples.trigger_lute(dissonant_note, activity_intensity * 0.4);
            }
            
            // Deep bass drones for dense populations - more restrained
            if density > 0.6 && self.update_counter % 256 == 0 { // Higher threshold and less frequent
                let bass_note = base_frequencies[0] * 0.5; // Bass octave
                self.medieval_samples.trigger_lute(bass_note, activity_intensity * 0.7);
            }
            
            // High register sparkles for complex patterns
            if cluster_count > 0.8 && symmetry > 0.3 {
                let high_note = base_frequencies[6] * 2.0; // High octave
                self.medieval_samples.trigger_lute(high_note, activity_intensity * 0.6);
            }
        }
        
        // Much more controlled bell system
        if generation % 60.0 < 1.0 && neural_mod[2] > 0.4 { // Much less frequent bells
            let bell_scale = [130.81, 146.83, 164.81, 174.61, 196.0, 220.0, 246.94]; // C3 to B3
            let bell_index = (neural_mod[3] * 7.0) as usize % 7;
            let bell_note = bell_scale[bell_index] * (1.0 + symmetry * 0.2);
            self.medieval_samples.trigger_bell(bell_note, 0.4 + activity * 0.3);
        }
        
        // Activity-based bell cascades - much more restrained
        if activity > 0.2 && symmetry > 0.3 && self.update_counter % 128 == 0 {
            let high_bells = [220.0, 246.94, 277.18, 311.13]; // High register bells
            let bell_idx = (activity * 4.0) as usize % 4;
            let bell_note = high_bells[bell_idx] * (1.0 + population * 0.2);
            self.medieval_samples.trigger_bell(bell_note, activity * 0.5);
        }
        
        // Population density creates deep resonant bell drones - much more selective
        if density > 0.4 && cluster_avg_size > 0.5 && self.update_counter % 256 == 0 {
            let deep_bells = [65.41, 73.42, 82.41, 98.0]; // Very deep bells
            let deep_idx = (density * 4.0) as usize % 4;
            let deep_bell = deep_bells[deep_idx] * (1.0 + neural_mod[3] * 0.3);
            self.medieval_samples.trigger_bell(deep_bell, density * 8.0);
        }
        
        // Chaos creates bell clusters and dissonance - much more restrained
        if chaos > 0.4 && neural_mod[4] > 0.6 && self.update_counter % 192 == 0 {
            let chaos_bells = [138.59, 155.56, 185.0, 207.65]; // Slightly detuned bells
            let chaos_idx = (chaos * 4.0) as usize % 4;
            let chaos_bell = chaos_bells[chaos_idx] * (0.99 + chaos * 0.02); // Less detuning
            self.medieval_samples.trigger_bell(chaos_bell, chaos * 0.6);
        }
        
        // Symmetrical patterns trigger bell arpeggios - much less frequent
        if symmetry > 0.7 && generation % 80.0 < 1.0 {
            let arp_bells = [164.81, 196.0, 246.94]; // Shorter arpeggio
            for (i, &bell_freq) in arp_bells.iter().enumerate() {
                let delay_factor = i as f32 * 0.05;
                let bell_note = bell_freq * (1.0 + neural_mod[5] * 0.1);
                self.medieval_samples.trigger_bell(bell_note, (symmetry + delay_factor) * 0.4);
            }
        }
        
        // Dynamic synthesis mix based on multiple factors
        let activity_factor = (activity * 5.0).min(1.0);
        let chaos_factor = chaos * 0.6;
        let neural_factor = neural_mod[7] * 0.4;
        self.synthesis_mix = 0.3 + activity_factor * 0.4 + chaos_factor + neural_factor;
        
        // Modulate reverb based on space and activity
        self.crypt_reverb.wet_amount = 0.2 + (1.0 - density) * 0.3 + activity * 0.2;
        
        // Dynamic tape saturation based on intensity
        let intensity = population + activity * 2.0;
        self.tape_saturation.drive = 1.2 + intensity * 0.8;
        self.tape_saturation.output_gain = 0.6 + neural_mod[4] * 0.2;
    }
    
    fn master_limiter(&mut self, input: f32) -> f32 {
        // Adaptive gain control to prevent clipping
        let input_level = input.abs();
        
        // Update peak detector with slight decay
        self.peak_detector = self.peak_detector * 0.9995 + input_level * 0.0005;
        
        // Adaptive compression based on peak levels
        if self.peak_detector > 0.8 {
            self.compression_ratio = (self.compression_ratio * 0.999 + 0.3 * 0.001).min(0.8);
            self.master_gain = (self.master_gain * 0.9999 + 0.7 * 0.0001).max(0.4);
        } else if self.peak_detector < 0.3 {
            self.compression_ratio = (self.compression_ratio * 0.999 + 1.0 * 0.001).max(0.3);
            self.master_gain = (self.master_gain * 0.9999 + 1.0 * 0.0001).min(1.2);
        }
        
        // Apply adaptive gain
        let gained_input = input * self.master_gain;
        
        // Multi-stage soft limiting
        let threshold1 = 0.7;
        let threshold2 = 0.85;
        
        let mut output = gained_input;
        
        // First stage - gentle compression
        if output.abs() > threshold1 {
            let sign = output.signum();
            let excess = output.abs() - threshold1;
            let compressed1 = threshold1 + excess * self.compression_ratio;
            output = sign * compressed1;
        }
        
        // Second stage - harder limiting
        if output.abs() > threshold2 {
            let sign = output.signum();
            let excess = output.abs() - threshold2;
            let compressed2 = threshold2 + excess * 0.1; // Hard limiting
            output = sign * compressed2.min(0.95); // Absolute ceiling
        }
        
        output
    }
    
    fn update_evolution_state(&mut self) {
        let sample_rate = self.sample_rate;
        
        // Update evolution phase - very slow cycle (about 30 seconds at 44.1kHz)
        self.evolution_phase += 1.0 / (sample_rate * 30.0);
        if self.evolution_phase >= 1.0 {
            self.evolution_phase -= 1.0;
        }
        
        // Update micro-variation timer - faster cycles for subtle changes
        self.micro_variation_timer += 1.0 / (sample_rate * 2.0); // 2-second cycles
        if self.micro_variation_timer >= 1.0 {
            self.micro_variation_timer -= 1.0;
        }
        
        // Update pattern memory - shift and add current complexity
        let current_complexity = self.calculate_pattern_complexity();
        for i in (1..16).rev() {
            self.pattern_memory[i] = self.pattern_memory[i - 1];
        }
        self.pattern_memory[0] = current_complexity;
        
        // Update temporal complexity - average of recent patterns
        self.temporal_complexity = self.pattern_memory.iter().sum::<f32>() / 16.0;
        
        // Update harmonic drift - slowly evolving harmonic content
        for i in 0..4 {
            let drift_speed = 1.0 / (sample_rate * (10.0 + i as f32 * 5.0)); // Different speeds per harmonic
            let drift_amount = (self.evolution_phase * std::f32::consts::PI * 2.0 * (i + 1) as f32).sin() * 0.1;
            self.harmonic_drift[i] += drift_speed * drift_amount;
            self.harmonic_drift[i] = self.harmonic_drift[i].max(0.3).min(2.0); // Keep in reasonable range
        }
        
        // Update scale evolution - gradual shifts between musical contexts
        let scale_drift_speed = 1.0 / (sample_rate * 20.0); // 20-second evolution cycle
        let temporal_influence = self.temporal_complexity * 0.5;
        self.scale_evolution += scale_drift_speed * (1.0 + temporal_influence);
        if self.scale_evolution >= 4.0 {
            self.scale_evolution -= 4.0;
        }
        
        // Apply continuous variations to synthesis parameters
        self.apply_evolutionary_changes();
    }
    
    fn calculate_pattern_complexity(&self) -> f32 {
        let features = &self.current_features;
        let activity = features[2];
        let symmetry = features[5];
        let chaos = features[6];
        let cluster_count = features[3];
        
        // Combine factors to get overall pattern complexity
        let base_complexity = activity * 0.3 + chaos * 0.4 + symmetry * 0.2 + cluster_count * 0.1;
        
        // Add some temporal variation based on evolution phase
        let temporal_mod = (self.evolution_phase * std::f32::consts::PI * 4.0).sin() * 0.05;
        
        (base_complexity + temporal_mod).clamp(0.0, 1.0)
    }
    
    fn apply_evolutionary_changes(&mut self) {
        // Continuously evolving cathedral drone parameters
        let phase_sin = (self.evolution_phase * std::f32::consts::PI * 2.0).sin();
        let micro_sin = (self.micro_variation_timer * std::f32::consts::PI * 8.0).sin();
        
        // Evolve drone oscillator frequencies slightly
        for (i, osc) in self.cathedral_drone.oscillators.iter_mut().enumerate() {
            let base_freq = match i {
                0 => 65.41,  // C2
                1 => 82.41,  // E2
                _ => 98.0,   // G2
            };
            
            // Apply harmonic drift and micro-variations
            let drift_factor = self.harmonic_drift[i % 4];
            let micro_variation = 1.0 + micro_sin * 0.002 * (i + 1) as f32; // Very subtle pitch drift
            let temporal_variation = 1.0 + phase_sin * 0.008; // Slow breathing effect
            
            osc.frequency = base_freq * drift_factor * micro_variation * temporal_variation;
            
            // Evolve harmonic content over time
            for h in 0..8 {
                let harmonic_phase = self.evolution_phase * (h + 1) as f32 * 0.7;
                let harmonic_variation = (harmonic_phase * std::f32::consts::PI * 2.0).sin() * 0.1;
                osc.harmonic_weights[h] *= 1.0 + harmonic_variation;
                osc.harmonic_weights[h] = osc.harmonic_weights[h].clamp(0.1, 1.0);
            }
        }
        
        // Evolve reverb parameters for spatial variation
        let reverb_evolution = (self.evolution_phase * std::f32::consts::PI * 3.0).cos();
        self.crypt_reverb.wet_amount += reverb_evolution * 0.05;
        self.crypt_reverb.wet_amount = self.crypt_reverb.wet_amount.clamp(0.1, 0.8);
        
        // Evolve sample triggering probability based on temporal complexity
        if self.temporal_complexity > 0.3 && self.micro_variation_timer < 0.1 {
            // Trigger ambient variations during low activity periods
            self.trigger_evolutionary_samples();
        }
        
        // Continuously adjust synthesis mix based on evolution phase
        let base_mix = 0.3 + self.temporal_complexity * 0.4;
        let evolution_mod = phase_sin * 0.15;
        self.synthesis_mix = (base_mix + evolution_mod).clamp(0.1, 0.9);
    }
    
    fn trigger_evolutionary_samples(&mut self) {
        // Use evolved scale position for continuous musical variety
        let evolved_scale_pos = (self.scale_evolution + self.temporal_complexity) % 4.0;
        let scale_index = evolved_scale_pos as usize % 4;
        
        let base_frequencies = match scale_index {
            0 => [220.0, 246.94, 277.18, 293.66, 329.63, 369.99, 415.30], // A Minor
            1 => [261.63, 293.66, 329.63, 349.23, 392.00, 440.0, 493.88], // C Major
            2 => [220.0, 233.08, 261.63, 277.18, 311.13, 349.23, 369.99], // A Dorian
            _ => [196.0, 220.0, 246.94, 261.63, 293.66, 329.63, 349.23],  // G Mixolydian
        };
        
        // Subtle evolving harmonies during stable patterns
        let note_index = (self.evolution_phase * 7.0) as usize % 7;
        let base_note = base_frequencies[note_index];
        
        // Apply harmonic drift to note selection
        let harmonic_modifier = self.harmonic_drift[note_index % 4];
        let evolved_note = base_note * harmonic_modifier;
        
        // Gentle volume based on temporal complexity
        let ambient_volume = 0.2 + self.temporal_complexity * 0.3;
        
        // Randomly choose between lute and bell for variety
        let micro_choice = (self.micro_variation_timer * 100.0) as i32 % 3;
        match micro_choice {
            0 => self.medieval_samples.trigger_lute(evolved_note, ambient_volume),
            1 => self.medieval_samples.trigger_bell(evolved_note * 0.5, ambient_volume), // Lower bell
            _ => {
                // Harmonic chord for richer evolution
                self.medieval_samples.trigger_lute(evolved_note, ambient_volume * 0.7);
                self.medieval_samples.trigger_lute(evolved_note * 1.25, ambient_volume * 0.5); // Third
            }
        }
    }

    /// Build a 7-note diatonic scale (Ionian, Aeolian, Dorian, Mixolydian) starting from `root_hz`.
    fn build_scale(root_hz: f32, mode: usize) -> [f32; 7] {
        // Semitone intervals for major scale
        const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
        // Rotate intervals for modes
        let rotated: Vec<i32> = MAJOR.iter()
            .cycle()
            .skip(mode) // mode shift
            .take(7)
            .map(|v| *v - MAJOR[mode]) // normalise to root = 0
            .collect();

        let mut notes = [0f32; 7];
        for (i, semis) in rotated.iter().enumerate() {
            notes[i] = root_hz * 2f32.powf(*semis as f32 / 12.0);
        }
        notes
    }
}

// Global instance and volume control
static HYBRID_ENGINE: Mutex<Option<HybridDungeonSynthEngine>> = Mutex::new(None);
static MASTER_VOLUME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// Audio source for rodio
struct HybridAudioSource {
    sample_rate: u32,
    channels: u16,
    sample_counter: usize,
}

impl HybridAudioSource {
    fn new() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            sample_counter: 0,
        }
    }
}

impl Iterator for HybridAudioSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Use the actual global engine that receives real game features
        if let Ok(mut engine_guard) = HYBRID_ENGINE.try_lock() {
            if let Some(ref mut engine) = engine_guard.as_mut() {
                let game_features = engine.current_features;
                let (left, right) = engine.process_sample(game_features);
                
                // Alternate between left and right channels
                let output = if self.sample_counter % 2 == 0 {
                    left
                } else {
                    right
                };
                
                // Apply master volume control
                let master_volume = get_hybrid_volume();
                
                self.sample_counter += 1;
                return Some(output * 0.6 * master_volume); // Doubled base volume for more presence
            }
        }
        
        // Return silence if engine is locked or unavailable
        Some(0.0)
    }
}

impl Source for HybridAudioSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None // Infinite stream
    }
}

pub fn init_hybrid_dungeon_synth() {
    let engine = HybridDungeonSynthEngine::new(44100.0);
    *HYBRID_ENGINE.lock().unwrap() = Some(engine);
    
    // Initialize volume from config
    let default_volume = 0.7; // Default volume if config not available
    set_hybrid_volume(default_volume);
    
    // Initialize audio output with better error handling and persistence
    std::thread::spawn(|| {
        println!("🎵 Starting hybrid audio thread...");
        
        match OutputStream::try_default() {
            Ok((_stream, stream_handle)) => {
                println!("✅ Audio output stream created successfully");
                
                match Sink::try_new(&stream_handle) {
                    Ok(sink) => {
                        println!("✅ Audio sink created successfully");
                        
                        let audio_source = HybridAudioSource::new();
                        sink.append(audio_source);
                        sink.set_volume(0.7);
                        
                        println!("🏰🔊 Hybrid Dungeon Synth Engine initialized with AUDIO OUTPUT!");
                        println!("🎵 Audio thread running - you should now hear medieval dungeon synth audio!");
                        
                        // Keep the thread alive indefinitely
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to create audio sink: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("❌ Failed to initialize audio output: {}", e);
                println!("   This might be because:");
                println!("   - No audio device available");
                println!("   - Audio permissions not granted");
                println!("   - Another application is using the audio device");
            }
        }
    });
    
    println!("🏰 Hybrid Dungeon Synth Engine initialized!");
}

pub fn update_hybrid_dungeon_synth(features: [f32; 8]) {
    // Update the engine with real game state features
    if let Some(ref mut engine) = HYBRID_ENGINE.lock().unwrap().as_mut() {
        engine.current_features = features;
    }
}

/// Update the cell matrix for optimized processing of large populations
pub fn update_hybrid_cell_data(cells: &[(i32, i32)], camera_x: f32, camera_y: f32, viewport_size: f32) {
    if let Ok(mut engine_guard) = HYBRID_ENGINE.try_lock() {
        if let Some(ref mut engine) = engine_guard.as_mut() {
            engine.cell_matrix.update_from_cells(cells, camera_x, camera_y, viewport_size);
        }
    }
}

/// Set the master volume for the hybrid audio engine (0.0 to 2.0 for overdrive)
pub fn set_hybrid_volume(volume: f32) {
    let volume_clamped = volume.clamp(0.0, 2.0); // Allow up to 200% for overdrive
    let volume_bits = volume_clamped.to_bits();
    MASTER_VOLUME.store(volume_bits, std::sync::atomic::Ordering::Relaxed);
    if volume_clamped > 1.0 {
        println!("🔊🔥 OVERDRIVE! Hybrid audio volume: {:.0}%", volume_clamped * 100.0);
    } else {
        println!("🔊 Hybrid audio volume set to: {:.0}%", volume_clamped * 100.0);
    }
}

/// Get the current master volume
pub fn get_hybrid_volume() -> f32 {
    let volume_bits = MASTER_VOLUME.load(std::sync::atomic::Ordering::Relaxed);
    f32::from_bits(volume_bits)
}

/// Returns the current scale root frequency (degree 0) if the engine is active.
pub fn get_scale_root() -> Option<f32> {
    if let Ok(engine_guard) = HYBRID_ENGINE.try_lock() {
        if let Some(ref engine) = *engine_guard {
            return Some(engine.scale_notes[0]);
        }
    }
    None
}

pub fn generate_hybrid_samples(buffer: &mut [(f32, f32)]) {
    if let Some(ref mut engine) = HYBRID_ENGINE.lock().unwrap().as_mut() {
        for sample in buffer.iter_mut() {
            let game_features = engine.current_features;
            *sample = engine.process_sample(game_features);
        }
    }
}

/// Set the synthesis/sample mix (0.0 = all samples, 1.0 = all synth).
pub fn set_hybrid_synthesis_mix(mix: f32) {
    let mix_clamped = mix.clamp(0.0, 1.0);
    if let Ok(mut engine_guard) = HYBRID_ENGINE.try_lock() {
        if let Some(ref mut engine) = *engine_guard {
            engine.synthesis_mix = mix_clamped;
            println!("🎛️ Hybrid synthesis mix set to: {:.0}% synth / {:.0}% samples", mix_clamped * 100.0, (1.0 - mix_clamped) * 100.0);
        }
    }
} 