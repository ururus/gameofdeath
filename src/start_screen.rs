use bevy::prelude::*;

/// Marker component for start screen entities
#[derive(Component)]
pub struct StartScreenEntity;

/// Resource to track selected rule
#[derive(Resource)]
pub struct SelectedRule {
    pub current: RuleType,
    pub index: usize,
}

/// Available rule types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleType {
    Conway,
    HighLife,
    Seeds,
    Brian,        // Brian's Brain - 3-state automaton
    WireWorld,    // Wireworld - 4-state for digital circuits
    Immigration,  // Immigration - 2 competing species
    Mazectric,    // Mazectric - Creates maze-like structures
    Coral,        // Coral - Growth pattern automaton
    Gnarl,        // Gnarl - Chaotic growth
    Replicator,   // Replicator - Self-replicating patterns
}

impl RuleType {
    pub fn all() -> Vec<RuleType> {
        vec![
            RuleType::Conway, 
            RuleType::HighLife, 
            RuleType::Seeds,
            RuleType::Brian,
            RuleType::WireWorld,
            RuleType::Immigration,
            RuleType::Mazectric,
            RuleType::Coral,
            RuleType::Gnarl,
            RuleType::Replicator,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            RuleType::Conway => "Conway's Game of Life",
            RuleType::HighLife => "HighLife",
            RuleType::Seeds => "Seeds",
            RuleType::Brian => "Brian's Brain",
            RuleType::WireWorld => "WireWorld",
            RuleType::Immigration => "Immigration",
            RuleType::Mazectric => "Mazectric",
            RuleType::Coral => "Coral",
            RuleType::Gnarl => "Gnarl",
            RuleType::Replicator => "Replicator",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RuleType::Conway => "Classic B3/S23 - Born with 3, survives with 2-3",
            RuleType::HighLife => "B36/S23 - Conway plus replication at 6 neighbors",
            RuleType::Seeds => "B2/S0 - Every cell dies, born with exactly 2 neighbors",
            RuleType::Brian => "3-state: Ready → Firing → Refractory → Ready",
            RuleType::WireWorld => "4-state digital circuit simulation",
            RuleType::Immigration => "B3/S23 with 2 competing species",
            RuleType::Mazectric => "B3/S1234 - Creates intricate maze patterns",
            RuleType::Coral => "B3/S45678 - Coral-like growth structures",
            RuleType::Gnarl => "B1/S1 - Chaotic explosive growth",
            RuleType::Replicator => "B1357/S1357 - Perfect self-replication",
        }
    }
}

impl Default for SelectedRule {
    fn default() -> Self {
        Self {
            current: RuleType::Conway,
            index: 0,
        }
    }
}

/// Game state enum
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    StartScreen,
    Playing,
}

/// Marker components for UI elements
#[derive(Component)]
pub struct RuleNameText;

#[derive(Component)]
pub struct RuleDescriptionText;

#[derive(Component)]
pub struct LeftArrowButton;

#[derive(Component)]
pub struct RightArrowButton;

#[derive(Component)]
pub struct StartGameButton;

/// Setup the start screen UI
pub fn setup_start_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("Setting up start screen...");
    
    // Load the custom font
    let custom_font = asset_server.load("fonts/Geo-Regular.ttf");
    
    // Main container with dark background
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.0, 0.02)), // Very dark red tint
        StartScreenEntity,
    )).with_children(|parent| {
        // Game title
        parent.spawn((
            Text::new("GAME OF DEATH"),
            TextFont {
                font: custom_font.clone(),
                font_size: 72.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.1, 0.1)), // Blood red
            Node {
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
        ));
        
        // Subtitle
        parent.spawn((
            Text::new("a game of death"),
            TextFont {
                font: custom_font.clone(),
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(0.4, 0.0, 0.0)), // Dark red
            Node {
                margin: UiRect::bottom(Val::Px(50.0)),
                ..default()
            },
        ));
        
        // Rule selection container
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(30.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            BorderColor(Color::srgb(0.3, 0.0, 0.0)),
        )).with_children(|parent| {
            // Rule selection title
            parent.spawn((
                Text::new("SELECT GAME MODE"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.0, 0.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(30.0)),
                    ..default()
                },
            ));

            // Game mode selector (single line with left/right buttons)
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    margin: UiRect::bottom(Val::Px(20.0)),
                    width: Val::Px(600.0),
                    ..default()
                },
            )).with_children(|parent| {
                    // Left arrow button
                parent.spawn((
                    Button,
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        margin: UiRect::right(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.9)),
                    BorderColor(Color::srgb(0.2, 0.0, 0.0)),
                    LeftArrowButton,
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new("<"),
                        TextFont {
                            font: custom_font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.2, 0.2)),
                    ));
                });

                // Rule name and description container
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        width: Val::Px(460.0),
                        ..default()
                    },
                )).with_children(|parent| {
                    // Rule name
                    parent.spawn((
                        Text::new("Conway's Game of Life"),
                        TextFont {
                            font: custom_font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.2, 0.2)),
                        Node {
                            margin: UiRect::bottom(Val::Px(8.0)),
                            ..default()
                        },
                        RuleNameText,
                    ));
                    
                    // Rule description
                    parent.spawn((
                        Text::new("Classic B3/S23 - Born with 3, survives with 2-3"),
                        TextFont {
                            font: custom_font.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.5, 0.1, 0.1)),
                        RuleDescriptionText,
                    ));
                });

                // Right arrow button
                parent.spawn((
                    Button,
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(50.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        margin: UiRect::left(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.9)),
                    BorderColor(Color::srgb(0.2, 0.0, 0.0)),
                    RightArrowButton,
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new(">"),
                        TextFont {
                            font: custom_font.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.2, 0.2)),
                    ));
                });
            });

            // Start game button
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(60.0),
                    margin: UiRect::top(Val::Px(20.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.0, 0.0, 0.9)),
                BorderColor(Color::srgb(0.4, 0.0, 0.0)),
                StartGameButton,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("START GAME"),
                    TextFont {
                        font: custom_font.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.3, 0.3)),
                ));
            });
        });

        // Instructions at bottom
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-150.0)), // Center horizontally
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        )).with_children(|parent| {
            parent.spawn((
                Text::new("Arrow keys or buttons to change game mode"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.1, 0.1)),
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("ENTER or START GAME button to begin"),
                TextFont {
                    font: custom_font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.3, 0.05, 0.05)),
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("ESC to quit"),
                TextFont {
                    font: custom_font,
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.3, 0.05, 0.05)),
            ));
        });
    });
}

/// Handle start screen input and button interactions
pub fn handle_start_screen_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut selected_rule: ResMut<SelectedRule>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit_events: EventWriter<AppExit>,
    mut button_interaction_query: Query<(
        &Interaction,
        &mut BackgroundColor,
        Option<&LeftArrowButton>,
        Option<&RightArrowButton>,
        Option<&StartGameButton>,
    ), Changed<Interaction>>,
) {
    let rules = RuleType::all();

    // Handle button interactions
    let mut left_clicked = false;
    let mut right_clicked = false;
    let mut start_clicked = false;

    for (interaction, mut color, left_btn, right_btn, start_btn) in &mut button_interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.4, 0.05, 0.05));
                
                if left_btn.is_some() {
                    left_clicked = true;
                } else if right_btn.is_some() {
                    right_clicked = true;
                } else if start_btn.is_some() {
                    start_clicked = true;
                }
            }
            Interaction::Hovered => {
                if start_btn.is_some() {
                    *color = BackgroundColor(Color::srgb(0.3, 0.03, 0.03));
                } else {
                    *color = BackgroundColor(Color::srgb(0.2, 0.02, 0.02));
                }
            }
            Interaction::None => {
                if start_btn.is_some() {
                    *color = BackgroundColor(Color::srgba(0.2, 0.0, 0.0, 0.9));
                } else {
                    *color = BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.9));
                }
            }
        }
    }

    // Handle rule navigation
    if keyboard_input.just_pressed(KeyCode::ArrowLeft) || left_clicked {
        if selected_rule.index > 0 {
            selected_rule.index -= 1;
        } else {
            selected_rule.index = rules.len() - 1;
        }
        selected_rule.current = rules[selected_rule.index];
        println!("Selected rule: {:?}", selected_rule.current);
    }

    if keyboard_input.just_pressed(KeyCode::ArrowRight) || right_clicked {
        selected_rule.index = (selected_rule.index + 1) % rules.len();
        selected_rule.current = rules[selected_rule.index];
        println!("Selected rule: {:?}", selected_rule.current);
    }

    // Start game
    if keyboard_input.just_pressed(KeyCode::Enter) || start_clicked {
        println!("Starting game with rule: {:?}", selected_rule.current);
        next_state.set(GameState::Playing);
    }

    // Quit game
    if keyboard_input.just_pressed(KeyCode::Escape) {
        println!("Quitting game...");
        app_exit_events.send(AppExit::Success);
    }
}

/// Update start screen UI text when rule selection changes
pub fn update_start_screen_ui(
    selected_rule: Res<SelectedRule>,
    mut rule_name_query: Query<&mut Text, (With<RuleNameText>, Without<RuleDescriptionText>)>,
    mut rule_desc_query: Query<&mut Text, (With<RuleDescriptionText>, Without<RuleNameText>)>,
) {
    if selected_rule.is_changed() {
        // Update rule name
        if let Ok(mut text) = rule_name_query.get_single_mut() {
            **text = selected_rule.current.name().to_string();
        }
        
        // Update rule description
        if let Ok(mut text) = rule_desc_query.get_single_mut() {
            **text = selected_rule.current.description().to_string();
        }
    }
}

/// Cleanup start screen
pub fn cleanup_start_screen(
    mut commands: Commands,
    query: Query<Entity, With<StartScreenEntity>>,
) {
    println!("Cleaning up start screen...");
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
} 