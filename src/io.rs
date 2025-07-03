//! Pattern I/O: RLE (Run-Length Encoded) loader for Game of Life.
//!
//! Reference: <https://conwaylife.com/wiki/Run_Length_Encoded>
//! Only essential tokens are parsed. Comments and header lines are skipped.

use core::str::Chars;
use crate::{CellState, Grid};

pub mod lif;
pub use lif::load_lif_into_grid;

#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error("invalid character in RLE: {0}")]
    InvalidChar(char),
    #[error("run-length overflow or zero")]
    InvalidRun,
    #[error("pattern exceeds grid bounds")]
    OutOfBounds,
}

/// Load an RLE string into `grid`, placing top-left corner at `(ox, oy)`.
/// Returns `PatternError` if the pattern goes out of bounds or the RLE is malformed.
pub fn load_rle_into_grid(
    grid: &mut Grid,
    rle: &str,
    ox: usize,
    oy: usize,
) -> Result<(), PatternError> {
    // Remove comments and header lines.
    let cleaned: String = rle
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with('x'))
        .collect();
    parse_body(grid, &cleaned, ox, oy)
}

fn parse_body(
    grid: &mut Grid,
    body: &str,
    ox: usize,
    oy: usize,
) -> Result<(), PatternError> {
    let mut chars = body.chars();
    let (mut x, mut y) = (0usize, 0usize);
    while let Some(ch) = chars.next() {
        match ch {
            '0'..='9' => {
                let run = read_number(ch, &mut chars)?;
                if let Some(tok) = chars.next() {
                    apply_token(grid, tok, run, &mut x, &mut y, ox, oy)?;
                } else {
                    return Err(PatternError::InvalidRun);
                }
            }
            'b' | 'o' | '$' | '!' => apply_token(grid, ch, 1, &mut x, &mut y, ox, oy)?,
            '\n' | '\r' | ' ' => continue,
            _ => return Err(PatternError::InvalidChar(ch)),
        }
    }
    Ok(())
}

#[inline]
fn read_number(first: char, chars: &mut Chars) -> Result<usize, PatternError> {
    let mut n = first.to_digit(10).unwrap() as usize;
    while let Some(next) = chars.clone().next() {
        if next.is_ascii_digit() {
            chars.next();
            n = n * 10 + next.to_digit(10).unwrap() as usize;
        } else {
            break;
        }
    }
    if n == 0 { Err(PatternError::InvalidRun) } else { Ok(n) }
}

fn apply_token(
    grid: &mut Grid,
    tok: char,
    run: usize,
    x: &mut usize,
    y: &mut usize,
    ox: usize,
    oy: usize,
) -> Result<(), PatternError> {
    match tok {
        'b' => *x += run, // dead cells
        'o' => {
            for _ in 0..run {
                if ox + *x >= grid.cols() || oy + *y >= grid.rows() {
                    return Err(PatternError::OutOfBounds);
                }
                grid.set(ox + *x, oy + *y, CellState::Alive);
                *x += 1;
            }
        }
        '$' => {
            *y += run;
            *x = 0;
        }
        '!' => return Ok(()), // end of pattern
        _ => return Err(PatternError::InvalidChar(tok)),
    }
    Ok(())
}
