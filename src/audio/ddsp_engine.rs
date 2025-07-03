use std::sync::Mutex;
use std::f32::consts::PI;

// Extension trait for f32 to add sigmoid function
trait SigmoidExt {
    fn sigmoid(self) -> Self;
}

impl SigmoidExt for f32 {
    fn sigmoid(self) -> Self {
        1.0 / (1.0 + (-self).exp())
    }
}

/// Lightweight neural network for harmonic generation
/// This is a simple 2-layer MLP that can be loaded from pretrained weights
pub struct HarmonicDecoder {
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
    // Weights for a simple 2-layer network
    weights1: Vec<Vec<f32>>,
    bias1: Vec<f32>,
    weights2: Vec<Vec<f32>>,
    bias2: Vec<f32>,
}

impl HarmonicDecoder {
    pub fn new(input_size: usize, hidden_size: usize, output_size: usize) -> Self {
        // Initialize with random weights for now (would load from pretrained later)
        let mut weights1 = vec![vec![0.0; input_size]; hidden_size];
        let mut weights2 = vec![vec![0.0; hidden_size]; output_size];
        
        // Initialize with better weights that produce audible output
        for i in 0..hidden_size {
            for j in 0..input_size {
                // Bias towards producing some output
                weights1[i][j] = (rand::random::<f32>() - 0.3) * 0.3;
            }
        }
        
        for i in 0..output_size {
            for j in 0..hidden_size {
                // Ensure some harmonics are always active
                weights2[i][j] = (rand::random::<f32>() - 0.2) * 0.4;
            }
        }
        
        Self {
            input_size,
            hidden_size,
            output_size,
            weights1,
            bias1: vec![0.1; hidden_size], // Small positive bias
            weights2,
            bias2: vec![0.0; output_size],
        }
    }
    
    /// Forward pass through the network
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        assert_eq!(input.len(), self.input_size);
        
        // Hidden layer
        let mut hidden = vec![0.0; self.hidden_size];
        for i in 0..self.hidden_size {
            for j in 0..self.input_size {
                hidden[i] += self.weights1[i][j] * input[j];
            }
            hidden[i] += self.bias1[i];
            hidden[i] = hidden[i].tanh(); // Activation function
        }
        
        // Output layer
        let mut output = vec![0.0; self.output_size];
        for i in 0..self.output_size {
            for j in 0..self.hidden_size {
                output[i] += self.weights2[i][j] * hidden[j];
            }
            output[i] += self.bias2[i];
            output[i] = output[i].sigmoid(); // Normalize to [0,1]
        }
        
        output
    }
}

/// Similar lightweight network for noise generation
pub struct NoiseDecoder {
    decoder: HarmonicDecoder,
}

impl NoiseDecoder {
    pub fn new(input_size: usize) -> Self {
        Self {
            decoder: HarmonicDecoder::new(input_size, 32, 64), // Small noise decoder
        }
    }
    
    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        self.decoder.forward(input)
    }
}

/// Game state features extracted from the grid
#[derive(Debug, Clone)]
pub struct GameStateFeatures {
    pub population: f32,           // Current population normalized
    pub density: f32,             // Population density in viewport
    pub activity: f32,            // Rate of change (births/deaths per frame)
    pub cluster_count: f32,       // Number of separate clusters
    pub avg_cluster_size: f32,    // Average cluster size
    pub symmetry: f32,            // Measure of symmetry in current view
    pub chaos: f32,               // Measure of randomness vs patterns
    pub generation: f32,          // Current generation normalized
    pub centroid_x: f32,          // Centroid X position of live cells (-1..1)
    pub centroid_y: f32,          // Centroid Y position of live cells (-1..1)
}

impl Default for GameStateFeatures {
    fn default() -> Self {
        Self {
            population: 0.0,
            density: 0.0,
            activity: 0.0,
            cluster_count: 0.0,
            avg_cluster_size: 0.0,
            symmetry: 0.0,
            chaos: 0.0,
            generation: 0.0,
            centroid_x: 0.0,
            centroid_y: 0.0,
        }
    }
}

impl GameStateFeatures {
    /// Convert to input vector for neural networks
    pub fn to_vector(&self) -> Vec<f32> {
        vec![
            self.population,
            self.density,
            self.activity,
            self.cluster_count,
            self.avg_cluster_size,
            self.symmetry,
            self.chaos,
            self.generation,
            self.centroid_x,
            self.centroid_y,
        ]
    }
}

/// DDSP-style oscillator bank
pub struct HarmonicOscillator {
    sample_rate: f32,
    num_harmonics: usize,
    phases: Vec<f32>,
    _base_freq: f32,
}

impl HarmonicOscillator {
    pub fn new(sample_rate: f32, num_harmonics: usize) -> Self {
        Self {
            sample_rate,
            num_harmonics,
            phases: vec![0.0; num_harmonics],
            _base_freq: 220.0, // Base frequency in Hz
        }
    }
    
    /// Generate audio sample using harmonic amplitudes from neural network
    pub fn generate_sample(&mut self, harmonic_amps: &[f32], fundamental_freq: f32) -> f32 {
        let mut sample = 0.0;
        
        for (i, &amp) in harmonic_amps.iter().enumerate().take(self.num_harmonics) {
            let harmonic = (i + 1) as f32;
            let freq = fundamental_freq * harmonic;
            let phase_increment = 2.0 * PI * freq / self.sample_rate;
            
            // Generate harmonic
            sample += amp * self.phases[i].sin();
            
            // Update phase
            self.phases[i] += phase_increment;
            if self.phases[i] > 2.0 * PI {
                self.phases[i] -= 2.0 * PI;
            }
        }
        
        sample
    }
}

/// Noise generator with neural control
pub struct NoiseGenerator {
    _sample_rate: f32,
    noise_buffer: Vec<f32>,
    buffer_index: usize,
}

impl NoiseGenerator {
    pub fn new(sample_rate: f32) -> Self {
        let buffer_size = 1024;
        let mut noise_buffer = vec![0.0; buffer_size];
        
        // Fill with white noise
        for sample in noise_buffer.iter_mut() {
            *sample = (rand::random::<f32>() - 0.5) * 2.0;
        }
        
        Self {
            _sample_rate: sample_rate,
            noise_buffer,
            buffer_index: 0,
        }
    }
    
    /// Generate filtered noise sample
    pub fn generate_sample(&mut self, noise_params: &[f32]) -> f32 {
        let base_noise = self.noise_buffer[self.buffer_index];
        self.buffer_index = (self.buffer_index + 1) % self.noise_buffer.len();
        
        // Apply neural filtering (simplified)
        let filtered = if noise_params.len() >= 2 {
            base_noise * noise_params[0] * (1.0 + noise_params[1] * 0.5)
        } else {
            base_noise * 0.1
        };
        
        filtered
    }
}

/// Simple reverb using convolution (traditional DSP component)
pub struct ConvolutionReverb {
    impulse_response: Vec<f32>,
    delay_buffer: Vec<f32>,
    buffer_index: usize,
}

impl ConvolutionReverb {
    pub fn new(sample_rate: f32) -> Self {
        // Create a simple exponential decay impulse response
        let length = (sample_rate * 2.0) as usize; // 2 second reverb
        let mut impulse_response = vec![0.0; length];
        
        for (i, sample) in impulse_response.iter_mut().enumerate() {
            let t = i as f32 / sample_rate;
            *sample = (-t * 3.0).exp() * (rand::random::<f32>() - 0.5) * 0.3;
        }
        
        Self {
            impulse_response,
            delay_buffer: vec![0.0; length],
            buffer_index: 0,
        }
    }
    
    pub fn process(&mut self, input: f32) -> f32 {
        // Store input in delay buffer
        self.delay_buffer[self.buffer_index] = input;
        
        // Convolve with impulse response (simplified)
        let mut output = 0.0;
        for (i, &ir_sample) in self.impulse_response.iter().enumerate().take(256) { // Limit for performance
            let delay_index = (self.buffer_index + self.delay_buffer.len() - i) % self.delay_buffer.len();
            output += self.delay_buffer[delay_index] * ir_sample;
        }
        
        self.buffer_index = (self.buffer_index + 1) % self.delay_buffer.len();
        
        // Mix dry and wet signal
        input * 0.7 + output * 0.3
    }
}

/// Main DDSP audio engine
pub struct DDSPAudioEngine {
    harmonic_decoder: HarmonicDecoder,
    noise_decoder: NoiseDecoder,
    harmonic_osc: HarmonicOscillator,
    noise_gen: NoiseGenerator,
    reverb_l: ConvolutionReverb,
    reverb_r: ConvolutionReverb,
    _sample_rate: f32,
    enabled: bool,
    current_features: GameStateFeatures,
}

impl DDSPAudioEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            harmonic_decoder: HarmonicDecoder::new(GameStateFeatures::default().to_vector().len(), 64, 32),
            noise_decoder: NoiseDecoder::new(GameStateFeatures::default().to_vector().len()),
            harmonic_osc: HarmonicOscillator::new(sample_rate, 32),
            noise_gen: NoiseGenerator::new(sample_rate),
            reverb_l: ConvolutionReverb::new(sample_rate),
            reverb_r: ConvolutionReverb::new(sample_rate),
            _sample_rate: sample_rate,
            enabled: true, // Start enabled by default
            current_features: GameStateFeatures::default(),
        }
    }
    
    /// Update game state features
    pub fn update_features(&mut self, features: GameStateFeatures) {
        self.current_features = features;
    }
    
    /// Generate stereo audio sample
    pub fn generate_stereo_sample(&mut self) -> (f32, f32) {
        if !self.enabled {
            return (0.0, 0.0);
        }
        
        // Convert game features to neural network input (with minimum values for audible output)
        let mut input = self.current_features.to_vector();
        
        // Ensure minimum activity for audible sound even with empty grid
        input[0] = input[0].max(0.05); // Minimum population
        input[1] = input[1].max(0.02); // Minimum density
        input[2] = input[2].max(0.01); // Minimum activity
        
        // Get harmonic amplitudes from neural network
        let mut harmonic_amps = self.harmonic_decoder.forward(&input);
        
        // Boost first few harmonics to ensure audible output
        for i in 0..4.min(harmonic_amps.len()) {
            harmonic_amps[i] = harmonic_amps[i].max(0.3);
        }
        
        // Get noise parameters from neural network
        let noise_params = self.noise_decoder.forward(&input);
        
        // Calculate fundamental frequency based on game state (always audible)
        let fundamental = 220.0 + self.current_features.density * 330.0 + 
                         self.current_features.activity * 220.0 + 
                         self.current_features.population * 110.0;
        
        // Generate harmonic component
        let harmonic_sample = self.harmonic_osc.generate_sample(&harmonic_amps, fundamental);
        
        // Generate noise component
        let noise_sample = self.noise_gen.generate_sample(&noise_params);
        
        // Mix components (boost overall volume)
        let dry_l = (harmonic_sample * 0.8 + noise_sample * 0.2) * 3.0; // Further boost volume
        let dry_r = (harmonic_sample * 0.7 + noise_sample * 0.3) * 3.0; // Slightly different mix for stereo
        
        // Apply reverb
        let wet_l = self.reverb_l.process(dry_l);
        let wet_r = self.reverb_r.process(dry_r);
        
        (wet_l * 0.5, wet_r * 0.5) // Final volume control
    }
    
    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.enabled
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// Global DDSP engine instance
static DDSP_ENGINE: Mutex<Option<DDSPAudioEngine>> = Mutex::new(None);

/// Initialize DDSP audio system
pub fn init_ddsp_audio() {
    let sample_rate = 44100.0;
    let engine = DDSPAudioEngine::new(sample_rate);
    
    let mut global_engine = DDSP_ENGINE.lock().unwrap();
    *global_engine = Some(engine);
    
    println!("🎵 DDSP Neural Audio Engine initialized");
}

/// Update DDSP system with current game state
pub fn update_ddsp_audio(features: GameStateFeatures) {
    if let Ok(mut engine_guard) = DDSP_ENGINE.lock() {
        if let Some(ref mut engine) = *engine_guard {
            engine.update_features(features);
        }
    }
}

/// Generate stereo audio samples (called from audio callback)
pub fn generate_ddsp_samples(buffer: &mut [(f32, f32)]) {
    if let Ok(mut engine_guard) = DDSP_ENGINE.lock() {
        if let Some(ref mut engine) = *engine_guard {
            for sample in buffer.iter_mut() {
                *sample = engine.generate_stereo_sample();
            }
        }
    }
}

/// Toggle DDSP audio
pub fn toggle_ddsp_audio() -> bool {
    if let Ok(mut engine_guard) = DDSP_ENGINE.lock() {
        if let Some(ref mut engine) = *engine_guard {
            return engine.toggle();
        }
    }
    false
}

/// Check if DDSP audio is enabled
pub fn has_ddsp_audio() -> bool {
    if let Ok(engine_guard) = DDSP_ENGINE.lock() {
        if let Some(ref engine) = *engine_guard {
            return engine.is_enabled();
        }
    }
    false
} 