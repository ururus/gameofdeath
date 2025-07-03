# Game of Death

An interactive audiovisual playground based on Conway's Game of Life, built with the Bevy game engine. This project explores the emergent complexity of cellular automata and translates it into a dynamic, generative soundscape. It features a high-performance simulation engine, multiple rule sets, and a modular audio system that reacts in real-time to game events.

![Game of Death Screenshot](https://i.imgur.com/example.png)  <!-- Placeholder -->

## ✨ Features

### Core Simulation
- **Multiple Rule Sets**: Choose from classic rules like `Conway's Game of Life`, `HighLife`, `Seeds`, and more exotic automata like `Brian's Brain` and `Mazectric`.
- **Infinite Grid**: The simulation space is unbounded, allowing patterns to grow infinitely.
- **High Performance**: Optimized to simulate and render tens of thousands of cells smoothly, leveraging a custom rendering pipeline.
- **Interactive Start Screen**: A sleek UI for selecting the game mode before diving in.
- **Procedural Cell Rendering**: Cells have a "living" texture that pulses and animates, with different variations for birth, life, and death states. The animation speed and texture refresh rate are configurable.

### Audiovisual Experience
- **Reactive Audio Engine**: A sophisticated sound system that analyzes the simulation in real-time.
- **Game Feature Extraction**: Key metrics like `population`, `density`, `chaos`, `symmetry`, and `centroid` are extracted each frame.
- **Hybrid Dungeon Synth**: A multi-layered drone engine that shifts harmonically based on the game's state.
- **Illbient Groove Module**: A non-send resource that generates reactive drum and bass patterns (kick, hi-hat, bassline) that follow the game's emergent features.
- **Modular Synth UI**: An in-game, retractable control panel (press `P`) with synth-style knobs to control audio parameters like master volume and the mix between different sound layers.

## 🕹️ Controls

### 🚀 Start Screen
- **Arrow Keys / ‹ › Buttons**: Cycle through available game modes.
- **Enter / "START GAME" Button**: Begin the simulation with the selected rule.
- **ESC**: Quit the application.

### 🎮 In-Game
#### Simulation & Navigation
- **Spacebar**: Pause or resume the simulation.
- **`+` / `-`**: Speed up / slow down the simulation update interval.
- **`S`**: Advance the simulation by a single step (when paused).
- **`C`**: Clear the grid of all cells.
- **`R`**: Reset the entire game and return to the start screen.
- **`ESC`**: Return to the start screen without resetting the grid.

#### Camera
- **`W` `A` `S` `D`**: Pan the camera across the grid.
- **Mouse Wheel**: Zoom in and out.
- **`PageUp` / `PageDown`**: Zoom using the keyboard.
- **`Home`**: Reset camera position and zoom to default.

#### Mouse Interaction
- **Left-Click (& Drag)**: Place living cells on the grid.
- **Right-Click (& Drag)**: Erase cells from the grid.

#### UI & Audio
- **`H`**: Toggle the Heads-Up Display (HUD) which shows FPS and game stats.
- **`P`**: Toggle the modular synth control panel.
- **`,` / `.` (< / >)**: Decrease / Increase master audio volume.

## 🔧 Configuration

The game can be configured via the `oraclelife.toml` file in the root directory.

```toml
# oraclelife.toml
audio_engine = "Hybrid"  # "Hybrid", "DDSP", "DungeonSynth", "Spatial"
audio_volume = 0.7       # Initial volume (0.0 to 2.0)
```

## 🛠️ Building & Running

Ensure you have a recent Rust toolchain installed.

```bash
# Clone the repository
git clone <repository_url>
cd gameofdeath

# Run in debug mode
cargo run

# Build an optimized release version
cargo build --release
```

## 🏗️ Technical Stack

- **Engine**: [Bevy Engine](https://bevyengine.org/) (v0.15)
- **Audio Backend**: [Kira](https://kira.synthax.link/) for audio management.
- **Language**: [Rust](https://www.rust-lang.org/)

The project is structured into several modules, including a dedicated `audio` module for the synthesis engines, `cell_renderer` for the custom visuals, and `synth_ui` for the Bevy UI control panel. 