#![forbid(unsafe_code)]
mod pixels;
use std::{env, fs, io::{self, BufRead, Read, Write}, path::Path, process::ExitCode, str::FromStr};
use sim_core::SIMULATION_VERSION;
use simulation::{Command, ReplayLimits, Simulation, WorldSpec};
use world_model::{ChunkCoord, TileCoord};

fn main() -> ExitCode {
    match run() { Ok(()) => ExitCode::SUCCESS, Err(e) => { eprintln!("error: {e}"); ExitCode::FAILURE } }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let kind = args.first().map_or("check", String::as_str);
    match kind {
        "check" => { if args.len() > 1 { return Err("usage: world_headless check".into()); } check() }
        "demo" => {
            if args.len() > 2 { return Err("usage: world_headless demo [new-output-directory]".into()); }
            demo(Path::new(args.get(1).map_or("out", String::as_str)))
        }
        "shell" => {
            if args.len() > 2 { return Err("usage: world_headless shell [seed]".into()); }
            shell(number_or(args.get(1), 42, "seed")?)
        }
        "atlas" => {
            if args.len() > 3 { return Err("usage: world_headless atlas [seed] [new-file.bmp]".into()); }
            let seed = number_or(args.get(1), 42, "seed")?;
            let path = Path::new(args.get(2).map_or("atlas.bmp", String::as_str));
            let (bytes, info) = pixels::atlas_bmp(seed)?; pixels::write_new(path, &bytes)?;
            println!("{info}\nimage={}", path.display()); Ok(())
        }
        "replay" => {
            if args.len() != 2 { return Err("usage: world_headless replay file.lpw".into()); }
            let sim = read_journal(Path::new(&args[1]))?; println!("{}", status(&sim)); Ok(())
        }
        "terrain" => {
            if args.len() > 2 { return Err("usage: world_headless terrain [seed]".into()); }
            let seed = number_or(args.get(1), 42, "seed")?;
            let mut sim = Simulation::new(WorldSpec::terrain(seed, 1))?;
            sim.command(Command::Rain { millimetres: 20 })?; sim.advance(120)?;
            if sim.ledger().residual(sim.world().water_litres()) != 0 { return Err("water balance failed".into()); }
            println!("{}", status(&sim)); Ok(())
        }
        "help" | "--help" | "-h" => { println!("world_headless check | demo [new-dir] | shell [seed] | atlas [seed] [new.bmp] | replay file.lpw | terrain [seed]"); Ok(()) }
        // Retain the original M0 positional-seed invocation.
        n if args.len() == 1 && n.parse::<u64>().is_ok() => {
            let sim = Simulation::new(WorldSpec::terrain(parse(n, "seed")?, 1))?;
            println!("{}", status(&sim)); Ok(())
        }
        _ => Err("unknown command; use --help".into()),
    }
}
fn open_gate(sim: &mut Simulation) -> Result<(), String> {
    for y in 30..=34 { sim.command(Command::SetElevation { at: TileCoord::new(64, y), decimetres: 0 })?; } Ok(())
}
fn downstream(sim: &Simulation) -> u64 {
    sim.world().chunk(ChunkCoord::new(1, 0)).map_or(0, |c| c.tiles().iter().map(|t| u64::from(t.water_mm)).sum())
}
fn checked_scenario() -> Result<Simulation, String> {
    let mut sim = Simulation::new(WorldSpec::canal(42))?; sim.advance(120)?;
    if downstream(&sim) != 0 { return Err("closed gate leaked".into()); }
    open_gate(&mut sim)?; sim.advance(600)?;
    if downstream(&sim) == 0 { return Err("open gate did not release water".into()); }
    if sim.world().water_litres() != 160_000 || sim.ledger().residual(sim.world().water_litres()) != 0 {
        return Err("canal water was created or lost".into());
    }
    Ok(sim)
}
fn check() -> Result<(), String> {
    let sim = checked_scenario()?;
    let replay = Simulation::from_journal(&sim.to_journal(), ReplayLimits::default())?;
    if sim.checksum() != replay.checksum() { return Err("replay diverged".into()); }
    println!("{}", status(&sim)); println!("acceptance: gate_hold=pass gate_release=pass water_balance=pass replay=pass"); Ok(())
}
fn demo(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("create output directory: {e}"))?;
    let before = Simulation::new(WorldSpec::canal(42))?;
    let after = checked_scenario()?;
    // Validate replay before writing success reports.
    Simulation::from_journal(&after.to_journal(), ReplayLimits::default())?;
    pixels::write_new(&dir.join("canal_before.bmp"), &pixels::world_bmp(before.world())?)?;
    pixels::write_new(&dir.join("canal_after.bmp"), &pixels::world_bmp(after.world())?)?;
    pixels::write_new(&dir.join("canal.lpw"), &after.to_journal())?;
    pixels::write_new(&dir.join("status.json"), status(&after).as_bytes())?;
    println!("{}\nartifacts={}", status(&after), dir.display()); Ok(())
}
fn status(sim: &Simulation) -> String {
    let ledger = sim.ledger();
    format!("{{\"simulation_version\":{},\"seed\":\"{}\",\"tick\":{},\"active_chunks\":{},\"water_litres\":{},\"input_litres\":{},\"output_litres\":{},\"residual_litres\":{},\"checksum\":\"{:016x}\"}}",
        SIMULATION_VERSION, sim.spec().seed, sim.tick(), sim.world().chunks().len(), sim.world().water_litres(),
        ledger.input_litres, ledger.output_litres, ledger.residual(sim.world().water_litres()), sim.checksum())
}
fn read_journal(path: &Path) -> Result<Simulation, String> {
    let file = fs::File::open(path).map_err(|e| format!("open journal: {e}"))?;
    let mut bytes = Vec::new(); file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| format!("read journal: {e}"))?;
    Simulation::from_journal(&bytes, ReplayLimits::default())
}
fn parse<T: FromStr>(s: &str, name: &str) -> Result<T, String> { s.parse().map_err(|_| format!("invalid {name}: {s}")) }
fn number_or<T: FromStr>(value: Option<&String>, default: T, name: &str) -> Result<T, String> {
    value.map_or(Ok(default), |s| parse(s, name))
}
fn shell(seed: u64) -> Result<(), String> {
    let mut sim = Simulation::new(WorldSpec::canal(seed))?;
    println!("Living Pixel World hydraulic inspector. Type help. No GUI window is opened.");
    println!("{}", status(&sim));
    let stdin = io::stdin(); let mut lines = stdin.lock().lines();
    loop {
        print!("lpw> "); io::stdout().flush().map_err(|e| e.to_string())?;
        let Some(line) = lines.next() else { break; }; let line = line.map_err(|e| e.to_string())?;
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() { continue; }
        if words == ["quit"] || words == ["exit"] { break; }
        match shell_command(&mut sim, &words) { Ok(message) => println!("{message}"), Err(e) => eprintln!("error: {e}") }
    }
    Ok(())
}
fn shell_command(sim: &mut Simulation, words: &[&str]) -> Result<String, String> {
    match words {
        ["help"] => return Ok("status | tick N | rain MM | evaporate MM | pour X Y MM | ground X Y DM | gate open|closed | inspect X Y | map NEW.bmp | save NEW.lpw | load FILE.lpw | quit".into()),
        ["status"] => {},
        ["tick", n] => sim.advance(parse(n, "tick count")?)?,
        ["rain", mm] => sim.command(Command::Rain { millimetres: parse(mm, "rain depth")? })?,
        ["evaporate", mm] => sim.command(Command::Evaporate { max_mm: parse(mm, "evaporation depth")? })?,
        ["pour", x, y, mm] => sim.command(Command::Pour { at: TileCoord::new(parse(x, "x")?, parse(y, "y")?), millimetres: parse(mm, "water depth")? })?,
        ["ground", x, y, dm] => sim.command(Command::SetElevation { at: TileCoord::new(parse(x, "x")?, parse(y, "y")?), decimetres: parse(dm, "elevation")? })?,
        ["gate", state] if *state == "open" || *state == "closed" => {
            if sim.spec().preset != simulation::Preset::Canal { return Err("gate helper only applies to the canal fixture".into()); }
            if simulation::MAX_COMMANDS - sim.commands().len() < 5 { return Err("journal has insufficient room for five gate edits".into()); }
            let decimetres = if *state == "open" { 0 } else { 20 };
            for y in 30..=34 { sim.command(Command::SetElevation { at: TileCoord::new(64, y), decimetres })?; }
        }
        ["inspect", x, y] => {
            let at = TileCoord::new(parse(x, "x")?, parse(y, "y")?);
            return sim.world().tile(at).map(|tile| format!("{at:?}: {tile:?}")).ok_or("tile is not active".into());
        }
        ["map", path] => pixels::write_new(Path::new(path), &pixels::world_bmp(sim.world())?)?,
        ["save", path] => {
            sim.check_replay_budget(ReplayLimits::default())?;
            pixels::write_new(Path::new(path), &sim.to_journal())?;
        }
        ["load", path] => { let loaded = read_journal(Path::new(path))?; *sim = loaded; }
        _ => return Err("unrecognised command or wrong argument count; type help".into()),
    }
    Ok(status(sim))
}
