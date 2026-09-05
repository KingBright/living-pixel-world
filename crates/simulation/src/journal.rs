//! Little-endian seed + command journal. Replay is O(elapsed ticks), not a snapshot system.
use sim_core::{checksum64, SIMULATION_VERSION};
use world_model::{ChunkCoord, TileCoord, TILE_COUNT};
use crate::{Command, Preset, RecordedCommand, Simulation, WorldSpec, HYDROLOGY_PERIOD, MAX_CHUNKS, MAX_COMMANDS};

const MAGIC: &[u8; 8] = b"LPWJRNL\0";
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
#[derive(Debug, Clone, Copy)]
pub struct ReplayLimits {
    pub max_ticks: u64,
    pub max_cell_updates: u64,
}
impl Default for ReplayLimits {
    fn default() -> Self { Self { max_ticks: 60_000, max_cell_updates: 100_000_000 } }
}
impl Simulation {
    /// Check the default loader's bounded work estimate before offering a journal to a user.
    /// This does not run a replay and is not proof of a successful restore.
    pub fn check_replay_budget(&self, limits: ReplayLimits) -> Result<(), String> {
        check_cost(self.tick(), self.spec.chunks.len(), &self.commands, limits)
    }
    #[must_use]
    pub fn to_journal(&self) -> Vec<u8> {
        let mut out = Vec::new(); out.extend_from_slice(MAGIC);
        out.extend_from_slice(&SIMULATION_VERSION.to_le_bytes());
        out.push(match self.spec.preset { Preset::Terrain => 0, Preset::Canal => 1 });
        out.extend_from_slice(&self.spec.seed.to_le_bytes()); out.extend_from_slice(&self.tick().to_le_bytes());
        out.extend_from_slice(&(self.spec.chunks.len() as u32).to_le_bytes());
        for coord in &self.spec.chunks {
            out.extend_from_slice(&coord.x.to_le_bytes()); out.extend_from_slice(&coord.y.to_le_bytes());
        }
        out.extend_from_slice(&(self.commands.len() as u32).to_le_bytes());
        for event in &self.commands {
            out.extend_from_slice(&event.tick.to_le_bytes());
            match event.command {
                Command::Rain { millimetres } => { out.push(0); out.extend_from_slice(&millimetres.to_le_bytes()); }
                Command::Evaporate { max_mm } => { out.push(1); out.extend_from_slice(&max_mm.to_le_bytes()); }
                Command::Pour { at, millimetres } => {
                    out.push(2); write_pos(&mut out, at); out.extend_from_slice(&millimetres.to_le_bytes());
                }
                Command::SetElevation { at, decimetres } => {
                    out.push(3); write_pos(&mut out, at); out.extend_from_slice(&decimetres.to_le_bytes());
                }
            }
        }
        out.extend_from_slice(&self.checksum().to_le_bytes());
        let hash = checksum64(out.iter().copied()); out.extend_from_slice(&hash.to_le_bytes()); out
    }

    pub fn from_journal(bytes: &[u8], limits: ReplayLimits) -> Result<Self, String> {
        if bytes.len() < 16 || bytes.len() > MAX_FILE_BYTES { return Err("journal size is outside allowed bounds".into()); }
        let (payload, trailer) = bytes.split_at(bytes.len() - 8);
        let stored_hash = u64::from_le_bytes(trailer.try_into().map_err(|_| "truncated checksum")?);
        if stored_hash != checksum64(payload.iter().copied()) { return Err("journal checksum mismatch".into()); }
        let mut reader = Reader { bytes: payload, pos: 0 };
        if &reader.take::<8>()? != MAGIC { return Err("not a Living Pixel World journal".into()); }
        let version = u32::from_le_bytes(reader.take()?);
        if version != SIMULATION_VERSION { return Err(format!("unsupported simulation version {version}; migration required")); }
        let preset = match reader.take::<1>()?[0] {
            0 => Preset::Terrain, 1 => Preset::Canal, _ => return Err("unknown world preset".into()),
        };
        let seed = u64::from_le_bytes(reader.take()?); let ticks = u64::from_le_bytes(reader.take()?);
        if ticks > limits.max_ticks { return Err("journal exceeds replay tick budget".into()); }
        let count = u32::from_le_bytes(reader.take()?) as usize;
        if count == 0 || count > MAX_CHUNKS { return Err("journal chunk count exceeds budget".into()); }
        let mut chunks = Vec::with_capacity(count);
        for _ in 0..count {
            chunks.push(ChunkCoord::new(i32::from_le_bytes(reader.take()?), i32::from_le_bytes(reader.take()?)));
        }
        let spec = WorldSpec { seed, preset, chunks }; spec.validate()?;
        let count = u32::from_le_bytes(reader.take()?) as usize;
        if count > MAX_COMMANDS { return Err("journal command count exceeds budget".into()); }
        let mut events = Vec::with_capacity(count); let mut previous_tick = 0;
        for _ in 0..count {
            let tick = u64::from_le_bytes(reader.take()?);
            if tick < previous_tick || tick > ticks { return Err("journal commands are not in chronological order".into()); }
            previous_tick = tick;
            let command = match reader.take::<1>()?[0] {
                0 => Command::Rain { millimetres: u32::from_le_bytes(reader.take()?) },
                1 => Command::Evaporate { max_mm: u32::from_le_bytes(reader.take()?) },
                2 => Command::Pour { at: reader.position()?, millimetres: u32::from_le_bytes(reader.take()?) },
                3 => Command::SetElevation { at: reader.position()?, decimetres: i16::from_le_bytes(reader.take()?) },
                _ => return Err("unknown command tag".into()),
            };
            events.push(RecordedCommand { tick, command });
        }
        let expected_state = u64::from_le_bytes(reader.take()?);
        if reader.pos != payload.len() { return Err("unexpected trailing journal bytes".into()); }
        // Count full-grid commands too: zero-tick rain spam must not bypass the work budget.
        check_cost(ticks, spec.chunks.len(), &events, limits)?;
        // Parse and validate budgets before allocating a world or running any simulation.
        let mut sim = Self::new(spec)?;
        for event in events {
            advance_to(&mut sim, event.tick)?; sim.command(event.command)?;
        }
        advance_to(&mut sim, ticks)?;
        if sim.checksum() != expected_state { return Err("replayed state differs from saved state".into()); }
        Ok(sim)
    }
}
fn check_cost(ticks: u64, chunks: usize, events: &[RecordedCommand], limits: ReplayLimits) -> Result<(), String> {
    if ticks > limits.max_ticks { return Err("journal exceeds replay tick budget".into()); }
    let grid_commands = events.iter().filter(|e| matches!(e.command,
        Command::Rain { .. } | Command::Evaporate { .. })).count() as u64;
    // Coarse bounded-work estimate, not elapsed CPU time: creation, solver passes,
    // validation/mutation of global commands, and debug accounting after each command.
    let passes = (ticks / HYDROLOGY_PERIOD).checked_mul(2)
        .and_then(|n| n.checked_add(grid_commands.checked_mul(2)?))
        .and_then(|n| n.checked_add(events.len() as u64))
        .and_then(|n| n.checked_add(1)).ok_or("replay cost overflow")?;
    let work = passes.checked_mul(chunks as u64)
        .and_then(|n| n.checked_mul(TILE_COUNT as u64)).ok_or("replay cost overflow")?;
    if work > limits.max_cell_updates { return Err("journal exceeds replay cell-update budget".into()); }
    Ok(())
}
fn advance_to(sim: &mut Simulation, target: u64) -> Result<(), String> {
    while sim.tick() < target { sim.advance((target - sim.tick()).min(60_000))?; } Ok(())
}
fn write_pos(out: &mut Vec<u8>, at: TileCoord) {
    out.extend_from_slice(&at.x.to_le_bytes()); out.extend_from_slice(&at.y.to_le_bytes());
}
struct Reader<'a> { bytes: &'a [u8], pos: usize }
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self.pos.checked_add(N).ok_or("journal offset overflow")?;
        let slice = self.bytes.get(self.pos..end).ok_or("truncated journal")?;
        let value = slice.try_into().map_err(|_| "invalid journal field length")?;
        self.pos = end; Ok(value)
    }
    fn position(&mut self) -> Result<TileCoord, String> {
        Ok(TileCoord::new(i64::from_le_bytes(self.take()?), i64::from_le_bytes(self.take()?)))
    }
}
