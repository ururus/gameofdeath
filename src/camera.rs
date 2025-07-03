use bevy::prelude::*;

/// Camera component for handling zoom and pan
#[derive(Component)]
pub struct GameCamera {
    pub zoom: f32,
    pub target_zoom: f32,
    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            zoom: 1.0,        // Start at 1x zoom (reasonable default)
            target_zoom: 1.0, // Start at 1x zoom (reasonable default)  
            pan_speed: 500.0,
            zoom_speed: 0.1,  // Increase zoom speed for better responsiveness
            min_zoom: 0.05,   // Allow much more zoom out
            max_zoom: 20.0,   // Allow much more zoom in
        }
    }
}

/// Resource for tracking camera state
#[derive(Resource)]
pub struct CameraState {
    pub cell_size: f32,
    pub grid_offset: Vec2,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            cell_size: 20.0, // Match CellRenderConfig::default cell_size to align coordinate conversions
            grid_offset: Vec2::ZERO,
        }
    }
}

/// Setup the game camera
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d::default(),
        GameCamera::default(),
    ));
}

/// Handle camera controls (zoom and pan)
pub fn handle_camera_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut camera_query: Query<(&mut Transform, &mut OrthographicProjection, &mut GameCamera)>,
    _camera_state: Res<CameraState>,
    time: Res<Time>,
) {
    if let Ok((mut transform, mut projection, mut camera)) = camera_query.get_single_mut() {
        let dt = time.delta_secs();

        // Handle zoom with mouse wheel
        for event in mouse_wheel_events.read() {
            camera.target_zoom *= 1.0 + event.y * 0.001; // Restore original wheel zoom speed
            camera.target_zoom = camera.target_zoom.clamp(camera.min_zoom, camera.max_zoom);
        }

        // Handle zoom with keyboard
        if keyboard_input.pressed(KeyCode::PageUp) {
            camera.target_zoom *= 1.0 + 2.0 * dt; // Restore original keyboard zoom speed
            camera.target_zoom = camera.target_zoom.clamp(camera.min_zoom, camera.max_zoom);
        }
        if keyboard_input.pressed(KeyCode::PageDown) {
            camera.target_zoom *= 1.0 - 2.0 * dt; // Restore original keyboard zoom speed
            camera.target_zoom = camera.target_zoom.clamp(camera.min_zoom, camera.max_zoom);
        }

        // Smooth zoom interpolation
        camera.zoom = camera.zoom + (camera.target_zoom - camera.zoom) * 5.0 * dt;
        
        // Use orthographic projection scale instead of transform scale
        projection.scale = 1.0 / camera.zoom;

        // Handle panning with WASD only
        let mut pan_direction = Vec2::ZERO;
        
        if keyboard_input.pressed(KeyCode::KeyW) {
            pan_direction.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            pan_direction.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            pan_direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            pan_direction.x += 1.0;
        }

        // Apply panning (adjust for zoom level)
        if pan_direction != Vec2::ZERO {
            let pan_amount = pan_direction.normalize() * camera.pan_speed * dt * projection.scale;
            transform.translation += pan_amount.extend(0.0);
        }

        // Reset camera position
        if keyboard_input.just_pressed(KeyCode::Home) {
            transform.translation = Vec3::ZERO;
            camera.target_zoom = 1.0;
        }
    }
}

/// Convert screen coordinates to world coordinates
pub fn screen_to_world(
    screen_pos: Vec2,
    camera_transform: &Transform,
    projection: &OrthographicProjection,
    window_size: Vec2,
) -> Vec2 {
    // Get camera position
    let camera_pos = camera_transform.translation.truncate();
    
    // Calculate screen center
    let screen_center = window_size / 2.0;
    
    // Get offset from screen center in pixels
    let screen_offset = screen_pos - screen_center;
    
    // Flip Y axis (screen Y goes down, world Y goes up)
    let screen_offset = Vec2::new(screen_offset.x, -screen_offset.y);
    
    // Convert screen pixels to world units
    // The orthographic projection scale determines how many world units per pixel
    let world_offset = screen_offset * projection.scale;
    
    // Add camera position to get final world coordinates
    camera_pos + world_offset
}

/// Convert world coordinates to grid coordinates
pub fn world_to_grid(world_pos: Vec2, camera_state: &CameraState) -> (i32, i32) {
    // Convert world position to grid coordinates
    let grid_pos = (world_pos - camera_state.grid_offset) / camera_state.cell_size;
    
    // Use floor to get the cell the position is actually in (not nearest center)
    (grid_pos.x.floor() as i32, grid_pos.y.floor() as i32)
}

/// Convert grid coordinates to world coordinates
pub fn grid_to_world(grid_x: i32, grid_y: i32, camera_state: &CameraState) -> Vec2 {
    Vec2::new(
        grid_x as f32 * camera_state.cell_size + camera_state.grid_offset.x,
        grid_y as f32 * camera_state.cell_size + camera_state.grid_offset.y, // No Y flip needed now
    )
}

/// Convert screen coordinates to world coordinates using Bevy's built-in camera methods
pub fn screen_to_world_bevy(
    screen_pos: Vec2,
    camera_transform: &Transform,
    camera: &bevy::render::camera::Camera,
    _window_size: Vec2,
) -> Option<Vec2> {
    // Convert Transform to GlobalTransform
    let global_transform = GlobalTransform::from(*camera_transform);
    
    // Create a Ray from the camera through the screen position
    if let Ok(ray) = camera.viewport_to_world(&global_transform, screen_pos) {
        // For 2D, we want the intersection with the Z=0 plane
        // Ray equation: origin + direction * t
        // For Z=0: origin.z + direction.z * t = 0
        // So: t = -origin.z / direction.z
        if ray.direction.z != 0.0 {
            let t = -ray.origin.z / ray.direction.z;
            let world_point = ray.origin + ray.direction * t;
            Some(world_point.truncate())
        } else {
            None
        }
    } else {
        None
    }
}

pub fn screen_to_world_2d(
    screen_pos: Vec2,
    camera_transform: &Transform,
    camera: &OrthographicProjection,
    window_size: Vec2,
) -> Vec2 {
    // Convert screen coordinates to normalized device coordinates (NDC)
    // Screen (0,0) is top-left, NDC (-1,-1) is bottom-left
    let ndc = Vec2::new(
        (screen_pos.x / window_size.x) * 2.0 - 1.0,
        (1.0 - screen_pos.y / window_size.y) * 2.0 - 1.0,
    );
    
    // For orthographic projection, world size is determined by the scale
    let world_size = Vec2::new(
        window_size.x * camera.scale,
        window_size.y * camera.scale,
    );
    
    // Convert NDC to world coordinates relative to camera
    let local_pos = Vec2::new(
        ndc.x * world_size.x * 0.5,
        ndc.y * world_size.y * 0.5,
    );
    
    // Add camera position to get final world coordinates
    Vec2::new(
        camera_transform.translation.x + local_pos.x,
        camera_transform.translation.y + local_pos.y,
    )
} 