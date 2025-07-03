

/// Spatial audio mapping system for infinite grid coordinates
/// Transforms infinite (x, y) coordinates into musical parameters
/// using non-linear, interdependent functions
pub struct SpatialMapper {
    /// Reference point for coordinate normalization
    pub origin: (f32, f32),
    /// Scale factors for coordinate transformation
    pub scale_x: f32,
    pub scale_y: f32,
    /// Musical scale parameters
    pub base_frequency: f32,
    pub frequency_range: f32,
}

impl SpatialMapper {
    pub fn new() -> Self {
        Self {
            origin: (0.0, 0.0),
            scale_x: 0.1,
            scale_y: 0.1,
            base_frequency: 220.0, // A3 note as base
            frequency_range: 3000.0, // Up to ~3.2kHz
        }
    }
    
    /// Update origin based on camera/view center for relative mapping
    pub fn update_origin(&mut self, camera_x: f32, camera_y: f32) {
        self.origin = (camera_x, camera_y);
    }
    
    /// Transform infinite coordinates to musical frequency
    /// Uses spiral mapping with harmonic relationships
    pub fn coord_to_frequency(&self, x: i32, y: i32) -> f32 {
        // Normalize coordinates relative to current view
        let norm_x = (x as f32 - self.origin.0) * self.scale_x;
        let norm_y = (y as f32 - self.origin.1) * self.scale_y;
        
        // Create interdependent spiral mapping
        // Distance from origin affects base pitch
        let distance = (norm_x * norm_x + norm_y * norm_y).sqrt();
        let angle = norm_y.atan2(norm_x);
        
        // Spiral frequency mapping with harmonic series
        let spiral_factor = (distance * 0.5).sin() * 0.5 + 0.5; // 0.0 to 1.0
        let angle_factor = (angle * 3.0).sin() * 0.2 + 0.2;    // 0.0 to 0.4
        let harmonic_factor = (distance * 0.3).cos() * 0.1 + 0.1; // 0.0 to 0.2
        
        // Combine factors for musical frequency
        let frequency_multiplier = spiral_factor + angle_factor + harmonic_factor;
        
        // Map to musical frequency range (220Hz to 3.2kHz)
        self.base_frequency + (frequency_multiplier * self.frequency_range)
    }
    
    /// Transform coordinates to amplitude using wave interference
    pub fn coord_to_amplitude(&self, x: i32, y: i32) -> f32 {
        let norm_x = (x as f32 - self.origin.0) * self.scale_x;
        let norm_y = (y as f32 - self.origin.1) * self.scale_y;
        
        // Wave interference pattern for amplitude
        let wave1 = (norm_x * 0.7).sin();
        let wave2 = (norm_y * 0.9).cos();
        let interference = (wave1 + wave2) * 0.5;
        
        // Distance falloff for spatial audio
        let distance = (norm_x * norm_x + norm_y * norm_y).sqrt();
        let distance_factor = 1.0 / (1.0 + distance * 0.1);
        
        // Combine interference and distance (0.1 to 0.8)
        let amplitude = (interference.abs() * distance_factor).clamp(0.1, 0.8);
        amplitude
    }
    
    /// Transform coordinates to stereo panning (-1.0 to 1.0)
    pub fn coord_to_panning(&self, x: i32, _y: i32) -> f32 {
        let norm_x = (x as f32 - self.origin.0) * self.scale_x;
        
        // Sigmoid function for smooth panning
        let panning = (norm_x * 0.3).tanh();
        panning.clamp(-1.0, 1.0)
    }
    
    /// Transform coordinates to filter cutoff frequency
    /// Higher Y = brighter sound (higher cutoff)
    pub fn coord_to_filter_cutoff(&self, _x: i32, y: i32) -> f32 {
        let norm_y = (y as f32 - self.origin.1) * self.scale_y;
        
        // Exponential mapping for filter cutoff (200Hz to 8kHz)
        let cutoff_factor = (norm_y * 0.2).sin() * 0.5 + 0.5; // 0.0 to 1.0
        let cutoff_freq = 200.0 + cutoff_factor * 7800.0;
        cutoff_freq.clamp(200.0, 8000.0)
    }
    
    /// Get harmonic series based on coordinate cluster
    /// Nearby cells create harmonic relationships
    pub fn get_harmonic_series(&self, x: i32, y: i32, harmonics: usize) -> Vec<f32> {
        let fundamental = self.coord_to_frequency(x, y);
        let mut series = Vec::new();
        
        for i in 1..=harmonics {
            // Create slight detuning for organic sound
            let detune_factor = (x + y + i as i32) as f32 * 0.001;
            let harmonic_freq = fundamental * i as f32 + detune_factor;
            
            // Amplitude decreases with harmonic number
            let harmonic_amp = 1.0 / (i as f32).sqrt();
            
            series.push(harmonic_freq * harmonic_amp);
        }
        
        series
    }
}

/// Specialized mapper for drone layer
pub struct DroneMapper {
    /// Base frequency for drone (40-120 Hz)
    pub base_freq: f32,
    /// Frequency modulation depth
    pub mod_depth: f32,
}

impl DroneMapper {
    pub fn new() -> Self {
        Self {
            base_freq: 58.0, // Low bass drone
            mod_depth: 6.0,  // ±6Hz modulation
        }
    }
    
    /// Calculate drone frequency based on total cell population
    pub fn population_to_drone_freq(&self, population: usize) -> f32 {
        // Slow oscillation based on population
        let pop_factor = (population as f32 * 0.01).sin();
        self.base_freq + (pop_factor * self.mod_depth)
    }
    
    /// Calculate grain density based on local cell density
    pub fn density_to_grain_count(&self, local_density: f32) -> usize {
        // 2-8 grains based on density
        let grain_count = 2.0 + (local_density * 6.0);
        grain_count.clamp(2.0, 8.0) as usize
    }
}

/// Pattern-based frequency mapper
pub struct PatternMapper {
    /// Scale modes for different cellular automaton rules
    scales: std::collections::HashMap<String, Vec<f32>>,
}

impl PatternMapper {
    pub fn new() -> Self {
        let mut scales = std::collections::HashMap::new();
        
        // Dorian mode for Conway (mystical)
        scales.insert("Conway".to_string(), vec![
            1.0, 1.125, 1.2, 1.35, 1.5, 1.6875, 1.8, 2.0
        ]);
        
        // Mixolydian mode for HighLife (bright)
        scales.insert("HighLife".to_string(), vec![
            1.0, 1.125, 1.25, 1.35, 1.5, 1.6875, 1.8, 2.0
        ]);
        
        // Phrygian mode for Seeds (mysterious)
        scales.insert("Seeds".to_string(), vec![
            1.0, 1.067, 1.2, 1.35, 1.5, 1.6, 1.8, 2.0
        ]);
        
        Self { scales }
    }
    
    /// Get scale notes for a given rule
    pub fn get_scale_frequencies(&self, rule: &str, base_freq: f32) -> Vec<f32> {
        let scale_ratios = self.scales.get(rule)
            .unwrap_or(self.scales.get("Conway").unwrap());
            
        scale_ratios.iter().map(|ratio| base_freq * ratio).collect()
    }
    
    /// Map pattern type to musical characteristics
    pub fn pattern_to_melody(&self, pattern_type: &str, base_freq: f32) -> Vec<f32> {
        match pattern_type {
            "Glider" => {
                // Ascending arpeggio
                vec![base_freq, base_freq * 1.25, base_freq * 1.5, base_freq * 2.0]
            },
            "Blinker" => {
                // Alternating notes
                vec![base_freq, base_freq * 1.5]
            },
            "Block" => {
                // Sustained chord
                vec![base_freq, base_freq * 1.25, base_freq * 1.5]
            },
            _ => {
                // Default pattern
                vec![base_freq]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_coordinate_mapping() {
        let mapper = SpatialMapper::new();
        
        // Test frequency mapping stays in audible range
        let freq1 = mapper.coord_to_frequency(0, 0);
        let freq2 = mapper.coord_to_frequency(1000, 1000);
        let freq3 = mapper.coord_to_frequency(-1000, -1000);
        
        assert!(freq1 >= 220.0 && freq1 <= 3220.0);
        assert!(freq2 >= 220.0 && freq2 <= 3220.0);
        assert!(freq3 >= 220.0 && freq3 <= 3220.0);
        
        // Test amplitude mapping
        let amp = mapper.coord_to_amplitude(100, 100);
        assert!(amp >= 0.1 && amp <= 0.8);
        
        println!("✅ Coordinate mapping tests passed");
        println!("   Freq range: {:.1}Hz - {:.1}Hz", freq1, freq2);
        println!("   Amplitude: {:.2}", amp);
    }
} 