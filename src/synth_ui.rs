use bevy::prelude::*;
use bevy::prelude::{UiRect, Val};
use crate::audio::{set_hybrid_volume, set_hybrid_synthesis_mix};
use bevy::input::mouse::{MouseWheel, MouseScrollUnit};

// Resource holding current values for user-tweakable audio parameters.
#[derive(Resource, Debug)]
pub struct SynthParameters {
    pub volume: f32, // 0.0 .. 2.0 (overdrive possible)
    pub mix: f32,    // 0.0 .. 1.0
}

impl Default for SynthParameters {
    fn default() -> Self {
        Self { volume: 0.7, mix: 0.7 }
    }
}

// Marker for the whole panel root node.
#[derive(Component)]
struct SynthPanel;

// Component tagging parameter value text so we can refresh.
#[derive(Component, Copy, Clone)]
enum ParamLabel {
    Volume,
    Mix,
}

/// Graphical knob widget bound to a parameter.
#[derive(Component)]
struct Knob {
    param: ParamLabel,
    radius: f32,
}

// Action variants for buttons.
#[derive(Component, Copy, Clone)]
enum SynthButtonAction {
    VolumeUp,
    VolumeDown,
    MixUp,
    MixDown,
}

pub struct SynthControlPanelPlugin;

impl Plugin for SynthControlPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SynthParameters>()
            .add_systems(Startup, setup_synth_panel)
            .add_systems(
                Update,
                (
                    toggle_panel_visibility,
                    button_interaction_system,
                    refresh_param_labels,
                    knob_scroll_system,
                    knob_visual_system,
                    push_params_to_engine,
                ),
            );
    }
}

/// Key used to open / close the panel.
const TOGGLE_KEY: KeyCode = KeyCode::KeyP;

fn setup_synth_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    params: Res<SynthParameters>,
) {
    let font_handle = asset_server.load("fonts/Geo-Regular.ttf");
    // Root panel (hidden by default)
    let panel_bg = Color::rgb(0.1, 0.1, 0.12);

    let panel_entity = commands
        .spawn((
            Node {
                width: Val::Px(260.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                padding: UiRect::all(Val::Px(8.0)),
                ..Default::default()
            },
            BackgroundColor(panel_bg),
            Visibility::Hidden,
            SynthPanel,
        ))
        .id();

    // Title text
    commands.entity(panel_entity).with_children(|parent| {
        parent.spawn((
            Text::new("Modular Synth"),
            TextFont {
                font: font_handle.clone().into(),
                font_size: 24.0,
                ..Default::default()
            },
            TextColor(Color::rgb(1.0, 0.9, 0.3).into()),
        ));

        spawn_param_row(
            parent,
            &font_handle,
            "Master Vol",
            ParamLabel::Volume,
            params.volume,
        );
        spawn_param_row(
            parent,
            &font_handle,
            "Synth Mix",
            ParamLabel::Mix,
            params.mix,
        );

        // Spacer to make panel nicer
        parent.spawn((Node { flex_grow: 1.0, ..Default::default() },));

        // Close hint text
        parent.spawn((
            Text::new("[P] toggle"),
            TextFont { font: font_handle.clone().into(), font_size: 14.0, ..Default::default() },
            TextColor(Color::rgb(0.6,0.6,0.6).into()),
        ));
    });
}

fn spawn_param_row(
    parent: &mut ChildBuilder,
    font: &Handle<Font>,
    label: &str,
    label_component: ParamLabel,
    initial_value: f32,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(40.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            BackgroundColor(Color::rgb(0.15, 0.15, 0.17)),
        ))
        .with_children(|row| {
        // label
        row.spawn((
            Text::new(label),
            TextFont { font: font.clone().into(), font_size:16.0, ..Default::default() },
            TextColor(Color::WHITE.into()),
        ));
        // knob
        spawn_knob(row, font, label_component, initial_value);
        // value text
        row.spawn((
            Text::new(format!("{:.2}", initial_value)),
            TextFont { font: font.clone().into(), font_size:14.0, ..Default::default() },
            TextColor(Color::rgb(0.9,0.9,0.4).into()),
            label_component,
        ));
    });
}

fn spawn_knob(
    parent: &mut ChildBuilder,
    font: &Handle<Font>,
    param: ParamLabel,
    _initial_value: f32,
) {
    let radius = 14.0;
    // Outer knob circle
    parent.spawn((
        Button,
        Knob { param, radius },
        Node {
            width: Val::Px(radius * 2.0),
            height: Val::Px(radius * 2.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        BackgroundColor(Color::rgb(0.2, 0.2, 0.22)),
    )).with_children(|knob_parent| {
        // Indicator bar
        knob_parent.spawn((
            Node {
                width: Val::Px(2.0),
                height: Val::Px(radius),
                ..Default::default()
            },
            BackgroundColor(Color::WHITE),
            Transform::default(),
            GlobalTransform::default(),
        ));
    });
}

// System: toggle panel visibility with key.
fn toggle_panel_visibility(
    mut panel_query: Query<&mut Visibility, With<SynthPanel>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(TOGGLE_KEY) {
        if let Ok(mut vis) = panel_query.get_single_mut() {
            *vis = match *vis {
                Visibility::Hidden => Visibility::Visible,
                _ => Visibility::Hidden,
            };
        }
    }
}

// System: handle button press interactions.
fn button_interaction_system(
    mut interaction_query: Query<(&Interaction, &SynthButtonAction), (Changed<Interaction>, With<Button>)>,
    mut params: ResMut<SynthParameters>,
) {
    for (interaction, action) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            match action {
                SynthButtonAction::VolumeUp => {
                    params.volume = (params.volume + 0.05).min(2.0);
                    set_hybrid_volume(params.volume);
                }
                SynthButtonAction::VolumeDown => {
                    params.volume = (params.volume - 0.05).max(0.0);
                    set_hybrid_volume(params.volume);
                }
                SynthButtonAction::MixUp => {
                    params.mix = (params.mix + 0.05).min(1.0);
                    set_hybrid_synthesis_mix(params.mix);
                }
                SynthButtonAction::MixDown => {
                    params.mix = (params.mix - 0.05).max(0.0);
                    set_hybrid_synthesis_mix(params.mix);
                }
            }
        }
    }
}

// System: update displayed parameter values.
fn refresh_param_labels(
    params: Res<SynthParameters>,
    mut query: Query<(&ParamLabel, &mut Text)>,
) {
    if !params.is_changed() {
        return;
    }
    for (label, mut text) in &mut query {
        match label {
            ParamLabel::Volume => {
                *text = Text::new(format!("{:.2}", params.volume));
            }
            ParamLabel::Mix => {
                *text = Text::new(format!("{:.2}", params.mix));
            }
        }
    }
}

// --------- Knob interaction ----------

fn knob_scroll_system(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut knobs: Query<(&Interaction, &Knob)>,
    mut params: ResMut<SynthParameters>,
) {
    let mut delta = 0.0f32;
    for ev in mouse_wheel_events.read() {
        delta += match ev.unit {
            MouseScrollUnit::Line => ev.y * 0.05,
            MouseScrollUnit::Pixel => ev.y * 0.001,
        } as f32;
    }
    if delta == 0.0 { return; }

    for (interaction, knob) in &mut knobs {
        if *interaction == Interaction::Hovered {
            match knob.param {
                ParamLabel::Volume => {
                    params.volume = (params.volume + delta).clamp(0.0, 2.0);
                }
                ParamLabel::Mix => {
                    params.mix = (params.mix + delta).clamp(0.0, 1.0);
                }
            }
        }
    }
}

// Visual rotation update
fn knob_visual_system(
    knobs: Query<(&Knob, &Children)>,
    mut indicators: Query<&mut Transform>,
    params: Res<SynthParameters>,
) {
    for (knob, children) in &knobs {
        let val = match knob.param {
            ParamLabel::Volume => params.volume / 2.0, // 0..1
            ParamLabel::Mix => params.mix,             // 0..1
        };
        // Map value to angle (-135° .. +135°)
        let angle = (-135.0_f32).to_radians() + val * 270.0_f32.to_radians();
        if let Some(&child) = children.first() {
            if let Ok(mut t) = indicators.get_mut(child) {
                t.rotation = Quat::from_rotation_z(angle);
            }
        }
    }
}

// Parameter → engine sync every frame if changed.
fn push_params_to_engine(params: Res<SynthParameters>) {
    if params.is_changed() {
        set_hybrid_volume(params.volume);
        set_hybrid_synthesis_mix(params.mix);
    }
} 