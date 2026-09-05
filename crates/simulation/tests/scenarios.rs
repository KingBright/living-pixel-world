use simulation::{Command, ReplayLimits, Simulation, WorldSpec};
use world_model::{ChunkCoord, TileCoord};
fn canal() -> Simulation { Simulation::new(WorldSpec::canal(42)).unwrap() }
fn open_gate(sim: &mut Simulation) {
    for y in 30..=34 {
        sim.command(Command::SetElevation { at: TileCoord::new(64, y), decimetres: 0 }).unwrap();
    }
}
fn downstream(sim: &Simulation) -> u64 {
    sim.world().chunk(ChunkCoord::new(1, 0)).unwrap().tiles().iter().map(|t| u64::from(t.water_mm)).sum()
}
#[test]
fn closed_dam_holds_and_open_gate_releases_real_water() {
    let mut sim = canal(); let initial = sim.world().water_litres();
    assert_eq!(initial, 160_000); sim.advance(120).unwrap();
    assert_eq!(downstream(&sim), 0);
    open_gate(&mut sim); sim.advance(600).unwrap();
    assert!(downstream(&sim) > 0);
    // Independent reference calculation for simulation version 2; native execution is required.
    assert_eq!(downstream(&sim), 4760);
    assert_eq!(sim.world().water_litres(), initial);
    assert_eq!(sim.ledger().residual(sim.world().water_litres()), 0);
}
#[test]
fn rain_evaporation_and_pour_are_accounted_by_actual_amounts() {
    let mut sim = canal(); sim.command(Command::Rain { millimetres: 10 }).unwrap();
    assert_eq!(sim.ledger().input_litres, 81920);
    sim.command(Command::Evaporate { max_mm: u32::MAX }).unwrap();
    assert_eq!(sim.world().water_litres(), 0);
    sim.command(Command::Pour { at: TileCoord::new(60, 32), millimetres: 1700 }).unwrap();
    sim.advance(60).unwrap(); assert_eq!(sim.world().water_litres(), 1700);
    assert_eq!(sim.ledger().residual(sim.world().water_litres()), 0);
}
#[test]
fn rejected_command_changes_neither_state_nor_log() {
    let mut sim = canal(); let before = sim.to_journal();
    assert!(sim.command(Command::Rain { millimetres: u32::MAX }).is_err());
    assert_eq!(sim.to_journal(), before);
    assert!(sim.command(Command::Pour { at: TileCoord::new(-1000, 0), millimetres: 1 }).is_err());
    assert_eq!(sim.to_journal(), before);
}
#[test]
fn batching_ticks_does_not_change_the_result() {
    let mut a = canal(); let mut b = canal(); open_gate(&mut a); open_gate(&mut b);
    a.advance(137).unwrap();
    for _ in 0..137 { b.advance(1).unwrap(); }
    assert_eq!(a.checksum(), b.checksum()); assert_eq!(a.to_journal(), b.to_journal());
}
#[test]
fn snapshot_free_journal_restores_state_and_same_tick_command_order() {
    let mut a = canal(); a.advance(13).unwrap();
    a.command(Command::Pour { at: TileCoord::new(61, 32), millimetres: 1200 }).unwrap();
    a.command(Command::Evaporate { max_mm: 3 }).unwrap(); open_gate(&mut a);
    a.advance(127).unwrap();
    // A command at the final tick must be restored even without a following advance.
    a.command(Command::Rain { millimetres: 2 }).unwrap();
    let data = a.to_journal(); let b = Simulation::from_journal(&data, ReplayLimits::default()).unwrap();
    assert_eq!(a.checksum(), b.checksum()); assert_eq!(data, b.to_journal());
}
#[test]
fn bad_files_and_unsupported_versions_are_rejected() {
    let sim = canal(); let data = sim.to_journal();
    for end in 0..data.len() { assert!(Simulation::from_journal(&data[..end], ReplayLimits::default()).is_err()); }
    let mut corrupt = data.clone(); corrupt[20] ^= 1;
    assert!(Simulation::from_journal(&corrupt, ReplayLimits::default()).is_err());
    let mut wrong_version = data; wrong_version[8..12].copy_from_slice(&999_u32.to_le_bytes());
    let len = wrong_version.len(); let hash = sim_core::checksum64(wrong_version[..len-8].iter().copied());
    wrong_version[len-8..].copy_from_slice(&hash.to_le_bytes());
    let error = Simulation::from_journal(&wrong_version, ReplayLimits::default()).unwrap_err();
    assert!(error.contains("version"));
}
#[test]
fn excessive_replay_cost_is_rejected_before_reconstruction() {
    let mut sim = canal(); sim.advance(60).unwrap();
    assert!(Simulation::from_journal(&sim.to_journal(), ReplayLimits { max_ticks: 59, max_cell_updates: u64::MAX }).is_err());
    assert!(Simulation::from_journal(&sim.to_journal(), ReplayLimits { max_ticks: 60, max_cell_updates: 1 }).is_err());
}
#[test]
fn world_order_is_canonical_but_duplicate_chunks_are_rejected() {
    let mut spec = WorldSpec::terrain(44, 1); spec.chunks.reverse();
    let a = Simulation::new(spec).unwrap(); let b = Simulation::new(WorldSpec::terrain(44, 1)).unwrap();
    assert_eq!(a.checksum(), b.checksum());
    let mut spec = WorldSpec::terrain(44, 0); spec.chunks.push(ChunkCoord::new(0, 0));
    assert!(Simulation::new(spec).is_err());
}
#[test]
fn clock_is_part_of_the_canonical_state_hash() {
    let mut sim = canal(); let before = sim.checksum(); sim.advance(1).unwrap();
    assert_ne!(sim.checksum(), before);
}

#[test]
fn zero_tick_grid_commands_cannot_bypass_replay_work_budget() {
    let mut sim = canal();
    for _ in 0..10 { sim.command(Command::Rain { millimetres: 1 }).unwrap(); }
    let limits = ReplayLimits { max_ticks: 0, max_cell_updates: 8192 * 10 };
    assert!(sim.check_replay_budget(limits).is_err());
    assert!(Simulation::from_journal(&sim.to_journal(), limits).is_err());
}
