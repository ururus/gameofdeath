//! Plain-text *.lif* (Life 1.x) pattern loader.
//!
//! • Header line starts with “#Life 1.” (ignored)
//! • Comment lines begin with ‘#’.
//! • “#P x y” re-positions the cursor.
//! • Pattern rows use ‘.’ (dead) and ‘*’ (alive).

use crate::{CellState, Grid};

#[derive(Debug, thiserror::Error)]
pub enum LifError {
    #[error("pattern exceeds grid bounds")]
    OutOfBounds,
    #[error("invalid character '{0}' in .lif file")]
    BadChar(char),
}

/// Load a .lif string into `grid`, top-left offset `(ox, oy)`.
pub fn load_lif_into_grid(
    grid: &mut Grid,
    lif: &str,
    ox: usize,
    oy: usize,
) -> Result<(), LifError> {
    let mut px = 0isize;
    let mut py = 0isize;

    for line in lif.lines() {
        if line.starts_with("#Life") || line.starts_with("#N") || line.starts_with("#D") {
            continue; // header / comments
        }
        if let Some(rest) = line.strip_prefix("#P ") {
            let parts: Vec<_> = rest.split_whitespace().collect();
            if parts.len() == 2 {
                px = parts[0].parse::<isize>().unwrap_or(0);
                py = parts[1].parse::<isize>().unwrap_or(0);
            }
            continue;
        }

        for (dx, ch) in line.chars().enumerate() {
            match ch {
                '.' => {}
                '*' => {
                    let gx = ox as isize + px + dx as isize;
                    let gy = oy as isize + py;
                    if gx < 0
                        || gy < 0
                        || gx as usize >= grid.cols()
                        || gy as usize >= grid.rows()
                    {
                        return Err(LifError::OutOfBounds);
                    }
                    grid.set(gx as usize, gy as usize, CellState::Alive);
                }
                _ => return Err(LifError::BadChar(ch)),
            }
        }
        py += 1;
        px = 0;
    }
    Ok(())
}