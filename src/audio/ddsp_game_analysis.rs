use crate::infinite_grid::InfiniteGrid;
use crate::camera::CameraState;
use super::ddsp_engine::GameStateFeatures;
use std::collections::HashSet;

/// Analyze the game grid and extract neural network features
pub struct GameStateAnalyzer {
    previous_population: usize,
    previous_generation: u64,
    previous_features: Option<GameStateFeatures>, // Track previous features for better change detection
    _birth_count: usize,
    _death_count: usize,
    activity_history: Vec<f32>, // Track recent activity for smoothing
}

impl Default for GameStateAnalyzer {
    fn default() -> Self {
        Self {
            previous_population: 0,
            previous_generation: 0,
            previous_features: None,
            _birth_count: 0,
            _death_count: 0,
            activity_history: Vec::with_capacity(10), // Keep last 10 frames
        }
    }
}

impl GameStateAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Extract features from current game state
    pub fn extract_features(
        &mut self,
        grid: &InfiniteGrid,
        camera_state: &CameraState,
        generation: u64,
    ) -> GameStateFeatures {
        let alive_cells = grid.get_alive_cells_snapshot();
        let current_population = alive_cells.len();
        
        // Use a reasonable viewport size (we don't have access to actual camera viewport)
        // This is sufficient for feature extraction
        let viewport_width = 100.0; // cells
        let viewport_height = 75.0; // cells
        let camera_x = camera_state.grid_offset.x / camera_state.cell_size;
        let camera_y = camera_state.grid_offset.y / camera_state.cell_size;
        
        let min_x = (camera_x - viewport_width / 2.0) as i32;
        let max_x = (camera_x + viewport_width / 2.0) as i32;
        let min_y = (camera_y - viewport_height / 2.0) as i32;
        let max_y = (camera_y + viewport_height / 2.0) as i32;
        
        // Count cells in viewport
        let viewport_cells: Vec<_> = alive_cells.iter()
            .filter(|&&(x, y)| x >= min_x && x <= max_x && y >= min_y && y <= max_y)
            .collect();
        
        let viewport_population = viewport_cells.len();
        let viewport_area = (viewport_width * viewport_height) as usize;
        
        // Calculate activity (births/deaths since last frame) - more sensitive
        let activity = if generation > self.previous_generation {
            let pop_change = current_population as i32 - self.previous_population as i32;
            let raw_activity = (pop_change.abs() as f32) / (current_population.max(1) as f32);
            
            // Add to activity history for smoothing
            self.activity_history.push(raw_activity);
            if self.activity_history.len() > 10 {
                self.activity_history.remove(0);
            }
            
            // Use smoothed activity but be more sensitive to changes
            let smoothed = if self.activity_history.len() > 1 {
                let avg = self.activity_history.iter().sum::<f32>() / self.activity_history.len() as f32;
                // Boost small changes to make them more audible
                if avg < 0.01 { avg * 3.0 } else { avg }
            } else {
                raw_activity
            };
            
            smoothed.min(1.0)
        } else {
            // Gradually decay activity when no generation change
            if let Some(last_activity) = self.activity_history.last() {
                (last_activity * 0.8).max(0.0)
            } else {
                0.0
            }
        };
        
        // Analyze clusters in viewport
        let (cluster_count, avg_cluster_size) = self.analyze_clusters(&viewport_cells);
        
        // Calculate symmetry (measure horizontal/vertical symmetry in viewport)
        let symmetry = self.calculate_symmetry(&viewport_cells, camera_x, camera_y);
        
        // Calculate chaos (randomness vs organized patterns)
        let chaos = self.calculate_chaos(&viewport_cells);
        
        // Calculate centroid of alive cells in viewport (normalized -1..1 relative to viewport center)
        let (centroid_x, centroid_y) = if !viewport_cells.is_empty() {
            let sum_x: i32 = viewport_cells.iter().map(|&&(x,_y)| x).sum();
            let sum_y: i32 = viewport_cells.iter().map(|&&(_x,y)| y).sum();
            let count = viewport_cells.len() as f32;
            let mean_x = sum_x as f32 / count;
            let mean_y = sum_y as f32 / count;
            (
                ((mean_x - camera_x).clamp(-viewport_width, viewport_width) / (viewport_width/2.0)).clamp(-1.0,1.0),
                ((mean_y - camera_y).clamp(-viewport_height, viewport_height) / (viewport_height/2.0)).clamp(-1.0,1.0),
            )
        } else { (0.0, 0.0) };
        
        // Update state for next frame
        self.previous_population = current_population;
        self.previous_generation = generation;
        
        let features = GameStateFeatures {
            population: (current_population as f32 / 1000.0).min(1.0), // Normalize to ~[0,1]
            density: (viewport_population as f32) / (viewport_area as f32).max(1.0),
            activity: activity.min(1.0),
            cluster_count: (cluster_count as f32 / 10.0).min(1.0), // Normalize
            avg_cluster_size: (avg_cluster_size / 20.0).min(1.0), // Normalize
            symmetry,
            chaos,
            generation: ((generation % 1000) as f32) / 1000.0, // Normalize
            centroid_x,
            centroid_y,
        };
        
                 self.previous_features = Some(features.clone());
         features
    }
    
    /// Analyze clusters using flood-fill algorithm
    fn analyze_clusters(&self, cells: &[&(i32, i32)]) -> (usize, f32) {
        if cells.is_empty() {
            return (0, 0.0);
        }
        
        let cell_set: HashSet<(i32, i32)> = cells.iter().map(|&&pos| pos).collect();
        let mut visited = HashSet::new();
        let mut clusters = Vec::new();
        
        for &&(x, y) in cells {
            if visited.contains(&(x, y)) {
                continue;
            }
            
            // Start flood fill for new cluster
            let mut cluster_size = 0;
            let mut stack = vec![(x, y)];
            
            while let Some((cx, cy)) = stack.pop() {
                if visited.contains(&(cx, cy)) {
                    continue;
                }
                
                visited.insert((cx, cy));
                cluster_size += 1;
                
                // Check 8-connected neighbors
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        
                        let nx = cx + dx;
                        let ny = cy + dy;
                        
                        if cell_set.contains(&(nx, ny)) && !visited.contains(&(nx, ny)) {
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            
            clusters.push(cluster_size);
        }
        
        let cluster_count = clusters.len();
        let avg_cluster_size = if cluster_count > 0 {
            clusters.iter().sum::<usize>() as f32 / cluster_count as f32
        } else {
            0.0
        };
        
        (cluster_count, avg_cluster_size)
    }
    
    /// Calculate symmetry measure (0 = no symmetry, 1 = perfect symmetry)
    fn calculate_symmetry(&self, cells: &[&(i32, i32)], center_x: f32, center_y: f32) -> f32 {
        if cells.len() < 2 {
            return 0.0;
        }
        
        let cell_set: HashSet<(i32, i32)> = cells.iter().map(|&&pos| pos).collect();
        let mut symmetry_score = 0.0;
        let mut total_checks = 0;
        
        // Check horizontal symmetry
        for &&(x, y) in cells {
            let mirror_x = (2.0 * center_x) as i32 - x;
            total_checks += 1;
            
            if cell_set.contains(&(mirror_x, y)) {
                symmetry_score += 1.0;
            }
        }
        
        // Check vertical symmetry
        for &&(x, y) in cells {
            let mirror_y = (2.0 * center_y) as i32 - y;
            total_checks += 1;
            
            if cell_set.contains(&(x, mirror_y)) {
                symmetry_score += 1.0;
            }
        }
        
        if total_checks > 0 {
            symmetry_score / total_checks as f32
        } else {
            0.0
        }
    }
    
    /// Calculate chaos measure (0 = organized, 1 = random)
    fn calculate_chaos(&self, cells: &[&(i32, i32)]) -> f32 {
        if cells.len() < 3 {
            return 0.0;
        }
        
        let cell_set: HashSet<(i32, i32)> = cells.iter().map(|&&pos| pos).collect();
        let mut neighbor_counts = Vec::new();
        
        // Count neighbors for each cell
        for &&(x, y) in cells {
            let mut neighbors = 0;
            
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    
                    if cell_set.contains(&(x + dx, y + dy)) {
                        neighbors += 1;
                    }
                }
            }
            
            neighbor_counts.push(neighbors);
        }
        
        // Calculate variance in neighbor counts (higher variance = more chaos)
        let mean = neighbor_counts.iter().sum::<usize>() as f32 / neighbor_counts.len() as f32;
        let variance: f32 = neighbor_counts.iter()
            .map(|&count| {
                let diff = count as f32 - mean;
                diff * diff
            })
            .sum::<f32>() / neighbor_counts.len() as f32;
        
        // Normalize variance to [0, 1] range
        (variance / 8.0).min(1.0) // Max variance is around 8 for 8-connected neighbors
    }
}

use std::sync::Mutex;
use std::sync::OnceLock;

// Global analyzer instance using safe synchronization
static ANALYZER: OnceLock<Mutex<GameStateAnalyzer>> = OnceLock::new();

/// Get global analyzer instance
fn get_analyzer() -> &'static Mutex<GameStateAnalyzer> {
    ANALYZER.get_or_init(|| Mutex::new(GameStateAnalyzer::new()))
}

/// Extract quantitative features from game state for audio processing
pub fn extract_game_features(
    grid: &crate::InfiniteGrid,
    camera_state: &crate::camera::CameraState, 
    generation: u64
) -> GameStateFeatures {
    let analyzer_mutex = get_analyzer();
    let mut analyzer = analyzer_mutex.lock().unwrap();
    
    analyzer.extract_features(grid, camera_state, generation)
} 