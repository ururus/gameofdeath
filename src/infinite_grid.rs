use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use bevy::prelude::Resource;
use crate::CellState;
use crate::start_screen::RuleType;

/// Object pool for commonly used collections to reduce allocations
#[derive(Debug)]
pub struct CollectionPool {
    vec_pools: RefCell<Vec<Vec<(i32, i32)>>>,
    hashmap_pools: RefCell<Vec<HashMap<(i32, i32), CellState>>>,
    hashset_pools: RefCell<Vec<HashSet<(i32, i32)>>>,
}

impl CollectionPool {
    pub fn new() -> Self {
        Self {
            vec_pools: RefCell::new(Vec::with_capacity(8)),
            hashmap_pools: RefCell::new(Vec::with_capacity(4)),
            hashset_pools: RefCell::new(Vec::with_capacity(4)),
        }
    }

    pub fn get_vec(&self) -> Vec<(i32, i32)> {
        self.vec_pools.borrow_mut().pop().unwrap_or_else(|| Vec::with_capacity(1000))
    }

    pub fn return_vec(&self, mut vec: Vec<(i32, i32)>) {
        vec.clear();
        if vec.capacity() <= 10000 { // Don't store overly large vectors
            self.vec_pools.borrow_mut().push(vec);
        }
    }

    pub fn get_hashmap(&self) -> HashMap<(i32, i32), CellState> {
        self.hashmap_pools.borrow_mut().pop().unwrap_or_else(|| HashMap::with_capacity(1000))
    }

    pub fn return_hashmap(&self, mut map: HashMap<(i32, i32), CellState>) {
        map.clear();
        if map.capacity() <= 10000 { // Don't store overly large maps
            self.hashmap_pools.borrow_mut().push(map);
        }
    }

    pub fn get_hashset(&self) -> HashSet<(i32, i32)> {
        self.hashset_pools.borrow_mut().pop().unwrap_or_else(|| HashSet::with_capacity(2000))
    }

    pub fn return_hashset(&self, mut set: HashSet<(i32, i32)>) {
        set.clear();
        if set.capacity() <= 20000 { // Don't store overly large sets
            self.hashset_pools.borrow_mut().push(set);
        }
    }
}

thread_local! {
    static COLLECTION_POOL: CollectionPool = CollectionPool::new();
}

/// Infinite sparse grid using HashMap for storage with optimized collections
/// Only stores alive cells, treating missing cells as dead
#[derive(Clone, Debug, Default, Resource)]
pub struct InfiniteGrid {
    /// Map from (x, y) coordinates to cell state
    /// Only alive cells are stored
    alive_cells: HashMap<(i32, i32), CellState>,
    /// Cached alive cells vector to avoid recreating every frame
    cached_alive_positions: Vec<(i32, i32)>,
    /// Flag to track if cached positions are dirty
    cache_dirty: bool,
    /// Cached bounds for optimization
    bounds: Option<GridBounds>,
    /// Version counter for change detection
    version: u64,
}

#[derive(Clone, Debug)]
pub struct GridBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl InfiniteGrid {
    /// Create a new empty infinite grid
    pub fn new() -> Self {
        Self {
            alive_cells: HashMap::new(),
            cached_alive_positions: Vec::new(),
            cache_dirty: false,
            bounds: None,
            version: 0,
        }
    }

    /// Check if a cell is alive at the given coordinates
    pub fn is_alive(&self, x: i32, y: i32) -> bool {
        if let Some(state) = self.alive_cells.get(&(x, y)) {
            state.is_alive()
        } else {
            false
        }
    }

    /// Set a cell's state at the given coordinates
    pub fn set(&mut self, x: i32, y: i32, state: CellState) {
        match state {
            CellState::Dead => {
                self.alive_cells.remove(&(x, y));
                // Note: We don't update bounds when removing cells for performance
                // Bounds may be larger than actual content, which is fine
            }
            // All non-dead states are stored in the HashMap
            CellState::Alive | CellState::Dying | CellState::Wire | 
            CellState::ElectronHead | CellState::ElectronTail | 
            CellState::SpeciesA | CellState::SpeciesB => {
                self.alive_cells.insert((x, y), state);
                self.update_bounds(x, y);
            }
        }
        self.version += 1;
        self.cache_dirty = true; // Mark cache as dirty
    }

    /// Get the current version (for change detection)
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Count total alive cells
    pub fn live_cell_count(&self) -> usize {
        self.alive_cells.len()
    }

    /// Get all alive cell positions (cached version to avoid allocations)
    pub fn alive_cells(&self) -> impl Iterator<Item = &(i32, i32)> {
        self.alive_cells.keys()
    }

    /// Get a reference to all alive cell positions (cached for performance)
    pub fn get_alive_cells(&mut self) -> &Vec<(i32, i32)> {
        if self.cache_dirty {
            self.cached_alive_positions.clear();
            self.cached_alive_positions.extend(self.alive_cells.keys().copied());
            self.cache_dirty = false;
        }
        &self.cached_alive_positions
    }

    /// Get alive cells count without requiring mutable access (for read-only operations)
    pub fn alive_cells_count(&self) -> usize {
        self.alive_cells.len()
    }

    /// Get a snapshot of alive cell positions without requiring mutable access (for read-only operations)
    pub fn get_alive_cells_snapshot(&self) -> Vec<(i32, i32)> {
        self.alive_cells.keys().copied().collect()
    }

    /// Get cell state at coordinates
    pub fn get(&self, x: i32, y: i32) -> CellState {
        self.alive_cells.get(&(x, y)).cloned().unwrap_or(CellState::Dead)
    }

    /// Get population (number of alive cells)
    pub fn population(&self) -> usize {
        self.alive_cells.len()
    }

    /// Count neighbors for a cell (needed by audio system)
    pub fn count_neighbors(&self, x: i32, y: i32) -> u8 {
        self.live_neighbors(x, y)
    }

    /// Set a cell to alive
    pub fn set_alive(&mut self, x: i32, y: i32) {
        self.set(x, y, CellState::Alive);
    }

    /// Set a cell to dead
    pub fn set_dead(&mut self, x: i32, y: i32) {
        self.set(x, y, CellState::Dead);
    }

    /// Update the grid based on the specified rule type
    pub fn update(&mut self, rule: RuleType) {
        match rule {
            RuleType::Conway => self.step_conway(),
            RuleType::HighLife => self.step_highlife(),
            RuleType::Seeds => self.step_seeds(),
            RuleType::Brian => self.step_brian_brain(),
            RuleType::WireWorld => self.step_wireworld(),
            RuleType::Immigration => self.step_immigration(),
            RuleType::Mazectric => self.step_mazectric(),
            RuleType::Coral => self.step_coral(),
            RuleType::Gnarl => self.step_gnarl(),
            RuleType::Replicator => self.step_replicator(),
        }
    }

    /// Get bounds of the grid (may be None if empty)
    pub fn bounds(&self) -> Option<&GridBounds> {
        self.bounds.as_ref()
    }

    /// Clear all cells
    pub fn clear(&mut self) {
        self.alive_cells.clear();
        self.cached_alive_positions.clear();
        self.cache_dirty = false;
        self.bounds = None;
        self.version += 1;
    }

    /// Update the cached bounds when adding a cell
    fn update_bounds(&mut self, x: i32, y: i32) {
        match &mut self.bounds {
            Some(bounds) => {
                bounds.min_x = bounds.min_x.min(x);
                bounds.max_x = bounds.max_x.max(x);
                bounds.min_y = bounds.min_y.min(y);
                bounds.max_y = bounds.max_y.max(y);
            }
            None => {
                self.bounds = Some(GridBounds {
                    min_x: x,
                    max_x: x,
                    min_y: y,
                    max_y: y,
                });
            }
        }
    }

    /// Count live neighbors around a cell
    fn live_neighbors(&self, x: i32, y: i32) -> u8 {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if self.is_alive(x + dx, y + dy) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Advance the grid by one generation using Conway's rules
    /// For now, we'll hardcode Conway's rule and extend later
    pub fn step_conway(&mut self) {
        COLLECTION_POOL.with(|pool| {
            let mut new_alive_cells = pool.get_hashmap();
            let mut candidates = pool.get_hashset();

            // Add all currently alive cells and their neighbors as candidates
            for &(x, y) in self.alive_cells.keys() {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        candidates.insert((x + dx, y + dy));
                    }
                }
            }

            // Evaluate each candidate cell using Conway's rules
            for &(x, y) in &candidates {
                let alive = self.is_alive(x, y);
                let neighbors = self.live_neighbors(x, y);
                
                let new_state = match (alive, neighbors) {
                    (true, 2) | (true, 3) => CellState::Alive,
                    (false, 3) => CellState::Alive,
                    _ => CellState::Dead,
                };
                
                if new_state == CellState::Alive {
                    new_alive_cells.insert((x, y), new_state);
                }
            }

            // Replace the alive cells with the new generation
            self.alive_cells = new_alive_cells;
            self.recalculate_bounds();
            self.version += 1;
            self.cache_dirty = true;

            // Return collections to pool
            pool.return_hashset(candidates);
            // new_alive_cells is consumed by self.alive_cells, so no need to return it
        });
    }

    /// Advance the grid by one generation using HighLife rules
    pub fn step_highlife(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        // Add all currently alive cells and their neighbors as candidates
        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        // Evaluate each candidate cell using HighLife rules
        for &(x, y) in &candidates {
            let alive = self.is_alive(x, y);
            let neighbors = self.live_neighbors(x, y);
            
            let new_state = match (alive, neighbors) {
                (true, 2) | (true, 3) => CellState::Alive,
                (false, 3) | (false, 6) => CellState::Alive,
                _ => CellState::Dead,
            };
            
            if new_state == CellState::Alive {
                new_alive_cells.insert((x, y), CellState::Alive);
            }
        }

        // Update the grid
        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Advance the grid by one generation using Seeds rules
    pub fn step_seeds(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        // Add all currently alive cells and their neighbors as candidates
        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        // Evaluate each candidate cell using Seeds rules (B2/S0)
        for &(x, y) in &candidates {
            let alive = self.is_alive(x, y);
            let neighbors = self.live_neighbors(x, y);
            
            let new_state = if !alive && neighbors == 2 {
                CellState::Alive
            } else {
                CellState::Dead
            };
            
            if new_state == CellState::Alive {
                new_alive_cells.insert((x, y), CellState::Alive);
            }
        }

        // Update the grid
        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Recalculate bounds from scratch (used after step)
    fn recalculate_bounds(&mut self) {
        if self.alive_cells.is_empty() {
            self.bounds = None;
            return;
        }

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for &(x, y) in self.alive_cells.keys() {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        self.bounds = Some(GridBounds {
            min_x,
            max_x,
            min_y,
            max_y,
        });
    }

    /// Insert a pattern at the given offset
    pub fn insert_pattern<I>(&mut self, pattern: I, offset_x: i32, offset_y: i32)
    where
        I: Iterator<Item = (i32, i32)>,
    {
        for (x, y) in pattern {
            self.set(offset_x + x, offset_y + y, CellState::Alive);
        }
    }

    /// Get cells in a specific region (for rendering)
    pub fn cells_in_region(&self, min_x: i32, max_x: i32, min_y: i32, max_y: i32) -> impl Iterator<Item = &(i32, i32)> {
        self.alive_cells.keys().filter(move |&&(x, y)| {
            x >= min_x && x <= max_x && y >= min_y && y <= max_y
        })
    }
    /// Brian's Brain rule - 3-state automaton
    /// States: Dead, Alive (firing), Dying (refractory)
    pub fn step_brian_brain(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        // Add all cells and their neighbors as candidates
        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        for &(x, y) in &candidates {
            let current_state = self.get(x, y);
            let firing_neighbors = self.count_firing_neighbors(x, y);
            
            let new_state = match current_state {
                CellState::Dead => {
                    if firing_neighbors == 2 {
                        CellState::Alive // Become firing
                    } else {
                        CellState::Dead
                    }
                }
                CellState::Alive => CellState::Dying, // Firing → Refractory
                CellState::Dying => CellState::Dead,  // Refractory → Dead
                _ => CellState::Dead,
            };
            
            if new_state != CellState::Dead {
                new_alive_cells.insert((x, y), new_state);
            }
        }

        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// WireWorld rule - 4-state digital circuit simulation
    /// States: Empty, Wire, Electron Head, Electron Tail
    pub fn step_wireworld(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        // Add all cells and their neighbors as candidates
        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        for &(x, y) in &candidates {
            let current_state = self.get(x, y);
            
            let new_state = match current_state {
                CellState::Dead => CellState::Dead,
                CellState::Wire => {
                    let electron_heads = self.count_electron_heads(x, y);
                    if electron_heads == 1 || electron_heads == 2 {
                        CellState::ElectronHead
                    } else {
                        CellState::Wire
                    }
                }
                CellState::ElectronHead => CellState::ElectronTail,
                CellState::ElectronTail => CellState::Wire,
                _ => CellState::Dead,
            };
            
            if new_state != CellState::Dead {
                new_alive_cells.insert((x, y), new_state);
            }
        }

        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Immigration rule - Conway with 2 competing species
    /// B3/S23 but species can only give birth to their own kind
    pub fn step_immigration(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        for &(x, y) in &candidates {
            let current_state = self.get(x, y);
            let (species_a_neighbors, species_b_neighbors) = self.count_species_neighbors(x, y);
            let total_neighbors = species_a_neighbors + species_b_neighbors;
            
            let new_state = match current_state {
                CellState::SpeciesA => {
                    if total_neighbors == 2 || total_neighbors == 3 {
                        CellState::SpeciesA
                    } else {
                        CellState::Dead
                    }
                }
                CellState::SpeciesB => {
                    if total_neighbors == 2 || total_neighbors == 3 {
                        CellState::SpeciesB
                    } else {
                        CellState::Dead
                    }
                }
                CellState::Dead => {
                    if total_neighbors == 3 {
                        // Majority species gives birth
                        if species_a_neighbors > species_b_neighbors {
                            CellState::SpeciesA
                        } else if species_b_neighbors > species_a_neighbors {
                            CellState::SpeciesB
                        } else {
                            // Tie - random choice (use position hash)
                            if (x + y) % 2 == 0 { CellState::SpeciesA } else { CellState::SpeciesB }
                        }
                    } else {
                        CellState::Dead
                    }
                }
                _ => current_state, // Keep other states as-is
            };
            
            if new_state != CellState::Dead {
                new_alive_cells.insert((x, y), new_state);
            }
        }

        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Mazectric rule - B3/S1234 - Creates intricate maze patterns
    pub fn step_mazectric(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        for &(x, y) in &candidates {
            let alive = self.is_alive(x, y);
            let neighbors = self.live_neighbors(x, y);
            
            let new_state = match (alive, neighbors) {
                (true, 1) | (true, 2) | (true, 3) | (true, 4) => CellState::Alive,
                (false, 3) => CellState::Alive,
                _ => CellState::Dead,
            };
            
            if new_state == CellState::Alive {
                new_alive_cells.insert((x, y), CellState::Alive);
            }
        }

        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Coral rule - B3/S45678 - Coral-like growth structures
    pub fn step_coral(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        for &(x, y) in &candidates {
            let alive = self.is_alive(x, y);
            let neighbors = self.live_neighbors(x, y);
            
            let new_state = match (alive, neighbors) {
                (true, 4) | (true, 5) | (true, 6) | (true, 7) | (true, 8) => CellState::Alive,
                (false, 3) => CellState::Alive,
                _ => CellState::Dead,
            };
            
            if new_state == CellState::Alive {
                new_alive_cells.insert((x, y), CellState::Alive);
            }
        }

        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Gnarl rule - B1/S1 - Chaotic explosive growth
    pub fn step_gnarl(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        for &(x, y) in &candidates {
            let alive = self.is_alive(x, y);
            let neighbors = self.live_neighbors(x, y);
            
            let new_state = match (alive, neighbors) {
                (true, 1) => CellState::Alive,
                (false, 1) => CellState::Alive,
                _ => CellState::Dead,
            };
            
            if new_state == CellState::Alive {
                new_alive_cells.insert((x, y), CellState::Alive);
            }
        }

        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Replicator rule - B1357/S1357 - Perfect self-replication
    pub fn step_replicator(&mut self) {
        let mut new_alive_cells = HashMap::new();
        let mut candidates = std::collections::HashSet::new();

        for &(x, y) in self.alive_cells.keys() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    candidates.insert((x + dx, y + dy));
                }
            }
        }

        for &(x, y) in &candidates {
            let alive = self.is_alive(x, y);
            let neighbors = self.live_neighbors(x, y);
            
            let new_state = match (alive, neighbors) {
                (true, 1) | (true, 3) | (true, 5) | (true, 7) => CellState::Alive,
                (false, 1) | (false, 3) | (false, 5) | (false, 7) => CellState::Alive,
                _ => CellState::Dead,
            };
            
            if new_state == CellState::Alive {
                new_alive_cells.insert((x, y), CellState::Alive);
            }
        }

        self.alive_cells = new_alive_cells;
        self.recalculate_bounds();
        self.version += 1;
    }

    /// Helper function to count firing neighbors for Brian's Brain
    fn count_firing_neighbors(&self, x: i32, y: i32) -> u8 {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if self.get(x + dx, y + dy) == CellState::Alive {
                    count += 1;
                }
            }
        }
        count
    }

    /// Helper function to count electron heads for WireWorld
    fn count_electron_heads(&self, x: i32, y: i32) -> u8 {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if self.get(x + dx, y + dy) == CellState::ElectronHead {
                    count += 1;
                }
            }
        }
        count
    }

    /// Helper function to count species neighbors for Immigration
    fn count_species_neighbors(&self, x: i32, y: i32) -> (u8, u8) {
        let mut species_a = 0;
        let mut species_b = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                match self.get(x + dx, y + dy) {
                    CellState::SpeciesA => species_a += 1,
                    CellState::SpeciesB => species_b += 1,
                    _ => {}
                }
            }
        }
        (species_a, species_b)
    }
}

// Rules are now implemented directly in the step methods

/// Some common patterns for testing
pub mod patterns {

    /// Create a glider pattern
    pub fn glider() -> impl Iterator<Item = (i32, i32)> {
        vec![
            (1, 0),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ].into_iter()
    }

    /// Create a blinker pattern
    pub fn blinker() -> impl Iterator<Item = (i32, i32)> {
        vec![
            (0, 0),
            (1, 0),
            (2, 0),
        ].into_iter()
    }

    /// Create a block pattern (still life)
    pub fn block() -> impl Iterator<Item = (i32, i32)> {
        vec![
            (0, 0),
            (1, 0),
            (0, 1),
            (1, 1),
        ].into_iter()
    }
} 