//! Musical scales for ambient Game of Life audio
//! Maps cellular automaton rules to beautiful modal scales

use std::f32::consts::PI;

/// Musical scales for different CA rules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    /// Conway's Game of Life -> Dorian mode (mysterious, ambient)
    Dorian,
    /// HighLife -> Mixolydian mode (bright, alive)
    Mixolydian,
    /// Seeds -> Phrygian mode (dark, dramatic)
    Phrygian,
    /// Immigration -> Pentatonic (peaceful, flowing)
    Pentatonic,
    /// QuadLife -> Lydian (ethereal, otherworldly)
    Lydian,
    /// Brian's Brain -> Locrian (unstable, chaotic but musical)
    Locrian,
}

impl Default for Scale {
    fn default() -> Self {
        Scale::Dorian
    }
}

impl Scale {
    /// Get the base frequency for this scale (in Hz)
    /// Uses low frequencies for ambient drone (50-200 Hz range)
    pub fn get_base_frequency(self) -> f32 {
        match self {
            Scale::Dorian => 65.4,      // C2 
            Scale::Mixolydian => 73.4,  // D2
            Scale::Phrygian => 82.4,    // E2
            Scale::Pentatonic => 98.0,  // G2
            Scale::Lydian => 110.0,     // A2
            Scale::Locrian => 123.5,    // B2
        }
    }
    
    /// Get the scale intervals (as frequency multipliers)
    pub fn get_intervals(self) -> Vec<f32> {
        match self {
            Scale::Dorian => vec![
                1.0,      // Root
                1.125,    // Minor 2nd
                1.25,     // Minor 3rd  
                1.5,      // Perfect 4th
                1.667,    // Perfect 5th
                1.875,    // Minor 6th
                2.0,      // Minor 7th
            ],
            Scale::Mixolydian => vec![
                1.0,      // Root
                1.125,    // Major 2nd
                1.25,     // Major 3rd
                1.5,      // Perfect 4th
                1.667,    // Perfect 5th
                1.875,    // Major 6th
                1.9,      // Minor 7th
            ],
            Scale::Phrygian => vec![
                1.0,      // Root
                1.067,    // Minor 2nd
                1.25,     // Minor 3rd
                1.5,      // Perfect 4th
                1.667,    // Perfect 5th
                1.778,    // Minor 6th
                2.0,      // Minor 7th
            ],
            Scale::Pentatonic => vec![
                1.0,      // Root
                1.125,    // Major 2nd
                1.25,     // Major 3rd
                1.667,    // Perfect 5th
                1.875,    // Major 6th
            ],
            Scale::Lydian => vec![
                1.0,      // Root
                1.125,    // Major 2nd
                1.25,     // Major 3rd
                1.414,    // Augmented 4th (tritone - ethereal)
                1.667,    // Perfect 5th
                1.875,    // Major 6th
                2.0,      // Major 7th
            ],
            Scale::Locrian => vec![
                1.0,      // Root
                1.067,    // Minor 2nd
                1.25,     // Minor 3rd
                1.414,    // Diminished 4th
                1.587,    // Diminished 5th
                1.778,    // Minor 6th
                2.0,      // Minor 7th
            ],
        }
    }
    
    /// Get a frequency for a specific grid position using this scale
    /// Maps X coordinate to scale degrees, Y coordinate to octaves
    pub fn get_frequency_for_position(self, x: i32, y: i32, grid_size: (i32, i32)) -> f32 {
        let base_freq = self.get_base_frequency();
        let intervals = self.get_intervals();
        
        // Map X position to scale degree
        let scale_degree = ((x + grid_size.0 / 2) as usize) % intervals.len();
        let interval_multiplier = intervals[scale_degree];
        
        // Map Y position to octave (higher Y = higher octave)
        let octave_shift = (y as f32 / grid_size.1 as f32) * 3.0; // Up to 3 octaves
        let octave_multiplier = 2.0_f32.powf(octave_shift);
        
        base_freq * interval_multiplier * octave_multiplier
    }
    
    /// Get amplitude based on Y position (bottom = louder)
    pub fn get_amplitude_for_position(self, y: i32, grid_size: (i32, i32)) -> f32 {
        // Bottom of grid is louder (higher amplitude)
        let normalized_y = (y + grid_size.1 / 2) as f32 / grid_size.1 as f32;
        (1.0 - normalized_y).clamp(0.1, 0.8) // Between 0.1 and 0.8 amplitude
    }
    
    /// Generate a smooth envelope for grain synthesis
    pub fn generate_envelope(self, length: usize, envelope_type: EnvelopeType) -> Vec<f32> {
        let mut envelope = Vec::with_capacity(length);
        
        for i in 0..length {
            let t = i as f32 / length as f32;
            let amplitude = match envelope_type {
                EnvelopeType::Gaussian => {
                    // Gaussian bell curve
                    let sigma = 0.3;
                    (-((t - 0.5) / sigma).powi(2) / 2.0).exp()
                },
                EnvelopeType::Hann => {
                    // Hann window (smooth)
                    0.5 * (1.0 - (2.0 * PI * t).cos())
                },
                EnvelopeType::ExpDecay => {
                    // Exponential decay (percussive)
                    (-t * 3.0).exp()
                },
            };
            envelope.push(amplitude);
        }
        
        envelope
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EnvelopeType {
    Gaussian,
    Hann,
    ExpDecay,
}

/// Generate a sine wave with the given frequency and envelope
pub fn generate_grain(frequency: f32, sample_rate: f32, envelope: &[f32]) -> Vec<f32> {
    let mut grain = Vec::with_capacity(envelope.len());
    
    for (i, &env_amp) in envelope.iter().enumerate() {
        let t = i as f32 / sample_rate;
        let phase = 2.0 * PI * frequency * t;
        let sample = phase.sin() * env_amp;
        grain.push(sample);
    }
    
    grain
}

/// Create ambient chord based on cell cluster
pub fn generate_chord_for_cluster(scale: Scale, cluster_size: usize, base_freq: f32) -> Vec<f32> {
    let intervals = scale.get_intervals();
    let mut chord_frequencies = Vec::new();
    
    // Select chord tones based on cluster size
    let chord_size = (cluster_size % 4) + 2; // 2-5 note chords
    
    for i in 0..chord_size {
        let interval_index = (i * 2) % intervals.len(); // Use every other interval
        let frequency = base_freq * intervals[interval_index];
        chord_frequencies.push(frequency);
    }
    
    chord_frequencies
} 