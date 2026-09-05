//! Deterministic foundations. No renderer, operating-system time, or global RNG.
#![forbid(unsafe_code)]

pub type Tick = u64;
/// Increment when a change intentionally alters generated worlds or simulation results.
pub const SIMULATION_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationClock {
    tick: Tick,
    ticks_per_second: u32,
}

impl SimulationClock {
    #[must_use]
    pub const fn new(ticks_per_second: u32) -> Self {
        assert!(ticks_per_second > 0, "ticks_per_second must be positive");
        Self { tick: 0, ticks_per_second }
    }
    #[must_use]
    pub const fn tick(self) -> Tick { self.tick }
    #[must_use]
    pub const fn ticks_per_second(self) -> u32 { self.ticks_per_second }
    /// Fail explicitly at the end of the time domain, never silently travel backwards.
    pub fn advance(&mut self) -> StepContext {
        let context = StepContext { tick: self.tick, ticks_per_second: self.ticks_per_second };
        self.tick = self.tick.checked_add(1).expect("simulation clock exhausted");
        context
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepContext {
    pub tick: Tick,
    pub ticks_per_second: u32,
}
impl StepContext {
    #[must_use]
    pub const fn every(self, period_in_ticks: Tick) -> bool {
        period_in_ticks != 0 && self.tick % period_in_ticks == 0
    }
}

/// Domain-separated tuple hashing. This is simulation randomness, not cryptography.
#[must_use]
pub const fn keyed_u64(seed: u64, system: u64, tick: Tick, subject: StableId, sample: u64) -> u64 {
    let a = splitmix64(seed ^ 0x5365_6564_4c50_5732);
    let b = splitmix64(a ^ system);
    let c = splitmix64(b ^ tick);
    let d = splitmix64(c ^ subject.0);
    splitmix64(d ^ sample)
}
#[must_use]
pub fn keyed_unit_f32(seed: u64, system: u64, tick: Tick, subject: StableId, sample: u64) -> f32 {
    let value = keyed_u64(seed, system, tick, subject, sample) >> 40;
    value as f32 / (1_u32 << 24) as f32
}
#[must_use]
pub const fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

/// Streaming FNV-1a for reproducibility and accidental corruption detection, not authentication.
#[derive(Debug, Clone, Copy)]
pub struct Checksum64(u64);
impl Default for Checksum64 { fn default() -> Self { Self::new() } }
impl Checksum64 {
    #[must_use]
    pub const fn new() -> Self { Self(0xcbf2_9ce4_8422_2325) }
    pub fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01B3);
        }
    }
    #[must_use]
    pub const fn finish(self) -> u64 { self.0 }
}
#[must_use]
pub fn checksum64(bytes: impl IntoIterator<Item = u8>) -> u64 {
    let mut state = Checksum64::new();
    for byte in bytes { state.write(&[byte]); }
    state.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_checksum_vector() {
        assert_eq!(checksum64(*b"hello"), 0xa430_d846_80aa_bd0b);
    }
    #[test]
    fn streamed_checksum_matches() {
        let mut sum = Checksum64::new(); sum.write(b"he"); sum.write(b"llo");
        assert_eq!(sum.finish(), checksum64(*b"hello"));
    }
    #[test]
    fn random_is_call_order_independent() {
        let a = keyed_u64(7, 11, 13, StableId(17), 19);
        let _ = keyed_u64(99, 88, 77, StableId(66), 55);
        assert_eq!(a, keyed_u64(7, 11, 13, StableId(17), 19));
    }
    #[test]
    fn tuple_fields_do_not_cancel_like_xor_combiner() {
        assert_ne!(keyed_u64(0, 1, 0, StableId(0), 0),
            keyed_u64(1_u64.rotate_left(11), 0, 0, StableId(0), 0));
    }
    #[test]
    fn clock_and_schedule() {
        let mut clock = SimulationClock::new(60);
        assert!(clock.advance().every(6));
        assert!(!clock.advance().every(6));
        assert_eq!(clock.tick(), 2);
        assert!(!clock.advance().every(0));
    }
    #[test]
    #[should_panic(expected = "simulation clock exhausted")]
    fn clock_cannot_wrap() {
        let mut clock = SimulationClock { tick: u64::MAX, ticks_per_second: 60 };
        clock.advance();
    }
}
