//! Conservative integer surface-water relaxation on active chunks.
//!
//! This is NOT a momentum/pressure shallow-water solver. It is an auditable first
//! hydraulic approximation: 1 mm depth quanta, a 7 mm head deadband, and no velocity field.
//! All proposals read the same old state. Each undirected edge is evaluated exactly once.
//! A cell has at most four neighbours, so per-edge source/receiver budgets of one quarter
//! guarantee nonnegative depths and no overflow without throwing water away.
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use world_model::{ActiveWorld, TileCoord, TileFields, CHUNK_SIDE};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StepReport {
    pub transferred_litres: u64,
    pub changed_cells: usize,
}

/// Missing chunks are sealed boundaries. Never auto-load a chunk or discard frontier water.
pub fn step(world: &mut ActiveWorld) -> StepReport {
    let mut deltas = BTreeMap::<TileCoord, i64>::new();
    let mut report = StepReport::default();
    for chunk in world.chunks() {
        for y in 0..CHUNK_SIDE {
            for x in 0..CHUNK_SIDE {
                let from = chunk.coord.tile(x, y); let a = *chunk.tile(x, y);
                for (dx, dy) in [(1, 0), (0, 1)] {
                    if let Some(to) = from.offset(dx, dy) {
                        if let Some(&b) = world.tile(to) {
                            propose(from, a, to, b, &mut deltas, &mut report);
                        }
                    }
                }
            }
        }
    }
    for (pos, delta) in deltas {
        if delta != 0 {
            let tile = world.tile_mut(pos).expect("proposed cell remains active");
            tile.water_mm = u32::try_from(i64::from(tile.water_mm) + delta)
                .expect("per-edge water budgets guarantee representable depth");
            report.changed_cells += 1;
        }
    }
    report
}

fn propose(a_pos: TileCoord, a: TileFields, b_pos: TileCoord, b: TileFields,
    deltas: &mut BTreeMap<TileCoord, i64>, report: &mut StepReport) {
    let difference = a.surface_mm() - b.surface_mm();
    if difference == 0 { return; }
    let (source_pos, source, target_pos, target) = if difference > 0 {
        (a_pos, a, b_pos, b)
    } else { (b_pos, b, a_pos, a) };
    let amount = (difference.unsigned_abs() / 8)
        .min(u64::from(source.water_mm / 4))
        .min(u64::from((u32::MAX - target.water_mm) / 4));
    if amount == 0 { return; }
    *deltas.entry(source_pos).or_default() -= amount as i64;
    *deltas.entry(target_pos).or_default() += amount as i64;
    report.transferred_litres += amount;
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_model::{Chunk, ChunkCoord};
    fn pair() -> ActiveWorld {
        let mut w = ActiveWorld::default();
        w.insert_chunk(Chunk::empty(ChunkCoord::new(-1, 0)));
        w.insert_chunk(Chunk::empty(ChunkCoord::new(0, 0))); w
    }
    #[test]
    fn water_crosses_chunk_boundary_and_is_conserved() {
        let mut w = pair(); w.tile_mut(TileCoord::new(-1, 30)).unwrap().water_mm = 10_000;
        let before = w.water_litres(); let report = step(&mut w);
        assert!(w.tile(TileCoord::new(0, 30)).unwrap().water_mm > 0);
        assert_eq!(w.water_litres(), before); assert!(report.transferred_litres > 0);
    }
    #[test]
    fn repeated_steps_conserve_every_litre() {
        let mut w = pair();
        for x in -60..60 {
            let tile = w.tile_mut(TileCoord::new(x, 30)).unwrap();
            tile.water_mm = ((x + 60) * 731) as u32; tile.elevation_dm = (x % 7) as i16;
        }
        let expected = w.water_litres();
        for _ in 0..120 { step(&mut w); assert_eq!(w.water_litres(), expected); }
    }
    #[test]
    fn zero_water_and_flat_surface_do_not_move() {
        let mut w = pair(); assert_eq!(step(&mut w), StepReport::default());
        // Different floor heights with the same surface must not drive a flow.
        let a = TileFields { elevation_dm: -10, water_mm: 2000, ..TileFields::default() };
        let b = TileFields { elevation_dm: 0, water_mm: 1000, ..TileFields::default() };
        let mut deltas = BTreeMap::new(); let mut report = StepReport::default();
        propose(TileCoord::new(0, 0), a, TileCoord::new(1, 0), b, &mut deltas, &mut report);
        assert!(deltas.is_empty());
    }
    #[test]
    fn missing_chunk_is_closed_not_a_sink() {
        let mut w = ActiveWorld::default(); w.insert_chunk(Chunk::empty(ChunkCoord::new(0, 0)));
        w.tile_mut(TileCoord::new(63, 63)).unwrap().water_mm = 100_000;
        for _ in 0..12 { step(&mut w); }
        assert_eq!(w.water_litres(), 100_000); assert_eq!(w.chunks().len(), 1);
    }
    #[test]
    fn near_maximum_depth_never_overflows_or_loses_mass() {
        let mut w = pair();
        for c in [ChunkCoord::new(-1, 0), ChunkCoord::new(0, 0)] {
            for y in 0..64 { for x in 0..64 {
                w.chunk_mut(c).unwrap().tile_mut(x, y).water_mm = u32::MAX - ((x + y) % 9) as u32;
            }}
        }
        let expected = w.water_litres();
        for _ in 0..12 { step(&mut w); assert_eq!(w.water_litres(), expected); }
    }
}
