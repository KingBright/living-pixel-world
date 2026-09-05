//! Dense arrays of tile records. Chunk order is stable; unloaded chunks are absent.
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use sim_core::{Checksum64, SIMULATION_VERSION};

pub const CHUNK_SIDE: usize = 64;
pub const TILE_COUNT: usize = CHUNK_SIDE * CHUNK_SIDE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkCoord { pub x: i32, pub y: i32 }
impl ChunkCoord {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self { Self { x, y } }
    #[must_use]
    pub fn tile(self, local_x: usize, local_y: usize) -> TileCoord {
        assert!(local_x < CHUNK_SIDE && local_y < CHUNK_SIDE);
        TileCoord::new(i64::from(self.x) * 64 + local_x as i64,
            i64::from(self.y) * 64 + local_y as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileCoord { pub x: i64, pub y: i64 }
impl TileCoord {
    #[must_use]
    pub const fn new(x: i64, y: i64) -> Self { Self { x, y } }
    #[must_use]
    pub fn split(self) -> Option<(ChunkCoord, usize, usize)> {
        Some((ChunkCoord::new(i32::try_from(self.x.div_euclid(64)).ok()?,
            i32::try_from(self.y.div_euclid(64)).ok()?),
            self.x.rem_euclid(64) as usize, self.y.rem_euclid(64) as usize))
    }
    #[must_use]
    pub fn offset(self, dx: i64, dy: i64) -> Option<Self> {
        Some(Self::new(self.x.checked_add(dx)?, self.y.checked_add(dy)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Biome {
    Ocean = 0, Tundra = 1, Taiga = 2, TemperateForest = 3, Grassland = 4,
    Wetland = 5, Desert = 6, Alpine = 7, #[default] Barren = 8,
}

/// One square-metre tile. One millimetre of water equals one litre on this tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TileFields {
    pub elevation_dm: i16,
    /// u32 rather than u16: the old field could not represent ocean depths above 65.535 m.
    pub water_mm: u32,
    pub soil_moisture_q16: u16,
    pub temperature_d_c: i16,
    pub fertility_q16: u16,
    pub plant_biomass_g_m2: u16,
    pub biome: Biome,
    pub flags: u16,
}
impl TileFields {
    #[must_use]
    pub fn surface_mm(self) -> i64 { i64::from(self.elevation_dm) * 100 + i64::from(self.water_mm) }
    fn hash_into(self, hash: &mut Checksum64) {
        hash.write(&self.elevation_dm.to_le_bytes()); hash.write(&self.water_mm.to_le_bytes());
        hash.write(&self.soil_moisture_q16.to_le_bytes()); hash.write(&self.temperature_d_c.to_le_bytes());
        hash.write(&self.fertility_q16.to_le_bytes()); hash.write(&self.plant_biomass_g_m2.to_le_bytes());
        hash.write(&[self.biome as u8]); hash.write(&self.flags.to_le_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub coord: ChunkCoord,
    tiles: Box<[TileFields; TILE_COUNT]>,
    /// Presentation dirty counter; deliberately excluded from canonical state checksums.
    pub revision: u64,
}
impl Chunk {
    #[must_use]
    pub fn empty(coord: ChunkCoord) -> Self {
        // Allocate on the heap without first constructing a large stack array.
        let tiles = vec![TileFields::default(); TILE_COUNT].into_boxed_slice()
            .try_into().expect("fixed chunk allocation length");
        Self { coord, tiles, revision: 0 }
    }
    #[must_use]
    pub const fn tile_index(x: usize, y: usize) -> usize {
        assert!(x < CHUNK_SIDE && y < CHUNK_SIDE, "tile is outside chunk");
        y * CHUNK_SIDE + x
    }
    #[must_use]
    pub fn tile(&self, x: usize, y: usize) -> &TileFields { &self.tiles[Self::tile_index(x, y)] }
    pub fn tile_mut(&mut self, x: usize, y: usize) -> &mut TileFields {
        self.revision = self.revision.wrapping_add(1);
        &mut self.tiles[Self::tile_index(x, y)]
    }
    #[must_use]
    pub fn tiles(&self) -> &[TileFields; TILE_COUNT] { &self.tiles }
    #[must_use]
    pub fn checksum(&self) -> u64 {
        let mut hash = Checksum64::new();
        hash.write(&self.coord.x.to_le_bytes()); hash.write(&self.coord.y.to_le_bytes());
        for tile in self.tiles.iter() { tile.hash_into(&mut hash); }
        hash.finish()
    }
}

#[derive(Debug, Default, Clone)]
pub struct ActiveWorld { chunks: BTreeMap<ChunkCoord, Chunk> }
impl ActiveWorld {
    pub fn insert_chunk(&mut self, chunk: Chunk) -> Option<Chunk> { self.chunks.insert(chunk.coord, chunk) }
    pub fn remove_chunk(&mut self, coord: ChunkCoord) -> Option<Chunk> { self.chunks.remove(&coord) }
    #[must_use]
    pub fn chunk(&self, coord: ChunkCoord) -> Option<&Chunk> { self.chunks.get(&coord) }
    pub fn chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> { self.chunks.get_mut(&coord) }
    pub fn chunks(&self) -> impl ExactSizeIterator<Item = &Chunk> { self.chunks.values() }
    #[must_use]
    pub fn tile(&self, pos: TileCoord) -> Option<&TileFields> {
        let (coord, x, y) = pos.split()?; Some(self.chunk(coord)?.tile(x, y))
    }
    pub fn tile_mut(&mut self, pos: TileCoord) -> Option<&mut TileFields> {
        let (coord, x, y) = pos.split()?; Some(self.chunk_mut(coord)?.tile_mut(x, y))
    }
    #[must_use]
    pub fn water_litres(&self) -> u64 {
        self.chunks().flat_map(|c| c.tiles()).map(|t| u64::from(t.water_mm)).sum()
    }
    #[must_use]
    pub fn checksum(&self) -> u64 {
        let mut hash = Checksum64::new();
        hash.write(&SIMULATION_VERSION.to_le_bytes());
        hash.write(&(self.chunks.len() as u64).to_le_bytes());
        for chunk in self.chunks() { hash.write(&chunk.checksum().to_le_bytes()); }
        hash.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_coordinates_round_trip() {
        for x in [-129, -65, -64, -63, -1, 0, 63, 64, 65, 128] {
            for y in [-65, -1, 0, 64] {
                let pos = TileCoord::new(x, y); let (c, lx, ly) = pos.split().unwrap();
                assert_eq!(c.tile(lx, ly), pos);
            }
        }
        assert!(TileCoord::new(i64::MAX, 0).split().is_none());
    }
    #[test]
    fn water_depth_and_height_have_matching_units() {
        let tile = TileFields { elevation_dm: -1000, water_mm: 100_000, ..TileFields::default() };
        assert_eq!(tile.surface_mm(), 0);
    }
    #[test]
    fn insertion_order_does_not_change_state_hash() {
        let mut a = ActiveWorld::default(); let mut b = ActiveWorld::default();
        for x in -1..=1 { a.insert_chunk(Chunk::empty(ChunkCoord::new(x, 0))); }
        for x in (-1..=1).rev() { b.insert_chunk(Chunk::empty(ChunkCoord::new(x, 0))); }
        assert_eq!(a.checksum(), b.checksum());
    }
    #[test]
    fn revision_is_not_physics() {
        let mut chunk = Chunk::empty(ChunkCoord::new(0, 0)); let before = chunk.checksum();
        chunk.revision = 42; assert_eq!(before, chunk.checksum());
        chunk.tile_mut(0, 0).water_mm = 1; assert_ne!(before, chunk.checksum());
    }
    #[test]
    fn removed_chunk_has_no_active_state() {
        let mut w = ActiveWorld::default(); let c = ChunkCoord::new(0, 0);
        w.insert_chunk(Chunk::empty(c)); assert!(w.remove_chunk(c).is_some());
        assert_eq!(w.chunks().len(), 0); assert!(w.tile(TileCoord::new(0, 0)).is_none());
    }
}
