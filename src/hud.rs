use bevy::prelude::*;

/// HUD component for displaying game information
#[derive(Component)]
pub struct HudComponent {
    pub show: bool,
    pub last_update_time: f64,
}

impl Default for HudComponent {
    fn default() -> Self {
        Self {
            show: true,
            last_update_time: 0.0,
        }
    }
}

impl HudComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle(&mut self) { 
        self.show = !self.show; 
    }

    pub fn last_update(&self) -> f64 {
        self.last_update_time
    }

    pub fn update(&mut self, current_time: f64) {
        self.last_update_time = current_time;
    }
}

/// HUD display data
pub struct HudData {
    pub fps: f32,
    pub paused: bool,
    pub rule_name: String,
    pub update_interval: f64,
    pub grid_size: (usize, usize),
    pub live_cells: usize,
}

impl Default for HudData {
    fn default() -> Self {
        Self {
            fps: 0.0,
            paused: true,
            rule_name: "Conway".to_string(),
            update_interval: 0.1,
            grid_size: (100, 100),
            live_cells: 0,
        }
    }
}

// Note: The actual UI rendering will be implemented using Bevy UI components
// in a later step. This module now focuses on the HUD data structure.