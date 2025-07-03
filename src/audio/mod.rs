// Core audio modules that are actually used
pub mod spatial_mapping;
pub mod spatial_audio;
pub mod ddsp_engine;
pub mod ddsp_game_analysis;
pub mod scales;
pub mod hybrid_dungeon_synth;
pub mod kira_manager;
pub mod illbient_groove;

// Re-export spatial audio functions (the advanced system)
pub use spatial_audio::{
    init_spatial_audio,
    update_camera_position,
    process_spatial_audio,
    update_spatial_population,
    play_spatial_cell_birth,
    play_spatial_cell_death,
    toggle_spatial_audio,
    has_spatial_audio,
    get_active_voice_count,
};

// Re-export DDSP neural audio functions
pub use ddsp_engine::{
    GameStateFeatures,
};

pub use ddsp_game_analysis::{
    extract_game_features,
};

// Re-export hybrid dungeon synth functions
pub use hybrid_dungeon_synth::*;
pub use hybrid_dungeon_synth::get_scale_root;
pub use hybrid_dungeon_synth::set_hybrid_synthesis_mix;
pub use kira_manager::{KiraManager, setup_kira};
pub use illbient_groove::IllbientGroove;

// Re-export spatial mapping
pub use spatial_mapping::{SpatialMapper, DroneMapper, PatternMapper};

// Basic audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub enabled: bool,
    pub ambient_mode: bool,
    pub spatial_audio: bool,
    pub master_volume: f32,
    pub cell_birth_volume: f32,
    pub cell_death_volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ambient_mode: true,
            spatial_audio: true,
            master_volume: 0.7,
            cell_birth_volume: 0.5,
            cell_death_volume: 0.3,
        }
    }
}

// pub mod engine;  // Temporarily disabled due to Bevy audio API deprecation issues
// pub mod generative_engine;  // Temporarily disabled due to threading issues
// pub mod patterns;
// pub mod realtime_audio;     // Temporarily disabled due to threading issues
// pub mod simple_global;

// Export all audio functions for external use
pub use spatial_audio::*;
// pub use ddsp_output::*;    // Temporarily disabled due to threading issues
// pub use ddsp_ambient::*;   // Temporarily disabled due to threading issues
// pub use simple_global::{
//     init_global_audio, 
//     play_cell_birth, 
//     play_cell_death, 
//     update_population, 
//     toggle_audio, 
//     has_audio
// }; 