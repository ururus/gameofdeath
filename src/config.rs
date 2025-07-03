//! Configuration loader for Game-of-Death.
//!
//! * Looks for `oraclelife.toml` in the cwd unless overridden by `--config`.
//! * Provides defaults so the file is optional.
//!
//! Extend this struct whenever you add new tunables.

use serde::Deserialize;
use std::fs;
// use std::path::Path;

/// Audio engine options
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub enum AudioEngine {
    Spatial,  // Current spatial audio system
    DDSP,     // Neural DDSP synthesis
    DungeonSynth,  // New dungeon synth DDSP engine
    Hybrid,  // New hybrid approach combining synthesis, samples, and neural modulation
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::Spatial  // Default to existing system
    }
}

impl AudioEngine {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "spatial" => AudioEngine::Spatial,
            "ddsp" => AudioEngine::DDSP,
            "dungeonsynth" | "dungeon_synth" | "dungeon-synth" => AudioEngine::DungeonSynth,
            "hybrid" => AudioEngine::Hybrid,
            _ => AudioEngine::Spatial, // Default fallback
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    /// Desired frame-rate cap.
    pub fps:            u32,
    /// Grid size in cells.
    pub cols:           usize,
    pub rows:           usize,
    /// Initial random seed (optional).
    pub seed:           Option<u64>,
    /// Audio engine selection
    #[serde(default)]
    pub audio_engine:   AudioEngine,
    /// Master audio volume (0.0 to 1.0)
    #[serde(default = "default_volume")]
    pub audio_volume:   f32,
}

fn default_volume() -> f32 { 0.7 }

impl Default for Config {
    fn default() -> Self {
        Self {
            fps:  60,
            cols: 100,
            rows: 100,
            seed: None,
            audio_engine: AudioEngine::default(),
            audio_volume: default_volume(),
        }
    }
}

impl Config {
    /// Load from a TOML file; fall back to defaults on any error.
    pub fn load(path: Option<&str>) -> Self {
        let p = path.unwrap_or("oraclelife.toml");
        match fs::read_to_string(p) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_)   => Self::default(),
        }
    }
}