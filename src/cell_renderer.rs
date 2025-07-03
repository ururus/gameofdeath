use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use crate::CellState;
use bevy::time::{Timer, TimerMode};

/// Cell rendering component to track sprites
#[derive(Component)]
pub struct CellSprite {
    pub x: i32,
    pub y: i32,
    pub cell_type: CellState,
}

/// Cell animation component for dynamic effects
#[derive(Component)]
pub struct CellAnimation {
    pub animation_type: AnimationType,
    pub timer: Timer,
    pub progress: f32,
}

/// Types of cell animations
#[derive(Debug, Clone, Copy)]
pub enum AnimationType {
    Birth,
    Death,
    Pulse,
    Glow,
}

/// Cache for cell textures to reduce recreation
#[derive(Resource)]
pub struct CellTextureCache {
    pub textures: HashMap<CellState, Handle<Image>>,
    pub simple_texture: Option<Handle<Image>>,
    pub procedural_textures: HashMap<u32, Handle<Image>>, // Cache by generation/variation
    pub is_initialized: bool,
}

impl Default for CellTextureCache {
    fn default() -> Self {
        Self {
            textures: HashMap::new(),
            simple_texture: None,
            procedural_textures: HashMap::new(),
            is_initialized: false,
        }
    }
}

/// Cell texture pool for managing multiple texture variations
#[derive(Resource)]
pub struct CellTexturePool {
    pub alive_textures: Vec<Handle<Image>>,
    pub dying_textures: Vec<Handle<Image>>,
    pub newborn_textures: Vec<Handle<Image>>,
    pub is_initialized: bool,
    pub generation_seed: u64,
    pub evolution_timer: Timer,  // Timer for texture evolution
    pub texture_update_timer: Timer,  // Timer for individual cell texture updates
    pub last_update_time: f32,  // Track time for smooth transitions
}

impl Default for CellTexturePool {
    fn default() -> Self {
        Self {
            alive_textures: Vec::new(),
            dying_textures: Vec::new(),
            newborn_textures: Vec::new(),
            is_initialized: false,
            generation_seed: 1,
            evolution_timer: Timer::from_seconds(30.0, TimerMode::Repeating), // Evolve texture sets every 30 seconds
            texture_update_timer: Timer::from_seconds(1.0 / 60.0, TimerMode::Repeating), // Update individual textures 20 times per second
            last_update_time: 0.0,
        }
    }
}

/// Configuration for cell rendering
#[derive(Resource)]
pub struct CellRenderConfig {
    pub cell_size: f32,
    pub base_color: Color,
    pub generation_colors: bool,
    pub max_visible_cells: usize,
    pub lod_enabled: bool,
    /// Global multiplier for texture animation speed (1.0 = normal)
    pub animation_speed: f32,
    /// Frames-per-second for cycling between texture variations (visual refresh rate)
    pub texture_fps: f32,
}

impl Default for CellRenderConfig {
    fn default() -> Self {
        Self {
            cell_size: 20.0,
            base_color: Color::WHITE,
            generation_colors: false,
            max_visible_cells: 10000,
            lod_enabled: true,
            animation_speed: 2.0, // 2× faster animations by default
            texture_fps: 24.0,    // swap textures ~24 FPS
        }
    }
}

/// Optimized cell rendering using procedural textures with object pooling
pub fn render_optimized_cells(
    mut commands: Commands,
    mut grid: ResMut<crate::InfiniteGrid>,
    camera_query: Query<(&Transform, &OrthographicProjection, &crate::camera::GameCamera), With<crate::camera::GameCamera>>,
    _camera_state: Res<crate::camera::CameraState>,
    existing_cells: Query<(Entity, &CellSprite, Option<&CellAnimation>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    config: Res<CellRenderConfig>,
    mut texture_cache: ResMut<CellTextureCache>,
    mut texture_pool: ResMut<CellTexturePool>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Ok((camera_transform, _projection, game_camera)) = camera_query.get_single() {
        let window = windows.single();
        
        // Initialize texture pool if needed
        initialize_texture_pool(&mut texture_pool, &mut images, &config);
        
        // Calculate visible area bounds with some padding
        let camera_pos = camera_transform.translation.truncate();
        let zoom = game_camera.zoom;
        let window_size = Vec2::new(window.width(), window.height());
        let world_size = window_size / zoom;
        let padding = world_size * 0.1; // 10% padding
        
        let min_x = ((camera_pos.x - world_size.x / 2.0 - padding.x) / config.cell_size).floor() as i32;
        let max_x = ((camera_pos.x + world_size.x / 2.0 + padding.x) / config.cell_size).ceil() as i32;
        let min_y = ((camera_pos.y - world_size.y / 2.0 - padding.y) / config.cell_size).floor() as i32;
        let max_y = ((camera_pos.y + world_size.y / 2.0 + padding.y) / config.cell_size).ceil() as i32;
        
        // Get current alive cells first
        let alive_cells = grid.get_alive_cells();
        
        // Create a set of currently existing cell entities for efficient lookup
        let mut existing_positions = std::collections::HashSet::new();
        for (entity, cell_sprite, animation) in existing_cells.iter() {
            let pos = (cell_sprite.x, cell_sprite.y);
            
            // Check if cell position is within view bounds first
            if cell_sprite.x < min_x || cell_sprite.x > max_x ||
               cell_sprite.y < min_y || cell_sprite.y > max_y {
                commands.entity(entity).despawn();
            } else {
                existing_positions.insert(pos);
                
                // Check if this position is still alive and add death animation if needed
                let still_alive = alive_cells.iter().any(|&(x, y)| x == cell_sprite.x && y == cell_sprite.y);
                if !still_alive && animation.is_none() {
                    // Only add death animation if the entity doesn't already have one
                    commands.entity(entity).insert(CellAnimation {
                        animation_type: AnimationType::Death,
                        timer: Timer::from_seconds(0.2, TimerMode::Once), // 200ms death animation
                        progress: 0.0,
                    });
                }
            }
        }
        
        // Spawn new cell entities for visible alive cells
        for &(x, y) in alive_cells {
            if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                if !existing_positions.contains(&(x, y)) {
                    let world_x = x as f32 * config.cell_size;
                    let world_y = y as f32 * config.cell_size;
                    
                    // Get procedural texture for this cell
                    let cell_texture = if let Some(texture) = get_cell_texture(
                        &texture_pool,
                        CellState::Alive,
                        None, // No animation for new cells initially
                        (x, y),
                    ) {
                        texture
                    } else {
                        // Fallback to simple texture if procedural textures aren't ready
                        if texture_cache.simple_texture.is_none() {
                            texture_cache.simple_texture = Some(create_simple_cell_texture(&mut images, 32, config.base_color));
                        }
                        texture_cache.simple_texture.as_ref().unwrap().clone()
                    };

                    commands.spawn((
                        Sprite {
                            image: cell_texture,
                            color: Color::WHITE, // Let the texture handle the color
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(world_x, world_y, 0.0))
                            .with_scale(Vec3::splat(0.1)), // Start small for birth animation
                        CellSprite {
                            x,
                            y,
                            cell_type: CellState::Alive,
                        },
                        CellAnimation {
                            animation_type: AnimationType::Birth,
                            timer: Timer::from_seconds(0.3, TimerMode::Once), // 300ms birth animation
                            progress: 0.0,
                        },
                    ));
                }
            }
        }
    }
}

/// Create a simple white cell texture
fn create_simple_cell_texture(images: &mut Assets<Image>, size: u32, color: Color) -> Handle<Image> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    
    // Create a simple white square with soft edges
        for y in 0..size {
            for x in 0..size {
            let edge_distance = std::cmp::min(
                std::cmp::min(x, size - 1 - x),
                std::cmp::min(y, size - 1 - y)
            );
            
            let alpha = if edge_distance == 0 {
                64 // Soft edges
            } else if edge_distance == 1 {
                100 // Slightly soft
            } else {
                255 // Full opacity
            };
            
            data.push((color.to_linear().red * 255.0) as u8); // R
            data.push((color.to_linear().green * 255.0) as u8); // G  
            data.push((color.to_linear().blue * 255.0) as u8); // B
            data.push(alpha); // A
        }
    }
    
    let image = Image::new(
        Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        Default::default(),
    );
    
    images.add(image)
}

/// Update cell render configuration
pub fn update_cell_render_config(
    mut config: ResMut<CellRenderConfig>,
    camera_query: Query<&crate::camera::GameCamera>,
) {
    if let Ok(camera) = camera_query.get_single() {
        // Adjust max visible cells based on zoom level
        config.max_visible_cells = if camera.zoom < 0.5 {
            2000 // Fewer cells when zoomed out
        } else if camera.zoom > 2.0 {
            15000 // More cells when zoomed in
        } else {
            10000 // Default
        };
    }
}

/// Animate cell textures and evolve them over time
pub fn animate_cell_textures(
    mut cell_query: Query<(&mut Sprite, &CellSprite, Option<&CellAnimation>)>,
    mut texture_pool: ResMut<CellTexturePool>,
    time: Res<Time>,
    config: Res<CellRenderConfig>,
    mut images: ResMut<Assets<Image>>,
) {
    // Initialize texture pool if needed
    if !texture_pool.is_initialized {
        initialize_texture_pool(&mut texture_pool, &mut images, &config);
    }

    // Update timers (unscaled)
    texture_pool.evolution_timer.tick(time.delta());
    texture_pool.texture_update_timer.tick(time.delta());

    // Advance our own time tracker with speed multiplier
    texture_pool.last_update_time += time.delta().as_secs_f32() * config.animation_speed;
    
    // Evolve texture sets when evolution timer finishes (less frequent, major changes)
    if texture_pool.evolution_timer.just_finished() {
        texture_pool.generation_seed = texture_pool.generation_seed.wrapping_add(1);
        
        println!("🧬 Evolving texture sets... generation {}", texture_pool.generation_seed);
        
        // Regenerate 2-3 random textures from each set for major variation
        if !texture_pool.alive_textures.is_empty() {
            let num_to_evolve = (texture_pool.alive_textures.len() / 3).max(1);
            for i in 0..num_to_evolve {
                let index = ((texture_pool.generation_seed + i as u64) % texture_pool.alive_textures.len() as u64) as usize;
                let new_texture = create_dynamic_cell_texture(
                    &mut images,
                    32,
                    config.base_color,
                    index as u32,
                    texture_pool.generation_seed,
                    texture_pool.last_update_time,
                );
                texture_pool.alive_textures[index] = new_texture;
            }
        }
        
        // Evolve newborn textures too
        if !texture_pool.newborn_textures.is_empty() {
            let index = (texture_pool.generation_seed % texture_pool.newborn_textures.len() as u64) as usize;
            let newborn_color = Color::linear_rgb(
                config.base_color.to_linear().red * 1.2,
                config.base_color.to_linear().green * 1.2, 
                config.base_color.to_linear().blue * 1.2,
            );
            let new_texture = create_dynamic_cell_texture(
                &mut images,
                32,
                newborn_color,
                index as u32,
                texture_pool.generation_seed.wrapping_add(100),
                texture_pool.last_update_time,
            );
            texture_pool.newborn_textures[index] = new_texture;
        }
    }

    // Fast texture updates for living appearance (20 times per second)
    let should_update_textures = texture_pool.texture_update_timer.just_finished();

    // Update all cell textures and animations
    for (mut sprite, cell_sprite, animation) in cell_query.iter_mut() {
        // Get appropriate texture based on cell state and animation
        if let Some(texture) = get_dynamic_cell_texture(
            &texture_pool, 
            cell_sprite.cell_type, 
            animation.as_deref(), 
            (cell_sprite.x, cell_sprite.y),
            texture_pool.last_update_time,
            config.texture_fps,
        ) {
            sprite.image = texture;
        }

        // Apply color effects based on animation
        if let Some(anim) = animation {
            let progress = anim.timer.fraction();
            match anim.animation_type {
                AnimationType::Birth => {
                    // Bright flash effect during birth with pulsing
                    let pulse = (texture_pool.last_update_time * 12.0).sin() * 0.3 + 1.0;
                    let intensity = (1.0 + (1.0 - progress) * 0.8) * pulse;
                    sprite.color = Color::linear_rgb(intensity, intensity, intensity);
                }
                AnimationType::Death => {
                    // Fade out effect during death with flicker
                    let flicker = (texture_pool.last_update_time * 20.0).sin() * 0.15 + 1.0;
                    sprite.color = Color::linear_rgba(flicker, flicker, flicker, progress);
                }
                AnimationType::Pulse => {
                    // Enhanced rhythmic pulsing effect
                    // This creates a pulsing effect by:
                    // 1. progress * PI * 4.0: Creates 2 complete sine wave cycles (0 to 8π) as animation progresses from 0 to 1
                    // 2. + texture_pool.last_update_time * 20.0: Adds continuous oscillation at 20 radians per second
                    // 3. .sin(): Converts to sine wave values between -1 and 1
                    // 4. * 0.5: Scales amplitude to ±0.5 range
                    // 5. + 1.0: Shifts range from [0.5, 1.5] so brightness varies from 50% to 150%
                    let pulse = (progress * std::f32::consts::PI * 4.0 + texture_pool.last_update_time * 20.0).sin() * 0.5 + 1.0;
                    sprite.color = Color::linear_rgb(pulse, pulse, pulse);
                }
                AnimationType::Glow => {
                    // Enhanced glow oscillation with time variation
                    let glow = ((progress * std::f32::consts::PI * 2.0) + (texture_pool.last_update_time * 8.0)).sin() * 0.4 + 1.0;
                    sprite.color = Color::linear_rgb(glow, glow, glow);
                }
            }
        } else {
            // Add more pronounced living pulsing to normal cells
            let living_pulse = (texture_pool.last_update_time * 3.0 + (cell_sprite.x + cell_sprite.y) as f32 * 0.2).sin() * 0.1 + 1.0;
            sprite.color = Color::linear_rgb(living_pulse, living_pulse, living_pulse);
        }
    }
}

/// Clean up despawned cells for memory management
pub fn animate_cells(
    mut commands: Commands,
    mut cell_query: Query<(Entity, &mut Transform, Option<&mut CellAnimation>), With<CellSprite>>,
    time: Res<Time>,
) {
    for (entity, mut transform, animation) in cell_query.iter_mut() {
        if let Some(mut anim) = animation {
            anim.timer.tick(time.delta());
            
            match anim.animation_type {
                AnimationType::Birth => {
                    let progress = anim.timer.fraction();
                    transform.scale = Vec3::splat(progress);
                    if anim.timer.finished() {
                        commands.entity(entity).remove::<CellAnimation>();
                    }
                }
                AnimationType::Death => {
                    let progress = 1.0 - anim.timer.fraction();
                    transform.scale = Vec3::splat(progress);
                    if anim.timer.finished() {
                        commands.entity(entity).despawn();
                    }
                }
                _ => {
                    // Remove finished animations
                    if anim.timer.finished() {
                        commands.entity(entity).remove::<CellAnimation>();
                    }
                }
            }
        }
    }
}

/// Create a procedural organic cell texture with variation
fn create_procedural_cell_texture(
        images: &mut Assets<Image>,
    size: u32, 
    base_color: Color,
    variation: u32,
    generation: u64,
    ) -> Handle<Image> {
    let mut data = Vec::with_capacity((size * size * 2) as usize);
    
    // Create procedural noise parameters based on variation and generation
    let seed = (variation as u64).wrapping_mul(generation).wrapping_add(42);
    let noise_scale = 0.1 + (variation % 5) as f32 * 0.02;
    let pulse_frequency = 2.0 + (variation % 3) as f32;
    let organic_factor = 0.3 + (variation % 7) as f32 * 0.1;
    
    let center_x = size as f32 / 2.0;
    let center_y = size as f32 / 2.0;
    let max_radius = (size as f32 / 2.0) * 0.7;
        
        for y in 0..size {
            for x in 0..size {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt() * 0.7;
            
            // Create organic edge with noise
            let angle = dy.atan2(dx);
            let noise_x = x as f32 * noise_scale + seed as f32 * 0.1;
            let noise_y = y as f32 * noise_scale + seed as f32 * 0.1;
            
            // Simple pseudo-noise function
            let noise = ((noise_x.sin() * noise_y.cos() * 1000.0) % 1.0).abs();
            let organic_radius = max_radius * (1.0 + organic_factor * (noise - 0.5));
            
            // Create pulsing effect
            let pulse = (angle * pulse_frequency).sin() * 0.1 + 1.0;
            let effective_radius = organic_radius * pulse;
            
            // Calculate alpha with soft edges
            let alpha = if distance <= effective_radius {
                let edge_softness = effective_radius * 0.2;
                if distance >= effective_radius - edge_softness {
                    let fade = 1.0 - (distance - (effective_radius - edge_softness)) / edge_softness;
                    (fade * 255.0) as u8
                } else {
                    // Inner glow effect
                    let inner_glow = 1.0 - (distance / effective_radius).powf(0.5);
                    ((0.8 + inner_glow * 0.2) * 255.0) as u8
                }
                } else {
                    0
                };
                
            // Add color variation based on distance from center
            let color_variation = 1.0 - (distance / effective_radius).min(1.0) * 0.3;
            let base_linear = base_color.to_linear();
            
            data.push((base_linear.red * color_variation * 255.0) as u8);
            data.push((base_linear.green * color_variation * 255.0) as u8);
            data.push((base_linear.blue * color_variation * 255.0) as u8);
            data.push(alpha);
            }
        }
        
        let image = Image::new(
            Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
        data,
            TextureFormat::Rgba8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
        );
        
        images.add(image)
    }
    
/// Create multiple texture variations for a cell state
fn create_texture_variations(
        images: &mut Assets<Image>,
    base_color: Color,
    variation_count: usize,
    generation: u64,
) -> Vec<Handle<Image>> {
    let mut textures = Vec::with_capacity(variation_count);
    
    for i in 0..variation_count {
        let texture = create_dynamic_cell_texture(
            images, 
            64, // Standard cell texture size
            base_color, 
            i as u32,
            generation,
            0.0, // Initial time
        );
        textures.push(texture);
    }
    
    textures
}

/// Initialize the texture pool with procedural textures
fn initialize_texture_pool(
    texture_pool: &mut CellTexturePool,
    images: &mut Assets<Image>,
    config: &CellRenderConfig,
) {
    if texture_pool.is_initialized {
        return;
    }
    
    // Create variations for alive cells (bright, pulsing, organic)
    texture_pool.alive_textures = create_texture_variations(
        images,
        config.base_color,
        8, // 8 variations for alive cells
        texture_pool.generation_seed,
    );
    
    // Create variations for newborn cells (brighter, more energetic)
    let newborn_color = Color::linear_rgb(
        config.base_color.to_linear().red * 1.2,
        config.base_color.to_linear().green * 1.2, 
        config.base_color.to_linear().blue * 1.2,
    );
    texture_pool.newborn_textures = create_texture_variations(
        images,
        newborn_color,
        4, // 4 variations for newborn cells
        texture_pool.generation_seed.wrapping_add(100),
    );
    
    // Create variations for dying cells (dimmer, fading)
    let dying_color = Color::linear_rgb(
        config.base_color.to_linear().red * 0.6,
        config.base_color.to_linear().green * 0.6,
        config.base_color.to_linear().blue * 0.6,
    );
    texture_pool.dying_textures = create_texture_variations(
        images,
        dying_color,
        4, // 4 variations for dying cells
        texture_pool.generation_seed.wrapping_add(200),
    );
    
    texture_pool.is_initialized = true;
    println!("🎨 Procedural cell textures initialized! {} alive, {} newborn, {} dying variants", 
             texture_pool.alive_textures.len(),
             texture_pool.newborn_textures.len(),
             texture_pool.dying_textures.len());
}

/// Get a texture for a cell based on its state and variation
fn get_cell_texture(
    texture_pool: &CellTexturePool,
    _cell_state: CellState,
    animation: Option<&CellAnimation>,
    position: (i32, i32),
) -> Option<Handle<Image>> {
    if !texture_pool.is_initialized {
        return None;
    }
    
    // Choose texture set based on animation state
    let texture_set = if let Some(anim) = animation {
        match anim.animation_type {
            AnimationType::Birth => &texture_pool.newborn_textures,
            AnimationType::Death => &texture_pool.dying_textures,
            _ => &texture_pool.alive_textures,
        }
                } else {
        &texture_pool.alive_textures
    };
    
    if texture_set.is_empty() {
        return None;
    }
    
    // Use position to deterministically select texture variation
    let variation_index = ((position.0.abs() + position.1.abs()) as usize) % texture_set.len();
    Some(texture_set[variation_index].clone())
}

/// Create a dynamic cell texture with living appearance
fn create_dynamic_cell_texture(
    images: &mut Assets<Image>,
    size: u32,
    base_color: Color,
    variation: u32,
    generation: u64,
    _last_update_time: f32,
) -> Handle<Image> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    
    // Create procedural noise parameters based on variation and generation
    let seed = (variation as u64).wrapping_mul(generation).wrapping_add(42);
    let noise_scale = 0.1 + (variation % 5) as f32 * 0.02;
    let pulse_frequency = 2.0 + (variation % 3) as f32;
    let organic_factor = 0.3 + (variation % 7) as f32 * 0.1;
    
    let center_x = size as f32 / 2.0;
    let center_y = size as f32 / 2.0;
    let max_radius = (size as f32 / 2.0) * 0.9;
    
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            // Create organic edge with noise
            let angle = dy.atan2(dx);
            let noise_x = x as f32 * noise_scale + seed as f32 * 0.1;
            let noise_y = y as f32 * noise_scale + seed as f32 * 0.1;
            
            // Simple pseudo-noise function
            let noise = ((noise_x.sin() * noise_y.cos() * 1000.0) % 1.0).abs();
            let organic_radius = max_radius * (1.0 + organic_factor * (noise - 0.5));
            
            // Create pulsing effect
            let pulse = (angle * pulse_frequency).sin() * 0.1 + 1.0;
            let effective_radius = organic_radius * pulse;
            
            // Calculate alpha with soft edges
            let alpha = if distance <= effective_radius {
                let edge_softness = effective_radius * 0.2;
                if distance >= effective_radius - edge_softness {
                    let fade = 1.0 - (distance - (effective_radius - edge_softness)) / edge_softness;
                    (fade * 255.0) as u8
    } else {
                    // Inner glow effect
                    let inner_glow = 1.0 - (distance / effective_radius).powf(0.5);
                    ((0.8 + inner_glow * 0.2) * 255.0) as u8
                }
    } else { 
                0
            };
            
            // Add color variation based on distance from center
            let color_variation = 1.0 - (distance / effective_radius).min(1.0) * 0.3;
            let base_linear = base_color.to_linear();
            
            data.push((base_linear.red * color_variation * 255.0) as u8);
            data.push((base_linear.green * color_variation * 255.0) as u8);
            data.push((base_linear.blue * color_variation * 255.0) as u8);
            data.push(alpha);
        }
    }
    
    let image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );
    
    images.add(image)
}

/// Get a dynamic texture for a cell based on its state and variation
fn get_dynamic_cell_texture(
    texture_pool: &CellTexturePool,
    _cell_state: CellState,
    animation: Option<&CellAnimation>,
    position: (i32, i32),
    last_update_time: f32,
    texture_fps: f32,
) -> Option<Handle<Image>> {
    if !texture_pool.is_initialized {
        return None;
    }
    
    // Choose texture set based on animation state
    let texture_set = if let Some(anim) = animation {
        match anim.animation_type {
            AnimationType::Birth => &texture_pool.newborn_textures,
            AnimationType::Death => &texture_pool.dying_textures,
            _ => &texture_pool.alive_textures,
        }
    } else {
        &texture_pool.alive_textures
    };
    
    if texture_set.is_empty() {
        return None;
    }
    
    // Create more dynamic texture variation based on time and position
    let base_index = (position.0.abs() + position.1.abs()) as usize % texture_set.len();
    
    // Increase time-based texture cycling for more living appearance
    let time_offset = ((last_update_time * texture_fps) as usize) % texture_set.len();
    let variation_index = (base_index + time_offset) % texture_set.len();
    
    Some(texture_set[variation_index].clone())
} 