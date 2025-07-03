use bevy::prelude::Resource;

#[derive(Resource)]
pub struct GameConfig {
    pub current_rule: crate::start_screen::RuleType,
    pub audio_engine: crate::config::AudioEngine,
    pub audio_volume: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            current_rule: crate::start_screen::RuleType::Conway,
            audio_engine: crate::config::AudioEngine::Spatial,
            audio_volume: 0.7,
        }
    }
} 