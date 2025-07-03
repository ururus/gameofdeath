use bevy::prelude::*;
use kira::manager::{AudioManager, AudioManagerSettings};

/// Bevy resource that stores the global Kira `AudioManager`.
/// Other systems can fetch this via `Res<KiraManager>` to play sounds, create sub-mixes, etc.
#[derive(Resource)]
pub struct KiraManager(pub AudioManager);

/// Initialise the Kira audio backend and store it as a Bevy resource.
/// Call this in a Startup schedule once at app launch.
pub fn setup_kira(mut commands: Commands) {
    match AudioManager::new(AudioManagerSettings::default()) {
        Ok(mut manager) => {
            println!("🎧 Kira audio engine initialised");
            // Future: spawn ambient pad once custom generator is implemented.
            commands.insert_resource(KiraManager(manager));
        }
        Err(err) => {
            eprintln!("❌ Failed to initialise Kira audio engine: {err}");
        }
    }
}

// (placeholder for future ambient generator implementation) 