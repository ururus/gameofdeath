use bevy::prelude::*;
// Removed unused import

// UI Components
#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct FpsText;

#[derive(Component)]
pub struct CellCountText;

#[derive(Component)]
pub struct RuleText;

#[derive(Component)]
pub struct StatusText;

#[derive(Component)]
pub struct VolumeText;

#[derive(Component)]
pub struct ZoomText;

#[derive(Component)]
pub struct HudContainer;

// UI marker component
#[derive(Component)]
pub struct UiRoot;

// UI Resources
#[derive(Resource)]
pub struct UiState {
    pub hud_visible: bool,
    pub fps: f64,
    pub generation: u64,
    pub population: usize,
    pub is_running: bool,
    pub current_rule: String,
    pub update_interval: f64,
    pub grid_info: String,
    pub audio_volume: f32,
    pub zoom_level: f32,
    pub fps_update_timer: f64, // Timer for FPS updates (every 2 seconds)
    pub last_fps_update: f64,  // Track when FPS was last updated
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            hud_visible: true,
            fps: 0.0,
            generation: 0,
            population: 0,
            is_running: false,
            current_rule: "Conway".to_string(),
            update_interval: 0.1,
            grid_info: "Empty grid".to_string(),
            audio_volume: 0.7,
            zoom_level: 1.0,
            fps_update_timer: 0.0,
            last_fps_update: 0.0,
        }
    }
}

pub fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load the custom font
    let custom_font = asset_server.load("fonts/Geo-Regular.ttf");
    
    // Create UI root (initially hidden)
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        },
        Visibility::Hidden, // Initially hidden
        HudContainer,
        UiRoot,
    ))
    .with_children(|parent| {
        // Top-left HUD panel
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            // FPS
            parent.spawn((
                Text::new("FPS: 60"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                FpsText,
            ));

            // Cell count
            parent.spawn((
                Text::new("Cells: 0"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                CellCountText,
            ));

            // Rule
            parent.spawn((
                Text::new("Rule: Conway"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                RuleText,
            ));

            // Status
            parent.spawn((
                Text::new("Status: Paused"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                StatusText,
            ));

            // Volume
            parent.spawn((
                Text::new("Volume: 70%"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.2)), // Gold color for volume
                VolumeText,
            ));

            // Zoom level
            parent.spawn((
                Text::new("Zoom: 1.0x"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 1.0)), // Light blue color for zoom
                ZoomText,
            ));
        });

        // Bottom-right help panel
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                bottom: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ))
        .with_children(|parent| {
            let controls = [
                "Space: Pause/Resume",
                "R: Back to Menu",
                "C: Clear Grid",
                "+/-: Speed Control",
                "</>/: Volume Control",
                "WASD: Pan Camera",
                "Mouse Wheel: Zoom",
                "LMB: Toggle Cells",
                "H: Toggle HUD",
                "Home: Reset Camera",
                "",
                "🎨 Visual Controls:",
                "V: Toggle Color Variation",
                "G: Toggle Generation Colors", 
                "[/]: Noise Density",
            ];

            for control in controls.iter() {
                parent.spawn((
                    Text::new(*control),
                    TextFont {
                        font: custom_font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                ));
            }
        });
    });
}

pub fn update_ui(
    mut fps_query: Query<&mut Text, (With<FpsText>, Without<CellCountText>, Without<RuleText>, Without<StatusText>, Without<VolumeText>, Without<ZoomText>)>,
    mut cell_count_query: Query<&mut Text, (With<CellCountText>, Without<FpsText>, Without<RuleText>, Without<StatusText>, Without<VolumeText>, Without<ZoomText>)>,
    mut rule_query: Query<&mut Text, (With<RuleText>, Without<FpsText>, Without<CellCountText>, Without<StatusText>, Without<VolumeText>, Without<ZoomText>)>,
    mut status_query: Query<&mut Text, (With<StatusText>, Without<FpsText>, Without<CellCountText>, Without<RuleText>, Without<VolumeText>, Without<ZoomText>)>,
    mut volume_query: Query<&mut Text, (With<VolumeText>, Without<FpsText>, Without<CellCountText>, Without<RuleText>, Without<StatusText>, Without<ZoomText>)>,
    mut zoom_query: Query<&mut Text, (With<ZoomText>, Without<FpsText>, Without<CellCountText>, Without<RuleText>, Without<StatusText>, Without<VolumeText>)>,
    mut ui_state: ResMut<UiState>,
    time: Res<Time>,
) {
    // Update FPS timer
    ui_state.fps_update_timer += time.delta_secs_f64();
    
    // Update FPS only every 2 seconds
    if ui_state.fps_update_timer >= 2.0 {
        if let Ok(mut text) = fps_query.get_single_mut() {
            **text = format!("FPS: {:.0}", ui_state.fps);
        }
        ui_state.fps_update_timer = 0.0; // Reset timer
        ui_state.last_fps_update = time.elapsed_secs_f64();
    }

    // Update cell count
    if let Ok(mut text) = cell_count_query.get_single_mut() {
        **text = format!("Cells: {}", ui_state.population);
    }

    // Update rule
    if let Ok(mut text) = rule_query.get_single_mut() {
        **text = format!("Rule: {}", ui_state.current_rule);
    }

    // Update status
    if let Ok(mut text) = status_query.get_single_mut() {
        let status = if ui_state.is_running { "Running" } else { "Paused" };
        **text = format!("Gen: {} | {} ({:.2}s)", ui_state.generation, status, ui_state.update_interval);
    }

    // Update volume
    if let Ok(mut text) = volume_query.get_single_mut() {
        let volume_percent = (ui_state.audio_volume * 100.0) as u32;
        if ui_state.audio_volume > 1.0 {
            **text = format!("🔊🔥 Volume: {}% OVERDRIVE!", volume_percent);
        } else {
            **text = format!("🔊 Volume: {}%", volume_percent);
        }
    }

    // Update zoom level
    if let Ok(mut text) = zoom_query.get_single_mut() {
        **text = format!("🔍 Zoom: {:.1}x", ui_state.zoom_level);
    }
}

pub fn toggle_hud_visibility(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut hud_query: Query<&mut Visibility, With<HudContainer>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyH) {
        ui_state.hud_visible = !ui_state.hud_visible;
        
        for mut visibility in hud_query.iter_mut() {
            *visibility = if ui_state.hud_visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
} 