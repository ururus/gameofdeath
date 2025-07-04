use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, DiagnosticsStore}, window::PrimaryWindow};
use std::collections::HashSet;

// Import our modules
use gameofdeath::*;
use gameofdeath::camera::{setup_camera, handle_camera_controls, GameCamera, CameraState, world_to_grid, screen_to_world};
use gameofdeath::start_screen::{GameState, SelectedRule, RuleType, setup_start_screen, handle_start_screen_input, cleanup_start_screen, update_start_screen_ui};
use gameofdeath::ui::{setup_ui, UiState, RuleControlsContainer, RuleControlText};
use gameofdeath::cell_renderer::{CellRenderConfig, CellTextureCache, CellTexturePool, render_optimized_cells, update_cell_render_config, animate_cell_textures, CellAnimation, AnimationType};
use gameofdeath::audio::{
    extract_game_features,
    update_hybrid_dungeon_synth,
    update_hybrid_cell_data,
    set_hybrid_volume,
    get_hybrid_volume,
    init_hybrid_dungeon_synth,
    setup_kira,
    IllbientGroove,
};
use gameofdeath::config::{Config, AudioEngine};
use gameofdeath::GameConfig;
use gameofdeath::synth_ui::SynthControlPanelPlugin;

/// Custom font resource for the game
#[derive(Resource)]
pub struct GameFont {
    pub handle: Handle<Font>,
}

// CellEntity component removed as it was unused



/// Audio enabled state resource
#[derive(Resource)]
pub struct AudioEnabled {
    pub spatial: bool,
    pub ddsp: bool,
    pub hybrid: bool,
}

impl Default for AudioEnabled {
    fn default() -> Self {
        Self {
            spatial: true,
            ddsp: false,
            hybrid: false,
        }
    }
}

/// Setup game audio based on configuration
fn setup_game_audio(game_config: Res<GameConfig>) {
    setup_audio(game_config.audio_engine, game_config.audio_volume);
}

/// Setup audio system
fn setup_audio(audio_engine: AudioEngine, volume: f32) {
    match audio_engine {
        AudioEngine::Spatial => {
            // Spatial audio removed - redirect to hybrid
            println!("🔊 Spatial audio disabled, using Hybrid instead");
            init_hybrid_dungeon_synth();
            set_hybrid_volume(volume);
            println!("🔮 Hybrid Dungeon Synth Engine ready! Volume: {:.0}%", volume * 100.0);
        }
        AudioEngine::DDSP => {
            // DDSP functionality temporarily disabled
            println!("🎵 DDSP Audio Engine (placeholder)");
        }
        AudioEngine::DungeonSynth => {
            // Dungeon synth functionality temporarily disabled
            println!("🏰 Dungeon Synth DDSP Audio Engine (placeholder)");
        }
        AudioEngine::Hybrid => {
            // Initialize hybrid dungeon synth engine
            init_hybrid_dungeon_synth();
            set_hybrid_volume(volume);
            println!("🔮 Hybrid Dungeon Synth Engine ready! Volume: {:.0}%", volume * 100.0);
        }
    }
    
    // Display comprehensive controls
    println!("\n🎮 GAME CONTROLS:");
    println!("  SPACE - Pause/Resume simulation");
    println!("  C - Clear grid");
    println!("  S - Single step (when paused)");
    println!("  +/- - Speed up/slow down");
    match audio_engine {
        AudioEngine::Spatial | AudioEngine::Hybrid => {
            println!("  M - Toggle hybrid dungeon synth audio");
            println!("  N - Show hybrid audio status");
            println!("  ↑/↓ - Volume up/down");
        }
        AudioEngine::DDSP => {
            println!("  M - Toggle DDSP neural audio");
            println!("  N - Show DDSP audio status");
        }
        AudioEngine::DungeonSynth => {
            println!("  M - Toggle dungeon synth audio");
            println!("  N - Show dungeon synth audio status");
        }
    }
    println!("  V - Toggle color variation");
    println!("  G - Toggle generation colors");
    println!("  [ ] - Adjust noise density");
    println!("  Note: Cells animate automatically (living textures!)");
    println!("  ESC - Return to start screen");
    println!("  Mouse - Click to add/remove cells");
    println!("  Arrow keys/WASD - Move camera");
    println!("  Mouse wheel - Zoom in/out");
    println!("  H - Toggle UI visibility");
}

/// Setup custom font system
fn setup_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/Geo-Regular.ttf");
    commands.insert_resource(GameFont {
        handle: font_handle.clone(),
    });
    println!("🔤 Loading custom font: Geo-Regular.ttf");
}

/// Game statistics
#[derive(Resource)]
pub struct GameStats {
    pub is_running: bool,
    pub generation: u64,
    pub last_update: f64,
    pub update_interval: f64,
    pub min_update_interval: f64,
    pub max_update_interval: f64,
}

impl Default for GameStats {
    fn default() -> Self {
        Self {
            is_running: false,
            generation: 0,
            last_update: 0.0,
            update_interval: 0.5,
            min_update_interval: 0.01,
            max_update_interval: 2.0,
        }
    }
}

/// Cached audio state to prevent repeated calculations
#[derive(Resource)]
pub struct AudioCache {
    pub last_features: [f32; 8],
    pub last_cell_count: usize,
    pub last_generation: u64,
    pub update_threshold: f32, // Minimum change required to update audio
    pub cell_count_threshold: usize, // Minimum cell count change
    pub generation_throttle: u64, // Only log every N generations when stable
}

impl Default for AudioCache {
    fn default() -> Self {
        Self {
            last_features: [0.0; 8],
            last_cell_count: 0,
            last_generation: 0,
            update_threshold: 0.005, // 0.5% change threshold (more sensitive)
            cell_count_threshold: 2, // Update on 2+ cell changes (more sensitive)
            generation_throttle: 10, // Log every 10 generations when stable (less spam)
        }
    }
}

/// Current brush parameters for cell placement
#[derive(Resource)]
pub struct BrushSettings {
    pub size: u32, // side length of square brush
}

impl Default for BrushSettings {
    fn default() -> Self { Self { size: 1 } }
}

#[derive(Resource, Default)]
pub struct OverlayCache {
    version: u64,
    horiz: HashSet<(i32, i32)>,
    vert: HashSet<(i32, i32)>,
}

impl OverlayCache {
    /// Recompute overlay flags when the grid has changed
    fn recompute(&mut self, grid: &InfiniteGrid) {
        // Clear existing data while keeping capacity
        self.horiz.clear();
        self.vert.clear();

        // Take a snapshot of all alive positions to avoid borrow conflicts
        let positions = grid.get_alive_cells_snapshot();
        for &(x, y) in &positions {
            // Check right neighbour once (left handled when we reach that cell)
            if grid.is_alive(x + 1, y) {
                self.horiz.insert((x, y));
                self.horiz.insert((x + 1, y));
            }
            // Check upper neighbour once (bottom handled when we reach that cell)
            if grid.is_alive(x, y + 1) {
                self.vert.insert((x, y));
                self.vert.insert((x, y + 1));
            }
        }
        self.version = grid.version();
    }
}

fn handle_game_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_stats: ResMut<GameStats>,
    mut grid: ResMut<InfiniteGrid>,
    mut game_state: ResMut<NextState<GameState>>,
    mut game_config: ResMut<GameConfig>,
    mut brush: ResMut<BrushSettings>,
) {
    // Pause/Resume
    if keyboard_input.just_pressed(KeyCode::Space) {
        game_stats.is_running = !game_stats.is_running;
    }

    // Speed controls
    if keyboard_input.just_pressed(KeyCode::Equal) {
        game_stats.update_interval = (game_stats.update_interval * 0.8).max(game_stats.min_update_interval);
    }
    if keyboard_input.just_pressed(KeyCode::Minus) {
        game_stats.update_interval = (game_stats.update_interval * 1.25).min(game_stats.max_update_interval);
    }

    // Clear grid
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        grid.clear();
        game_stats.generation = 0;
        game_stats.is_running = false;
    }

    // Reset game and return to start screen (R key)
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        // Clear the grid completely
        grid.clear();
        // Reset game stats
        game_stats.generation = 0;
        game_stats.is_running = false;
        // Navigate to start screen
        game_state.set(GameState::StartScreen);
        println!("🔄 Game reset - returning to start screen");
    }
    
    // Just return to start screen without reset (ESC key)
    if keyboard_input.just_pressed(KeyCode::Escape) {
        game_state.set(GameState::StartScreen);
    }

    // Step simulation
    if keyboard_input.just_pressed(KeyCode::KeyS) && !game_stats.is_running {
        // Single step
        game_stats.generation += 1;
    }

    // Audio controls
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        // Toggle audio based on current engine
        match game_config.audio_engine {
            AudioEngine::Spatial | AudioEngine::Hybrid => {
                // For now, just print status since we removed spatial audio
                println!("🔮 Hybrid audio is always enabled");
            }
            AudioEngine::DDSP => {
                println!("🎵 DDSP audio toggle (placeholder)");
            }
            AudioEngine::DungeonSynth => {
                println!("🏰 Dungeon synth audio toggle (placeholder)");
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyN) {
        // Show audio status
        match game_config.audio_engine {
            AudioEngine::Spatial | AudioEngine::Hybrid => {
                println!("🔮 Hybrid audio: Volume {:.0}%", get_hybrid_volume() * 100.0);
            }
            AudioEngine::DDSP => {
                println!("🎵 DDSP audio status (placeholder)");
            }
            AudioEngine::DungeonSynth => {
                println!("🏰 Dungeon synth audio status (placeholder)");
            }
        }
    }

    // Volume controls (< and > keys) - now supports overdrive up to 200%
    if keyboard_input.just_pressed(KeyCode::Period) { // > key (shift not required)
        game_config.audio_volume = (game_config.audio_volume + 0.1).min(2.0); // Allow up to 200%
        match game_config.audio_engine {
            AudioEngine::Hybrid => {
                set_hybrid_volume(game_config.audio_volume);
                if game_config.audio_volume > 1.0 {
                    println!("🔊🔥 OVERDRIVE! Volume: {:.0}%", game_config.audio_volume * 100.0);
                } else {
                    println!("🔊 Volume: {:.0}%", game_config.audio_volume * 100.0);
                }
            }
            _ => {
                println!("🔊 Volume: {:.0}% (applies to hybrid engine only)", game_config.audio_volume * 100.0);
            }
        }
    }
    
    if keyboard_input.just_pressed(KeyCode::Comma) { // < key (shift not required)
        game_config.audio_volume = (game_config.audio_volume - 0.1).max(0.0);
        match game_config.audio_engine {
            AudioEngine::Hybrid => {
                set_hybrid_volume(game_config.audio_volume);
                println!("🔊 Volume: {:.0}%", game_config.audio_volume * 100.0);
            }
            _ => {
                println!("🔊 Volume: {:.0}% (applies to hybrid engine only)", game_config.audio_volume * 100.0);
            }
        }
    }

    // Brush size controls with [ and ]
    if keyboard_input.just_pressed(KeyCode::BracketLeft) {
        brush.size = brush.size.saturating_sub(1).max(1);
        println!("🖌️ Brush size: {}", brush.size);
    }
    if keyboard_input.just_pressed(KeyCode::BracketRight) {
        brush.size = (brush.size + 1).min(20);
        println!("🖌️ Brush size: {}", brush.size);
    }
}

fn handle_mouse_input(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &OrthographicProjection, &GameCamera), With<GameCamera>>,
    camera_state: Res<CameraState>,
    mut grid: ResMut<InfiniteGrid>,
    game_config: Res<GameConfig>,
    brush: Res<BrushSettings>,
) {
    // Use pressed() for continuous placement while holding down mouse button
    if mouse_button_input.pressed(MouseButton::Left) || mouse_button_input.pressed(MouseButton::Right) {
        if let (Ok(window), Ok((camera_transform, projection, _game_camera))) = (windows.get_single(), camera_query.get_single()) {
            if let Some(cursor_position) = window.cursor_position() {
                // Use the working coordinate conversion
                let window_size = Vec2::new(window.width(), window.height());
                
                let world_pos = screen_to_world(cursor_position, camera_transform, projection, window_size);
                let (grid_x, grid_y) = world_to_grid(world_pos, &camera_state);

                let shift = keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);
                let alt = keyboard_input.pressed(KeyCode::AltLeft) || keyboard_input.pressed(KeyCode::AltRight);

                if mouse_button_input.pressed(MouseButton::Left) {
                    let state = state_for_click(game_config.current_rule, MouseButton::Left, shift, alt);
                    apply_brush(&mut grid, grid_x, grid_y, brush.size, state);
                }
                
                if mouse_button_input.pressed(MouseButton::Right) {
                    let state = state_for_click(game_config.current_rule, MouseButton::Right, shift, alt);
                    apply_brush(&mut grid, grid_x, grid_y, brush.size, state);
                }
            }
        }
    }
}

fn apply_brush(grid: &mut InfiniteGrid, cx: i32, cy: i32, size: u32, state: CellState) {
    let half = (size as i32) / 2;
    for dy in -half..=half {
        for dx in -half..=half {
            grid.set(cx + dx, cy + dy, state);
        }
    }
}

/// Return the cell state that should be written for a click under the given rule.
/// Left click usually creates, right click either deletes or places an alternate species.
fn state_for_click(rule: RuleType, button: MouseButton, shift: bool, alt: bool) -> CellState {
    use MouseButton::{Left, Right};
    match rule {
        RuleType::WireWorld => {
            if shift { CellState::ElectronHead }
            else if alt { CellState::ElectronTail }
            else if button == Left { CellState::Wire } else { CellState::Dead }
        }
        RuleType::Immigration => {
            match button {
                Left => CellState::SpeciesA,
                Right => CellState::SpeciesB,
                _ => CellState::Dead,
            }
        }
        // For Brian's Brain a firing cell is represented by Alive
        RuleType::Brian => {
            if shift { CellState::Dying }
            else if button == Left { CellState::Alive } else { CellState::Dead }
        }
        _ => {
            // Default Life-like rules: place Alive, remove on right-click
            if button == Left { CellState::Alive } else { CellState::Dead }
        }
    }
}

fn update_simulation(
    time: Res<Time>,
    mut game_stats: ResMut<GameStats>,
    mut grid: ResMut<InfiniteGrid>,
    game_config: Res<GameConfig>,
) {
    if !game_stats.is_running {
        return;
    }

    let current_time = time.elapsed_secs_f64();
    if current_time - game_stats.last_update >= game_stats.update_interval {
        grid.update(game_config.current_rule);
        game_stats.generation += 1;
        game_stats.last_update = current_time;
    }
}

fn update_audio_system(
    mut grid: ResMut<InfiniteGrid>, 
    camera_query: Query<&Transform, With<GameCamera>>,
    camera_state: Res<CameraState>,
    game_stats: Res<GameStats>,
    game_config: Res<GameConfig>,
    mut audio_cache: ResMut<AudioCache>,
    mut groove: Option<NonSendMut<IllbientGroove>>,
) {
    match game_config.audio_engine {
        AudioEngine::Spatial | AudioEngine::Hybrid => {
            // All audio engines now use hybrid processing for consistency and performance
            let features = extract_game_features(&grid, &camera_state, game_stats.generation);
            
            // Get alive cells for optimized processing (now returns a reference)
            let alive_cells = grid.get_alive_cells();
            
            // Update cell data for optimized processing and spatial modulation
            if let Ok(camera_transform) = camera_query.get_single() {
                let viewport_size = camera_state.cell_size * 50.0; // Approximate viewport size
                update_hybrid_cell_data(
                    alive_cells, // Pass reference instead of owned vector
                    camera_transform.translation.x,
                    camera_transform.translation.y,
                    viewport_size,
                );
            }
            
            // Convert to simple array format for hybrid engine
            let feature_array = [
                features.population,
                features.density, 
                features.activity,
                features.cluster_count,
                features.avg_cluster_size,
                features.symmetry,
                features.chaos,
                features.generation,
            ];
            
            // Check if audio update is needed (avoid redundant calculations)
            let mut should_update = false;
            let mut should_log = false;
            let cell_count = alive_cells.len();
            
            // Update if generation changed
            if game_stats.generation != audio_cache.last_generation {
                should_update = true;
            }
            // Update if cell count changed significantly (more sensitive)
            else if (cell_count as i32 - audio_cache.last_cell_count as i32).abs() >= audio_cache.cell_count_threshold as i32 {
                should_update = true;
                should_log = true; // Always log significant cell count changes
            }
            // Update if any feature changed significantly (more sensitive)
            else {
                for (i, &new_val) in feature_array.iter().enumerate() {
                    let old_val = audio_cache.last_features[i];
                    let change = (new_val - old_val).abs();
                    if change > audio_cache.update_threshold {
                        should_update = true;
                        // Log feature changes but not too frequently
                        if i < 3 || change > audio_cache.update_threshold * 2.0 { // Log major features or big changes
                            should_log = true;
                        }
                        break;
                    }
                }
            }
            
            // Only update audio if something significant changed
            if should_update {
                update_hybrid_dungeon_synth(feature_array);
                
                // Drive illbient groove
                if let Some(mut g) = groove {
                    let root = gameofdeath::audio::get_scale_root();
                    g.update(&features, root);
                }
                
                // Determine if we should log this update
                let generation_changed = game_stats.generation != audio_cache.last_generation;
                let cell_count_changed = cell_count != audio_cache.last_cell_count;
                let throttled_generation = game_stats.generation % audio_cache.generation_throttle == 0;
                
                should_log = should_log || cell_count_changed || (generation_changed && throttled_generation);
                
                // Update cache
                audio_cache.last_features = feature_array;
                audio_cache.last_cell_count = cell_count;
                audio_cache.last_generation = game_stats.generation;
                
                // Smart logging - less spam, more meaningful updates
                if should_log {
                    println!("🔮 Hybrid cells:{} features: [{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.0}]", 
                        cell_count,
                        feature_array[0], feature_array[1], feature_array[2], feature_array[3],
                        feature_array[4], feature_array[5], feature_array[6], feature_array[7]);
                }
            }
        }
        AudioEngine::DDSP => {
            // Extract game features for DDSP
            let features = extract_game_features(&grid, &camera_state, game_stats.generation);
            // DDSP processing placeholder
            println!("🎵 DDSP features: {:?}", features);
        }
        AudioEngine::DungeonSynth => {
            // Dungeon synth processing placeholder
            println!("🏰 Dungeon synth processing (placeholder)");
        }
    }
}

/// Update game UI state
fn update_game_ui(
    mut ui_state: ResMut<UiState>,
    game_stats: Res<GameStats>,
    mut grid: ResMut<InfiniteGrid>,
    game_config: Res<GameConfig>,
    diagnostics: Res<DiagnosticsStore>,
    camera_query: Query<&GameCamera>,
) {
    ui_state.generation = game_stats.generation;
    ui_state.is_running = game_stats.is_running;
    ui_state.update_interval = game_stats.update_interval;
    ui_state.current_rule = match game_config.current_rule {
        RuleType::Conway => "Conway".to_string(),
        RuleType::HighLife => "HighLife".to_string(),
        RuleType::Seeds => "Seeds".to_string(),
        RuleType::Brian => "Brian's Brain".to_string(),
        RuleType::WireWorld => "WireWorld".to_string(),
        RuleType::Immigration => "Immigration".to_string(),
        RuleType::Mazectric => "Mazectric".to_string(),
        RuleType::Coral => "Coral".to_string(),
        RuleType::Gnarl => "Gnarl".to_string(),
        RuleType::Replicator => "Replicator".to_string(),
    };
    ui_state.population = grid.get_alive_cells().len();
    ui_state.audio_volume = game_config.audio_volume;
    
    // Update zoom level from camera
    if let Ok(camera) = camera_query.get_single() {
        ui_state.zoom_level = camera.zoom;
    }
    
    // Update FPS
    if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps) = fps_diagnostic.smoothed() {
            ui_state.fps = fps;
        }
    }
}

/// Show HUD when entering playing state
fn show_hud(
    mut hud_query: Query<&mut Visibility, With<gameofdeath::ui::HudContainer>>,
    mut ui_state: ResMut<UiState>,
) {
    for mut visibility in hud_query.iter_mut() {
        *visibility = Visibility::Visible;
    }
    ui_state.hud_visible = true;
}

/// Hide HUD when exiting playing state
fn hide_hud(
    mut hud_query: Query<&mut Visibility, With<gameofdeath::ui::HudContainer>>,
    mut ui_state: ResMut<UiState>,
) {
    for mut visibility in hud_query.iter_mut() {
        *visibility = Visibility::Hidden;
    }
    ui_state.hud_visible = false;
}

/// Handle exiting start screen state
fn on_exit_start_screen(game_config: Res<GameConfig>) {
    match game_config.audio_engine {
        AudioEngine::DDSP => {
            println!("🎵 Switching DDSP audio to game mode (placeholder)");
        }
        AudioEngine::DungeonSynth => {
            println!("🏰 Switching dungeon synth to game mode (placeholder)");
        }
        AudioEngine::Hybrid => {
            println!("🔮 Switching hybrid audio to game mode");
        }
        _ => {}
    }
}

/// Handle entering playing state
fn on_enter_playing(
    mut game_config: ResMut<GameConfig>,
    mut grid: ResMut<InfiniteGrid>,
    mut game_stats: ResMut<GameStats>,
    selected_rule: Res<SelectedRule>,
) {
    // Apply the selected rule from start screen to game config
    game_config.current_rule = selected_rule.current;
    println!("🎯 Applied rule: {} to game", selected_rule.current.name());
    
    // Ensure no leftover exotic states from a previous game carry over.
    grid.clear();
    
    // Rule-specific speed presets
    game_stats.update_interval = match game_config.current_rule {
        RuleType::Seeds => 0.05,
        RuleType::Coral => 0.1,
        RuleType::Gnarl => 0.02,
        _ => 0.2,
    };

    match game_config.audio_engine {
        AudioEngine::DDSP => {
            println!("🎮 Game mode: DDSP neural audio active (placeholder)");
        }
        AudioEngine::Spatial => {
            println!("🎮 Game mode: Spatial polyphonic audio active");
        }
        AudioEngine::DungeonSynth => {
            println!("🎮 Game mode: Dungeon synth audio active (placeholder)");
        }
        AudioEngine::Hybrid => {
            println!("🎮 Game mode: Hybrid dungeon synth audio active");
        }
    }
}

/// Handle exiting playing state  
fn on_exit_playing(game_config: Res<GameConfig>) {
    match game_config.audio_engine {
        AudioEngine::DDSP => {
            println!("🎵 Switching DDSP audio to start screen mode (placeholder)");
        }
        AudioEngine::DungeonSynth => {
            println!("🏰 Switching dungeon synth to start screen mode (placeholder)");
        }
        AudioEngine::Hybrid => {
            println!("🔮 Switching hybrid audio to start screen mode");
        }
        _ => {}
    }
}

/// Clean up all game entities when exiting playing state
fn cleanup_game_entities(
    mut commands: Commands,
    cell_query: Query<Entity, With<CellAnimation>>,
) {
    // Despawn all cell entities
    for entity in cell_query.iter() {
        commands.entity(entity).despawn();
    }
    println!("🧹 Cleaned up {} cell entities", cell_query.iter().count());
}

/// Setup audio system for start screen
fn setup_start_screen_audio(config: Res<GameConfig>) {
    match config.audio_engine {
        AudioEngine::Spatial => {
            // Spatial audio removed - use hybrid instead
            init_hybrid_dungeon_synth();
            set_hybrid_volume(config.audio_volume);
            println!("🔊 Start screen: Using Hybrid audio instead of Spatial");
        },
        AudioEngine::DDSP => {
            println!("🎵 Start screen: DDSP audio system (placeholder)");
        },
        AudioEngine::DungeonSynth => {
            println!("🏰 Start screen: Dungeon synth audio system (placeholder)");
        },
        AudioEngine::Hybrid => {
            init_hybrid_dungeon_synth();
            set_hybrid_volume(config.audio_volume);
            println!("🔮 Start screen: Hybrid dungeon synth initialized! Volume: {:.0}%", config.audio_volume * 100.0);
        }
    }
}

fn update_start_screen_audio(
    config: Res<GameConfig>,
) {
    // The hybrid audio engine automatically handles start screen mode
    // by processing empty game features (all zeros) when no real game is running
    match config.audio_engine {
        AudioEngine::Spatial => {
            // Spatial audio doesn't need continuous updates on start screen
        },
        AudioEngine::DDSP => {
            // DDSP would update here if implemented
        },
        AudioEngine::DungeonSynth => {
            // Dungeon synth would update here if implemented
        },
                 AudioEngine::Hybrid => {
             // Update hybrid audio with empty game features for ambient start screen audio
             update_hybrid_dungeon_synth([0.0; 8]); // All zeros = ambient mode
         }
    }
}

/// Animate cell birth and death transitions with proper cleanup and speed adaptation
fn animate_cells(
    mut cell_query: Query<(Entity, &mut Transform, &mut CellAnimation)>,
    time: Res<Time>,
    mut commands: Commands,
    camera_query: Query<&crate::camera::GameCamera>,
    game_stats: Res<GameStats>,
    config: Res<gameofdeath::cell_renderer::CellRenderConfig>,
) {
    // Get zoom level for animation LOD
    let zoom = camera_query
        .get_single()
        .map(|camera| camera.zoom)
        .unwrap_or(1.0);
    
    // Simplify animations when zoomed out (below 2x zoom)
    let use_simple_animation = zoom < 2.0;
    
    // Calculate speed multiplier based on game speed
    // Fast game = fast animations, slow game = slow animations
    let speed_multiplier = (2.0 / game_stats.update_interval.max(0.01) as f32).min(4.0).max(0.5);
    
    for (entity, mut transform, mut animation) in cell_query.iter_mut() {
        // Apply speed multiplier to animation timing
        let delta_scaled = time.delta().mul_f32(speed_multiplier);
        animation.timer.tick(delta_scaled);
        
        // Determine base scale in world units (same calculation as cell_renderer::adjust_cell_scale_and_overlay)
        let px_to_world = config.cell_size / 32.0;
        let base_scale_world = config.base_scale * px_to_world;

        match animation.animation_type {
            AnimationType::Birth => {
                let progress = animation.timer.fraction();
                transform.scale = Vec3::splat(base_scale_world * progress);
                if animation.timer.finished() {
                    // Restore to the base scale when animation ends
                    transform.scale = Vec3::splat(base_scale_world);
                    commands.entity(entity).remove::<CellAnimation>();
                }
            }
            AnimationType::Death => {
                let progress = 1.0 - animation.timer.fraction();
                transform.scale = Vec3::splat(base_scale_world * progress);
                if animation.timer.finished() {
                    commands.entity(entity).despawn();
                }
            }
            AnimationType::Pulse => {
                // Continuous pulsing animation
                let pulse = (animation.timer.fraction() * std::f32::consts::PI * 4.0).sin().abs();
                let scale = 0.9 + pulse * 0.3; // Pulse between 0.9 and 1.1 (smaller)
                transform.scale = Vec3::splat(scale);
                
                // Reset timer for continuous pulsing
                if animation.timer.finished() {
                    animation.timer.reset();
                }
            }
            AnimationType::Glow => {
                // Glow animation with color changes (handled via transform for now)
                let glow = (animation.timer.fraction() * std::f32::consts::PI * 2.0).sin().abs();
                let scale = 1.0 + glow * 0.13; // Subtle glow effect (smaller)
                transform.scale = Vec3::splat(scale);
                
                // Reset timer for continuous glowing
                if animation.timer.finished() {
                    animation.timer.reset();
                }
            }
        }
    }
}

/// Smooth step function for gentle easing (replaces elastic spring)
fn smooth_step(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Adjust cell density and apply slight directional overlaps towards neighbouring connections.
///
/// Cells are scaled down uniformly (controlled by `base_scale`) to make the grid visually less dense.
/// If a neighbouring cell exists along an axis, we extend the sprite slightly in that axis (`overlay_scale`).
fn adjust_cell_scale_and_overlay(
    mut cell_query: Query<(&gameofdeath::cell_renderer::CellSprite, &mut Transform, Option<&gameofdeath::CellAnimation>)>,
    grid: Res<InfiniteGrid>,
    config: Res<gameofdeath::cell_renderer::CellRenderConfig>,
    mut cache: ResMut<OverlayCache>,
) {
    // Recompute overlay cache only if the grid changed since last calculation
    if cache.version != grid.version() {
        cache.recompute(&grid);
    }

    // Pre-compute factors in world-space (texture is 32×32 px by default)
    let px_to_world = config.cell_size / 32.0;
    let base_scale_world = config.base_scale * px_to_world;
    let overlay_world = config.overlay_scale * px_to_world;

    for (cell, mut transform, anim_opt) in cell_query.iter_mut() {
        // Skip cells that currently have any running animation to avoid conflicting scale updates.
        if anim_opt.is_some() {
            continue;
        }

        let mut scale_x = base_scale_world;
        let mut scale_y = base_scale_world;

        if cache.horiz.contains(&(cell.x, cell.y)) {
            scale_x += overlay_world;
        }
        if cache.vert.contains(&(cell.x, cell.y)) {
            scale_y += overlay_world;
        }

        transform.scale.x = scale_x;
        transform.scale.y = scale_y;
        // Keep original Z scale untouched.
    }
}

/// Allow quick insertion of demo patterns with number keys 1-5 depending on active rule.
fn pattern_hotkeys(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Query<&Transform, With<GameCamera>>,
    camera_state: Res<CameraState>,
    mut grid: ResMut<InfiniteGrid>,
    game_config: Res<GameConfig>,
) {
    let pos = if let Ok(t) = camera_query.get_single() { t.translation } else { return; };
    let (cx, cy) = world_to_grid(pos.truncate(), &camera_state);

    if keyboard_input.just_pressed(KeyCode::Digit1) {
        insert_rule_pattern(1, &game_config, &mut grid, cx, cy);
    }
    if keyboard_input.just_pressed(KeyCode::Digit2) {
        insert_rule_pattern(2, &game_config, &mut grid, cx, cy);
    }
    if keyboard_input.just_pressed(KeyCode::Digit3) {
        insert_rule_pattern(3, &game_config, &mut grid, cx, cy);
    }
}

fn insert_rule_pattern(slot: u8, config: &GameConfig, grid: &mut InfiniteGrid, ox: i32, oy: i32) {
    use gameofdeath::infinite_grid::patterns as pat;
    match (config.current_rule, slot) {
        (RuleType::HighLife, 1) => grid.insert_pattern(pat::highlife_replicator(), ox, oy),
        (RuleType::Conway, 1) => grid.insert_pattern(pat::glider(), ox, oy),
        (RuleType::Conway, 2) => grid.insert_pattern(pat::blinker(), ox, oy),
        (RuleType::Conway, 3) => grid.insert_pattern(pat::block(), ox, oy),
        _ => {}
    }
}

/// Dynamically populate the HUD panel with rule-specific controls when the rule changes.
fn update_rule_controls(
    game_config: Res<GameConfig>,
    mut container_query: Query<(Entity, &Children), With<RuleControlsContainer>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !game_config.is_changed() {
        return;
    }

    let (entity, children) = match container_query.get_single_mut() {
        Ok(v) => v,
        Err(_) => return,
    };

    // Clear existing children
    for &child in children.iter() {
        commands.entity(child).despawn_recursive();
    }

    let lines = rule_specific_controls(game_config.current_rule);
    if lines.is_empty() { return; }

    let font = asset_server.load("fonts/Geo-Regular.ttf");

    commands.entity(entity).with_children(|parent| {
        parent.spawn((
            Text::new("Rule Controls:"),
            TextFont { font: font.clone(), font_size: 14.0, ..default() },
            TextColor(Color::rgb(1.0, 0.85, 0.3)),
            RuleControlText,
        ));
        for l in lines {
            parent.spawn((
                Text::new(l),
                TextFont { font: font.clone(), font_size: 14.0, ..default() },
                TextColor(Color::WHITE),
                RuleControlText,
            ));
        }
    });
}

fn rule_specific_controls(rule: RuleType) -> Vec<&'static str> {
    match rule {
        RuleType::WireWorld => vec![
            "LMB: Wire",
            "Shift+Click: Electron Head",
            "Alt+Click: Electron Tail",
            "1: Clock pattern",
        ],
        RuleType::Brian => vec![
            "LMB: Firing cell",
            "Shift+Click: Dying cell",
        ],
        RuleType::Immigration => vec![
            "LMB: Species A",
            "RMB: Species B",
        ],
        RuleType::HighLife => vec!["1: Replicator seed"],
        _ => Vec::new(),
    }
}

fn main() {
    env_logger::init();
    
    // Load configuration from file
    let config = Config::load(None);
    
    App::default()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Game of Death".into(),
                resolution: (1200.0, 800.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .init_state::<GameState>()
        .init_resource::<GameStats>()
        .init_resource::<AudioEnabled>()
        .init_resource::<AudioCache>()
        .init_resource::<OverlayCache>()
        .init_resource::<CellTexturePool>()
        .insert_resource(GameConfig {
            current_rule: RuleType::Conway,
            audio_engine: config.audio_engine,
            audio_volume: config.audio_volume,
        })
        .init_resource::<InfiniteGrid>()
        .init_resource::<SelectedRule>()
        .init_resource::<UiState>()
        .init_resource::<CameraState>()
        .init_resource::<CellRenderConfig>()
        .init_resource::<CellTextureCache>()
        .init_resource::<BrushSettings>()
        .insert_non_send_resource(IllbientGroove::new(100.0))
        .add_plugins(SynthControlPanelPlugin)
        .add_systems(Startup, (setup_kira, setup_camera, setup_ui, setup_font, setup_start_screen_audio))
        .add_systems(
            Update,
            (
                handle_start_screen_input,
                update_start_screen_ui.after(handle_start_screen_input),
                update_start_screen_audio,
            )
                .run_if(in_state(GameState::StartScreen))
        )
        .add_systems(OnEnter(GameState::StartScreen), setup_start_screen)
        .add_systems(OnExit(GameState::StartScreen), (cleanup_start_screen, on_exit_start_screen))
        .add_systems(OnEnter(GameState::Playing), (show_hud, setup_game_audio, on_enter_playing))
        .add_systems(OnExit(GameState::Playing), (hide_hud, on_exit_playing, cleanup_game_entities))
        .add_systems(
            Update,
            (
                handle_camera_controls,
                handle_game_input,
                update_simulation,
                handle_mouse_input,
                // Ensure cell animations/despawns happen after rendering logic to avoid race conditions.
                render_optimized_cells,
                animate_cell_textures.after(render_optimized_cells),
                update_cell_render_config.after(render_optimized_cells),
                animate_cells.after(render_optimized_cells),
                update_game_ui,
                update_audio_system,
                gameofdeath::ui::update_ui,
                gameofdeath::ui::toggle_hud_visibility,
                adjust_cell_scale_and_overlay,
                pattern_hotkeys,
                update_rule_controls,
            )
                .run_if(in_state(GameState::Playing))
        )
        .run();
} 