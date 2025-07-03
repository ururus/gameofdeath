use crate::config::Config;

/// Available rule types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleChoice {
    Conway,
    HighLife,
    Seeds,
    Immigration,
    QuadLife,
    BriansBrain,
    Generations,
}

impl RuleChoice {
    /// Cycle to the next variant.
    pub fn next(self) -> Self {
        use RuleChoice::*;
        match self {
            Conway      => HighLife,
            HighLife    => Seeds,
            Seeds       => Immigration,
            Immigration => QuadLife,
            QuadLife    => BriansBrain,
            BriansBrain => Generations,
            Generations => Conway,
        }
    }

    /// Get a string representation of the rule
    pub fn name(&self) -> &'static str {
        match self {
            RuleChoice::Conway => "Conway",
            RuleChoice::HighLife => "HighLife",
            RuleChoice::Seeds => "Seeds",
            RuleChoice::Immigration => "Immigration",
            RuleChoice::QuadLife => "QuadLife", 
            RuleChoice::BriansBrain => "BriansBrain",
            RuleChoice::Generations => "Generations",
        }
    }

    /// Get a description of the rule
    pub fn description(&self) -> &'static str {
        match self {
            RuleChoice::Conway => "B3/S23 - Classic Game of Life",
            RuleChoice::HighLife => "B36/S23 - Conway + birth on 6",
            RuleChoice::Seeds => "B2/S0 - No survival, birth on 2",
            RuleChoice::Immigration => "B3/S23 with Immigration",
            RuleChoice::QuadLife => "QuadLife variant",
            RuleChoice::BriansBrain => "Brian's Brain (3 states)",
            RuleChoice::Generations => "Generational rules",
        }
    }
}

impl Default for RuleChoice {
    fn default() -> Self {
        RuleChoice::Conway
    }
}

// Settings state for Bevy UI
#[derive(Debug, Clone)]
pub struct GameSettings {
    pub grid_cols: usize,
    pub grid_rows: usize,
    pub rule_choice: RuleChoice,
    pub cell_size: f32,
    pub update_interval: f64,
    pub audio_enabled: bool,
    pub audio_volume: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            grid_cols: 100,
            grid_rows: 100,
            rule_choice: RuleChoice::Conway,
            cell_size: 8.0,
            update_interval: 0.1,
            audio_enabled: true,
            audio_volume: 0.5,
        }
    }
}

impl GameSettings {
    pub fn from_config(config: &Config) -> Self {
        Self {
            grid_cols: config.cols,
            grid_rows: config.rows,
            audio_volume: config.audio_volume,
            ..Default::default()
        }
    }

    pub fn to_config(&self) -> Config {
        Config {
            fps: 60,
            cols: self.grid_cols,
            rows: self.grid_rows,
            seed: None,
            audio_engine: crate::config::AudioEngine::Spatial, // Default
            audio_volume: self.audio_volume,
        }
    }
}

// Note: The actual UI rendering will be implemented using Bevy UI components
// in a later step. This module now focuses on the settings data structure.
