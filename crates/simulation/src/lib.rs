//! Canonical commands, water accounting, fixed-rate orchestration and versioned replay.
#![forbid(unsafe_code)]
mod journal;
pub use journal::ReplayLimits;
use sim_core::{Checksum64, SimulationClock, SIMULATION_VERSION};
use world_model::{ActiveWorld, Biome, Chunk, ChunkCoord, TileCoord, CHUNK_SIDE, TILE_COUNT};
use worldgen::WorldGenerator;

pub const MAX_CHUNKS: usize = 25;
pub const MAX_COMMANDS: usize = 10_000;
pub const HYDROLOGY_PERIOD: u64 = 6; // 60 Hz world clock, 10 Hz water solver.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset { Terrain, Canal }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSpec {
    pub seed: u64,
    pub preset: Preset,
    pub chunks: Vec<ChunkCoord>,
}
impl WorldSpec {
    #[must_use]
    pub fn terrain(seed: u64, radius: i32) -> Self {
        assert!((0..=2).contains(&radius), "active radius must be 0..=2");
        let mut chunks = Vec::new();
        for x in -radius..=radius { for y in -radius..=radius { chunks.push(ChunkCoord::new(x, y)); }}
        Self { seed, preset: Preset::Terrain, chunks }
    }
    #[must_use]
    pub fn canal(seed: u64) -> Self {
        Self { seed, preset: Preset::Canal, chunks: vec![ChunkCoord::new(0, 0), ChunkCoord::new(1, 0)] }
    }
    fn validate(&self) -> Result<(), String> {
        if self.chunks.is_empty() || self.chunks.len() > MAX_CHUNKS {
            return Err("active chunk count must be 1..=25".into());
        }
        if self.chunks.windows(2).any(|p| p[0] >= p[1]) {
            return Err("chunks must be unique and canonically sorted".into());
        }
        if self.preset == Preset::Canal && self.chunks != Self::canal(self.seed).chunks {
            return Err("canal fixture requires chunks (0,0) and (1,0)".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Controlled uniform precipitation, not yet a weather simulation.
    Rain { millimetres: u32 },
    /// Controlled sink: removes at most this depth from each cell.
    Evaporate { max_mm: u32 },
    Pour { at: TileCoord, millimetres: u32 },
    /// External terrain editing conserves water; earthwork/energy accounting is not implemented.
    SetElevation { at: TileCoord, decimetres: i16 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedCommand { pub tick: u64, pub command: Command }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterLedger {
    pub initial_litres: u64,
    pub input_litres: u64,
    pub output_litres: u64,
}
impl WaterLedger {
    #[must_use]
    pub fn residual(self, current_litres: u64) -> i128 {
        i128::from(current_litres) + i128::from(self.output_litres)
            - i128::from(self.initial_litres) - i128::from(self.input_litres)
    }
}

#[derive(Debug, Clone)]
pub struct Simulation {
    spec: WorldSpec,
    clock: SimulationClock,
    world: ActiveWorld,
    ledger: WaterLedger,
    commands: Vec<RecordedCommand>,
    last_water_step: hydrology::StepReport,
}
impl Simulation {
    pub fn new(mut spec: WorldSpec) -> Result<Self, String> {
        spec.chunks.sort(); spec.validate()?;
        let mut world = ActiveWorld::default();
        let generator = WorldGenerator::new(spec.seed);
        for &coord in &spec.chunks {
            let chunk = match spec.preset {
                Preset::Terrain => generator.generate_chunk(coord),
                Preset::Canal => canal_chunk(coord),
            };
            world.insert_chunk(chunk);
        }
        let ledger = WaterLedger { initial_litres: world.water_litres(), input_litres: 0, output_litres: 0 };
        Ok(Self { spec, clock: SimulationClock::new(60), world, ledger,
            commands: Vec::new(), last_water_step: hydrology::StepReport::default() })
    }
    #[must_use]
    pub fn tick(&self) -> u64 { self.clock.tick() }
    #[must_use]
    pub fn world(&self) -> &ActiveWorld { &self.world }
    #[must_use]
    pub fn spec(&self) -> &WorldSpec { &self.spec }
    #[must_use]
    pub fn ledger(&self) -> WaterLedger { self.ledger }
    #[must_use]
    pub fn commands(&self) -> &[RecordedCommand] { &self.commands }
    #[must_use]
    pub fn last_water_step(&self) -> hydrology::StepReport { self.last_water_step }

    /// Commands validate all changes first. An error leaves both state and journal unchanged.
    pub fn command(&mut self, command: Command) -> Result<(), String> {
        if self.commands.len() >= MAX_COMMANDS { return Err("command journal is full; snapshot support is pending".into()); }
        match command {
            Command::Rain { millimetres } => {
                let cells = (self.world.chunks().len() as u64) * (TILE_COUNT as u64);
                let amount = u64::from(millimetres).checked_mul(cells).ok_or("rain amount overflow")?;
                let input = self.ledger.input_litres.checked_add(amount).ok_or("water input ledger overflow")?;
                if self.world.chunks().flat_map(|c| c.tiles()).any(|t| t.water_mm.checked_add(millimetres).is_none()) {
                    return Err("rain would exceed water depth storage; nothing changed".into());
                }
                for &coord in &self.spec.chunks {
                    let chunk = self.world.chunk_mut(coord).expect("fixed active set");
                    for y in 0..64 { for x in 0..64 { chunk.tile_mut(x, y).water_mm += millimetres; }}
                }
                self.ledger.input_litres = input;
            }
            Command::Evaporate { max_mm } => {
                let amount: u64 = self.world.chunks().flat_map(|c| c.tiles())
                    .map(|t| u64::from(t.water_mm.min(max_mm))).sum();
                let output = self.ledger.output_litres.checked_add(amount).ok_or("water output ledger overflow")?;
                for &coord in &self.spec.chunks {
                    let chunk = self.world.chunk_mut(coord).expect("fixed active set");
                    for y in 0..64 { for x in 0..64 {
                        let tile = chunk.tile_mut(x, y); tile.water_mm -= tile.water_mm.min(max_mm);
                    }}
                }
                self.ledger.output_litres = output;
            }
            Command::Pour { at, millimetres } => {
                let tile = self.world.tile(at).ok_or("pour target is not in the active world")?;
                let depth = tile.water_mm.checked_add(millimetres).ok_or("pour would exceed water depth storage")?;
                let input = self.ledger.input_litres.checked_add(u64::from(millimetres)).ok_or("water input ledger overflow")?;
                self.world.tile_mut(at).expect("validated target").water_mm = depth;
                self.ledger.input_litres = input;
            }
            Command::SetElevation { at, decimetres } => {
                self.world.tile_mut(at).ok_or("terrain target is not in the active world")?.elevation_dm = decimetres;
            }
        }
        self.commands.push(RecordedCommand { tick: self.tick(), command });
        debug_assert_eq!(self.ledger.residual(self.world.water_litres()), 0);
        Ok(())
    }

    /// Advance a bounded batch. Call again for another batch; no operating-system time is read.
    pub fn advance(&mut self, ticks: u64) -> Result<(), String> {
        if ticks > 60_000 { return Err("one batch may advance at most 60000 ticks".into()); }
        self.tick().checked_add(ticks).ok_or("simulation clock exhausted")?;
        for _ in 0..ticks {
            self.clock.advance();
            if self.tick() % HYDROLOGY_PERIOD == 0 {
                self.last_water_step = hydrology::step(&mut self.world);
            }
        }
        debug_assert_eq!(self.ledger.residual(self.world.water_litres()), 0);
        Ok(())
    }
    #[must_use]
    pub fn checksum(&self) -> u64 {
        let mut sum = Checksum64::new();
        sum.write(&SIMULATION_VERSION.to_le_bytes()); sum.write(&self.spec.seed.to_le_bytes());
        sum.write(&[match self.spec.preset { Preset::Terrain => 0, Preset::Canal => 1 }]);
        sum.write(&self.tick().to_le_bytes()); sum.write(&self.world.checksum().to_le_bytes());
        sum.write(&self.ledger.initial_litres.to_le_bytes()); sum.write(&self.ledger.input_litres.to_le_bytes());
        sum.write(&self.ledger.output_litres.to_le_bytes()); sum.finish()
    }
}

/// Hand-authored hydraulic test fixture, not an open-world generator.
fn canal_chunk(coord: ChunkCoord) -> Chunk {
    let mut chunk = Chunk::empty(coord);
    for y in 0..CHUNK_SIDE {
        for x in 0..CHUNK_SIDE {
            let gx = coord.tile(x, y).x; let tile = chunk.tile_mut(x, y);
            tile.elevation_dm = 100; tile.biome = Biome::Grassland; tile.fertility_q16 = 40_000;
            tile.temperature_d_c = 180;
            if (30..=34).contains(&y) {
                tile.elevation_dm = if gx == 64 { 20 } else { 0 };
                tile.water_mm = if gx < 64 { 500 } else { 0 };
                tile.biome = Biome::Wetland;
            }
        }
    }
    chunk.revision = 0; chunk
}
