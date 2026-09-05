//! Regional priority-flood drainage analysis, using four-connected cells.
//!
//! Boundary cells are explicit outlets. Filled elevations are a *virtual drainage surface*:
//! they do not raise canonical terrain or create actual water. Parent links form a DAG,
//! including across flat depressions. A whole regional patch must be analysed together;
//! doing this independently for each streaming chunk would create false outlets at seams.
use std::{cmp::Reverse, collections::BinaryHeap};

pub const MAX_DRAINAGE_CELLS: usize = 512 * 512;

#[derive(Debug, Clone)]
pub struct Drainage {
    pub width: usize,
    pub height: usize,
    pub filled_elevation_mm: Vec<i32>,
    /// None only for boundary outlets. All other links lead towards an outlet.
    pub downstream: Vec<Option<usize>>,
    /// Sum of local runoff units upstream, including this cell.
    pub accumulation: Vec<u64>,
    pub total_runoff: u64,
}
impl Drainage {
    pub fn build(width: usize, height: usize, elevation_mm: &[i32], runoff: &[u32]) -> Result<Self, String> {
        let len = width.checked_mul(height).ok_or("drainage dimensions overflow")?;
        if width < 2 || height < 2 || len > MAX_DRAINAGE_CELLS {
            return Err("drainage grid must be at least 2x2 and at most 262144 cells".into());
        }
        if elevation_mm.len() != len || runoff.len() != len {
            return Err("drainage input lengths do not match dimensions".into());
        }
        let mut filled = elevation_mm.to_vec();
        let mut downstream = vec![None; len];
        let mut visited = vec![false; len];
        let mut order = Vec::with_capacity(len);
        let mut heap = BinaryHeap::new();
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                    let i = y * width + x;
                    visited[i] = true;
                    heap.push(Reverse((filled[i], i)));
                }
            }
        }
        while let Some(Reverse((spill, i))) = heap.pop() {
            order.push(i);
            for j in neighbours(i, width, height).into_iter().flatten() {
                if !visited[j] {
                    visited[j] = true;
                    filled[j] = elevation_mm[j].max(spill);
                    downstream[j] = Some(i);
                    heap.push(Reverse((filled[j], j)));
                }
            }
        }
        let mut accumulation: Vec<u64> = runoff.iter().map(|&r| u64::from(r)).collect();
        let total_runoff = accumulation.iter().sum();
        // A parent is popped before a child is discovered, so reverse pop order is topological.
        for &i in order.iter().rev() {
            if let Some(parent) = downstream[i] {
                let carry = accumulation[i];
                accumulation[parent] += carry;
            }
        }
        Ok(Self { width, height, filled_elevation_mm: filled, downstream, accumulation, total_runoff })
    }
    #[must_use]
    pub fn outlet_runoff(&self) -> u64 {
        self.downstream.iter().enumerate().filter(|(_, p)| p.is_none())
            .map(|(i, _)| self.accumulation[i]).sum()
    }
    #[must_use]
    pub fn is_river(&self, index: usize, threshold: u64) -> bool {
        self.accumulation.get(index).is_some_and(|&flow| flow >= threshold)
            && self.downstream.get(index).is_some_and(Option::is_some)
    }
}
fn neighbours(i: usize, width: usize, height: usize) -> [Option<usize>; 4] {
    let x = i % width; let y = i / width;
    [if x > 0 { Some(i - 1) } else { None },
     if x + 1 < width { Some(i + 1) } else { None },
     if y > 0 { Some(i - width) } else { None },
     if y + 1 < height { Some(i + width) } else { None }]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bowl_fills_to_real_spill_level_without_mutating_input() {
        let mut ground = vec![10; 25]; ground[12] = -20; ground[7] = -10; ground[2] = 7;
        let before = ground.clone(); let map = Drainage::build(5, 5, &ground, &[1; 25]).unwrap();
        assert_eq!(map.filled_elevation_mm[12], 7);
        assert_eq!(map.filled_elevation_mm[7], 7);
        assert_eq!(map.outlet_runoff(), 25); assert_eq!(ground, before);
    }
    #[test]
    fn flat_cells_reach_an_outlet_without_cycles() {
        let map = Drainage::build(16, 16, &[0; 256], &[1; 256]).unwrap();
        for origin in 0..256 {
            let mut i = origin; let mut steps = 0;
            while let Some(next) = map.downstream[i] {
                assert!(steps < 256, "drainage cycle");
                assert!(map.filled_elevation_mm[next] <= map.filled_elevation_mm[i]);
                i = next; steps += 1;
            }
            assert!(i % 16 == 0 || i % 16 == 15 || i < 16 || i >= 240);
        }
        assert_eq!(map.outlet_runoff(), 256);
    }
    #[test]
    fn weighted_runoff_is_conserved_at_outlets() {
        let ground: Vec<i32> = (0..144).map(|i| ((i * 37) % 53) - 26).collect();
        let runoff: Vec<u32> = (0..144).map(|i| (i % 7) as u32).collect();
        let map = Drainage::build(12, 12, &ground, &runoff).unwrap();
        assert_eq!(map.outlet_runoff(), map.total_runoff);
        assert_eq!(map.total_runoff, runoff.iter().map(|&v| u64::from(v)).sum::<u64>());
    }
    #[test]
    fn rejects_bad_dimensions_and_shapes() {
        assert!(Drainage::build(1, 4, &[0; 4], &[1; 4]).is_err());
        assert!(Drainage::build(2, 2, &[0; 3], &[1; 4]).is_err());
        assert!(Drainage::build(usize::MAX, 3, &[], &[]).is_err());
    }
}
