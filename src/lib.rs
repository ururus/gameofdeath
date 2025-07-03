//! gameofdeath — pure, renderer-agnostic logic.
//!
//! This crate is `no_std` + `alloc` friendly (though we keep `std` by default).
//! Cells are bit-packed in row-major `Vec<u64>` for cache-friendly traversal.
//! All APIs avoid panics; invalid coordinates return `Dead`.
//!
//! # Features
//! * **wrap** *(default)* — toroidal edges; disable for hard boundaries.
//!
//! # Example
//! ```
//! use gameofdeath::{Grid, ConwayRule, CellState};
//! let mut g = Grid::new(5, 5);
//! g.set(1, 2, CellState::Alive);
//! g.step(&ConwayRule);
//! ```

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub mod infinite_grid;
pub mod start_screen;
pub mod camera;
pub mod cell_renderer;
pub mod ui;
pub mod hud;
pub mod settings;
pub mod config;
pub mod io;
pub mod audio;
pub mod synth_ui;



// Main exports
pub use infinite_grid::InfiniteGrid;
pub use start_screen::{RuleType, GameState, SelectedRule};
pub use camera::{GameCamera, CameraState};
pub use ui::{UiState};
pub use config::{Config};

// Re-export animation types from cell_renderer
pub use cell_renderer::{CellAnimation, AnimationType};

// Audio exports - spatial and hybrid systems
// GameAudioManager has been removed, using spatial_audio and hybrid_dungeon_synth instead

/// Cell state for multi-state cellular automata
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CellState {
    Dead = 0,
    Alive = 1,
    // Additional states for multi-state rules
    Dying = 2,      // For Brian's Brain (refractory state)
    Wire = 3,       // For WireWorld (wire)
    ElectronHead = 4, // For WireWorld (electron head)
    ElectronTail = 5, // For WireWorld (electron tail)
    SpeciesA = 6,   // For Immigration (species A)
    SpeciesB = 7,   // For Immigration (species B)
}

impl CellState {
    /// Check if this cell state is considered "alive" for neighbor counting
    pub fn is_alive(&self) -> bool {
        match self {
            CellState::Dead | CellState::Wire | CellState::ElectronTail => false,
            CellState::Alive | CellState::Dying | CellState::ElectronHead | 
            CellState::SpeciesA | CellState::SpeciesB => true,
        }
    }
    
    /// Check if this is an electron (for WireWorld)
    pub fn is_electron(&self) -> bool {
        matches!(self, CellState::ElectronHead | CellState::ElectronTail)
    }
    
    /// Check if this is a species (for Immigration)
    pub fn is_species(&self) -> bool {
        matches!(self, CellState::SpeciesA | CellState::SpeciesB)
    }
}

impl Default for CellState {
    fn default() -> Self { Self::Dead }
}

/// Trait for any Life-like rule.
pub trait Rule {
    /// Compute the next state of a cell `(x, y)` given the whole grid.
    fn next(&self, grid: &Grid, x: usize, y: usize) -> CellState;
}

/// Canonical Conway's Game of Life rule (B3/S23).
#[derive(Copy, Clone, Debug, Default)]
pub struct ConwayRule;

impl Rule for ConwayRule {
    fn next(&self, grid: &Grid, x: usize, y: usize) -> CellState {
        let alive = grid.is_alive(x, y);
        let n = grid.live_neighbours(x, y);
        match (alive, n) {
            (true, 2) | (true, 3) => CellState::Alive,
            (false, 3) => CellState::Alive,
            _ => CellState::Dead,
        }
    }
}
/// HighLife rule (B36/S23): Conway's survival (2 or 3), plus birth on 3 or 6 neighbors.
#[derive(Copy, Clone, Debug, Default)]
pub struct HighLifeRule;

impl Rule for HighLifeRule {
    fn next(&self, grid: &Grid, x: usize, y: usize) -> CellState {
        let alive = grid.is_alive(x, y);
        let n     = grid.live_neighbours(x, y);
        match (alive, n) {
            (true, 2) | (true, 3)        => CellState::Alive,
            (false, 3) | (false, 6)      => CellState::Alive,
            _                            => CellState::Dead,
        }
    }
}

/// Seeds rule (B2/S0): no survival, birth only on exactly 2 neighbors.
#[derive(Copy, Clone, Debug, Default)]
pub struct SeedsRule;

impl Rule for SeedsRule {
    fn next(&self, grid: &Grid, x: usize, y: usize) -> CellState {
        let alive = grid.is_alive(x, y);
        let n     = grid.live_neighbours(x, y);
        if !alive && n == 2 {
            CellState::Alive
        } else {
            CellState::Dead
        }
    }
}

/// 2-D bit-packed grid.
#[derive(Clone, Debug)]
pub struct Grid {
    cols: usize,
    rows: usize,
    data: Vec<u64>, // rows * ceil(cols/64)
}

impl Grid {
    /// Create a new empty grid (all dead).
    pub fn new(cols: usize, rows: usize) -> Self {
        let words_per_row = (cols + 63) / 64;
        Self {
            cols,
            rows,
            data: vec![0; words_per_row * rows],
        }
    }

    /// Width in cells.
    #[inline]
    pub fn cols(&self) -> usize { self.cols }
    /// Height in cells.
    #[inline]
    pub fn rows(&self) -> usize { self.rows }

    /// Return `true` if `(x, y)` within bounds **and alive**.
    pub fn is_alive(&self, x: usize, y: usize) -> bool {
        if x >= self.cols || y >= self.rows {
            return false;
        }
        let idx = y * self.words_per_row() + x / 64;
        let bit = 1u64 << (x & 63);
        self.data[idx] & bit != 0
    }

    /// Set cell state safely.
    pub fn set(&mut self, x: usize, y: usize, state: CellState) {
        if x >= self.cols || y >= self.rows {
            return;
        }
        let idx = y * self.words_per_row() + x / 64;
        let bit = 1u64 << (x & 63);
        if state == CellState::Alive {
            self.data[idx] |= bit;
        } else {
            self.data[idx] &= !bit;
        }
    }

    /// Advance the whole grid by one generation using `rule`.
    pub fn step(&mut self, rule: &impl Rule) {
        let mut next = self.data.clone();
        for y in 0..self.rows {
            for x in 0..self.cols {
                let state = rule.next(self, x, y);
                let idx = y * self.words_per_row() + x / 64;
                let bit = 1u64 << (x & 63);
                if state == CellState::Alive {
                    next[idx] |= bit;
                } else {
                    next[idx] &= !bit;
                }
            }
        }
        self.data = next;
    }

    /// Count live neighbours (wrap or clamp based on `wrap` feature).
    fn live_neighbours(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        for dy in [-1i32, 0, 1] {
            for dx in [-1i32, 0, 1] {
                if dx == 0 && dy == 0 { continue; }
                let nx = neighbour_coord(x, dx, self.cols);
                let ny = neighbour_coord(y, dy, self.rows);
                if self.is_alive(nx, ny) { count += 1; }
            }
        }
        count
    }

    /// Count the total number of live cells
    pub fn live_cell_count(&self) -> usize {
        let mut count = 0;
        for y in 0..self.rows {
            for x in 0..self.cols {
                if self.is_alive(x, y) {
                    count += 1;
                }
            }
        }
        count
    }

    #[inline]
    fn words_per_row(&self) -> usize { (self.cols + 63) / 64 }

    // Temporarily comment out the audio processing method
    // pub fn process_audio(&self, audio_context: &AudioContext) {
    //     // Update the grid state in the audio context
    //     audio_context.update_grid(self);
    // }
}

#[inline]
fn neighbour_coord(pos: usize, delta: i32, max: usize) -> usize {
    let signed = pos as i32 + delta;
    if cfg!(feature = "wrap") {
        ((signed % max as i32 + max as i32) % max as i32) as usize
    } else {
        signed.clamp(0, (max - 1) as i32) as usize
    }
}

// ---------- tests ----------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blinker_oscillates() {
        let mut g = Grid::new(5, 5);
        g.set(2, 1, CellState::Alive);
        g.set(2, 2, CellState::Alive);
        g.set(2, 3, CellState::Alive);

        // After 1 step, should be horizontal
        g.step(&ConwayRule);
        assert!(!g.is_alive(2, 1));
        assert!(g.is_alive(1, 2));
        assert!(g.is_alive(2, 2));
        assert!(g.is_alive(3, 2));
        assert!(!g.is_alive(2, 3));

        // After another step, back to vertical
        g.step(&ConwayRule);
        assert!(g.is_alive(2, 1));
        assert!(!g.is_alive(1, 2));
        assert!(g.is_alive(2, 2));
        assert!(!g.is_alive(3, 2));
        assert!(g.is_alive(2, 3));
    }
}
