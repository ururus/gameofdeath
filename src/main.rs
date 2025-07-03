use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, DiagnosticsStore}, window::PrimaryWindow};

// Import our modules
use gameofdeath::*;
use gameofdeath::camera::{setup_camera, handle_camera_controls, GameCamera, CameraState, world_to_grid, screen_to_world};
use gameofdeath::start_screen::{GameState, SelectedRule, RuleType, setup_start_screen, handle_start_screen_input, cleanup_start_screen, update_start_screen_ui};
use gameofdeath::ui::{setup_ui, UiState};
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

/// Game configuration
#[derive(Resource)]
pub struct GameConfig {
    pub current_rule: RuleType,
    pub audio_engine: AudioEngine,
    pub audio_volume: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            current_rule: RuleType::Conway,
            audio_engine: AudioEngine::Spatial,
            audio_volume: 0.7,
        }
    }
}

fn handle_game_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_stats: ResMut<GameStats>,
    mut grid: ResMut<InfiniteGrid>,
    mut game_state: ResMut<NextState<GameState>>,
    mut game_config: ResMut<GameConfig>,
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
}

fn handle_mouse_input(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Transform, &OrthographicProjection, &GameCamera), With<GameCamera>>,
    camera_state: Res<CameraState>,
    mut grid: ResMut<InfiniteGrid>,
    _game_config: Res<GameConfig>,
) {
    // Use pressed() for continuous placement while holding down mouse button
    if mouse_button_input.pressed(MouseButton::Left) || mouse_button_input.pressed(MouseButton::Right) {
        if let (Ok(window), Ok((camera_transform, projection, _game_camera))) = (windows.get_single(), camera_query.get_single()) {
            if let Some(cursor_position) = window.cursor_position() {
                // Use the working coordinate conversion
                let window_size = Vec2::new(window.width(), window.height());
                
                let world_pos = screen_to_world(cursor_position, camera_transform, projection, window_size);
                let (grid_x, grid_y) = world_to_grid(world_pos, &camera_state);

                if mouse_button_input.pressed(MouseButton::Left) {
                    grid.set_alive(grid_x, grid_y);
                }
                
                if mouse_button_input.pressed(MouseButton::Right) {
                    grid.set_dead(grid_x, grid_y);
                }
            }
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
    selected_rule: Res<SelectedRule>,
) {
    // Apply the selected rule from start screen to game config
    game_config.current_rule = selected_rule.current;
    println!("🎯 Applied rule: {} to game", selected_rule.current.name());
    
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
        
        let progress = animation.timer.fraction();
        animation.progress = progress;
        
        match animation.animation_type {
            AnimationType::Birth => {
                if use_simple_animation {
                    // Simple instant appear when zoomed out
                    transform.scale = Vec3::splat(1.0);
                    if animation.timer.finished() {
                        // Remove animation component when done
                        commands.entity(entity).remove::<CellAnimation>();
                    }
                } else {
                    // Smooth scale-up when zoomed in
                    let scale = smooth_step(progress);
                    transform.scale = Vec3::splat(scale);
                    
                    if animation.timer.finished() {
                        transform.scale = Vec3::splat(1.0);
                        commands.entity(entity).remove::<CellAnimation>();
                    }
                }
            }
            AnimationType::Death => {
                if use_simple_animation {
                    // Simple instant disappear when zoomed out
                    commands.entity(entity).despawn();
                } else {
                    // Smooth fade-out when zoomed in
                    let scale = 1.0 - smooth_step(progress);
                    transform.scale = Vec3::splat(scale);
                    
                    if animation.timer.finished() {
                        commands.entity(entity).despawn();
                    }
                }
            }
            AnimationType::Pulse => {
                // Continuous pulsing animation
                let pulse = (progress * std::f32::consts::PI * 4.0).sin().abs();
                let scale = 0.8 + pulse * 0.4; // Pulse between 0.8 and 1.2
                transform.scale = Vec3::splat(scale);
                
                // Reset timer for continuous pulsing
                if animation.timer.finished() {
                    animation.timer.reset();
                }
            }
            AnimationType::Glow => {
                // Glow animation with color changes (handled via transform for now)
                let glow = (progress * std::f32::consts::PI * 2.0).sin().abs();
                let scale = 1.0 + glow * 0.2; // Subtle glow effect
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
                render_optimized_cells,
                animate_cell_textures,
                update_cell_render_config,
                animate_cells,
                update_game_ui,
                update_audio_system,
                gameofdeath::ui::update_ui,
                gameofdeath::ui::toggle_hud_visibility,
            )
                .run_if(in_state(GameState::Playing))
        )
        .run();
} 