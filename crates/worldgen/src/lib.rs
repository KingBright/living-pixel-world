//! Deterministic first-pass terrain, climate and biome generation.
//!
//! This is intentionally small. The production pipeline will add continental plates,
//! erosion, sediment, soils, resources and settlement history.
//! Regional drainage analysis is provided by `drainage`.

#![forbid(unsafe_code)]

pub mod drainage;

use sim_core::splitmix64;
use world_model::{Biome, CHUNK_SIDE, Chunk, ChunkCoord, TileFields};

const Q16_ONE: i64 = 65_535;
const SEA_LEVEL_Q16: i64 = 27_000;

#[derive(Debug, Clone, Copy)]
pub struct WorldGenerator {
    seed: u64,
}

impl WorldGenerator {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { seed }
    }

    #[must_use]
    pub fn generate_chunk(self, coord: ChunkCoord) -> Chunk {
        let mut chunk = Chunk::empty(coord);
        let origin_x = i64::from(coord.x) * CHUNK_SIDE as i64;
        let origin_y = i64::from(coord.y) * CHUNK_SIDE as i64;

        for local_y in 0..CHUNK_SIDE {
            for local_x in 0..CHUNK_SIDE {
                let world_x = origin_x + local_x as i64;
                let world_y = origin_y + local_y as i64;
                let tile = self.generate_tile(world_x, world_y);
                *chunk.tile_mut(local_x, local_y) = tile;
            }
        }

        // Generation itself is the baseline state, not a player-visible mutation.
        chunk.revision = 0;
        chunk
    }

    #[must_use]
    pub fn generate_tile(self, x: i64, y: i64) -> TileFields {
        assert!((-137_438_953_472..=137_438_953_471).contains(&x), "x outside chunk coordinate domain");
        assert!((-137_438_953_472..=137_438_953_471).contains(&y), "y outside chunk coordinate domain");
        let continental = fbm_q16(self.seed ^ 0xC011_71E3, x, y, 1_024, 5);
        let ridges = ridge_q16(fbm_q16(self.seed ^ 0xA17E_51D5, x, y, 256, 4));
        let detail = fbm_q16(self.seed ^ 0xD37A_11ED, x, y, 64, 3);
        let elevation_q16 = clamp_q16((continental * 7 + ridges * 2 + detail) / 10);

        let latitude = ((y.unsigned_abs() % 16_384) as i64 * Q16_ONE / 16_384).min(Q16_ONE);
        let base_temperature_q16 = Q16_ONE - latitude;
        let altitude_cooling_q16 = ((elevation_q16 - SEA_LEVEL_Q16).max(0) * 2 / 3).min(Q16_ONE);
        let temperature_q16 = clamp_q16(base_temperature_q16 - altitude_cooling_q16 / 2);

        let ocean_influence = fbm_q16(self.seed ^ 0x0CEA_0001, x, y, 512, 4);
        let rain_shadow = ridge_q16(fbm_q16(self.seed ^ 0x51AD_0007, x.saturating_add(311), y.saturating_sub(197), 384, 3));
        let moisture_q16 = clamp_q16(ocean_influence - rain_shadow / 4 + 8_000);
        let fertility_q16 = clamp_q16((moisture_q16 * 3 + (Q16_ONE - ridges) * 2) / 5);

        let is_ocean = elevation_q16 < SEA_LEVEL_Q16;
        let biome = classify_biome(
            is_ocean,
            elevation_q16,
            temperature_q16,
            moisture_q16,
        );

        let elevation_dm = (((elevation_q16 - SEA_LEVEL_Q16) * 3_000) / Q16_ONE)
            .clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
        let temperature_d_c = (((temperature_q16 * 500) / Q16_ONE) - 150)
            .clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16;
        // Keep ocean surfaces exactly at the datum after elevation quantization.
        let water_mm = if is_ocean {
            u32::try_from(-i32::from(elevation_dm) * 100).unwrap_or(0)
        } else { 0 };
        let plant_biomass_g_m2 = initial_biomass(biome, moisture_q16, temperature_q16);

        TileFields {
            elevation_dm,
            water_mm,
            soil_moisture_q16: moisture_q16 as u16,
            temperature_d_c,
            fertility_q16: fertility_q16 as u16,
            plant_biomass_g_m2,
            biome,
            flags: 0,
        }
    }
}

#[must_use]
fn classify_biome(
    is_ocean: bool,
    elevation_q16: i64,
    temperature_q16: i64,
    moisture_q16: i64,
) -> Biome {
    if is_ocean {
        return Biome::Ocean;
    }
    if elevation_q16 > 56_000 {
        return Biome::Alpine;
    }
    if temperature_q16 < 12_000 {
        return Biome::Tundra;
    }
    if temperature_q16 < 24_000 {
        return if moisture_q16 > 34_000 {
            Biome::Taiga
        } else {
            Biome::Grassland
        };
    }
    if moisture_q16 < 15_000 {
        return Biome::Desert;
    }
    if moisture_q16 > 52_000 && elevation_q16 < 36_000 {
        return Biome::Wetland;
    }
    if moisture_q16 > 31_000 {
        Biome::TemperateForest
    } else {
        Biome::Grassland
    }
}

#[must_use]
fn initial_biomass(biome: Biome, moisture_q16: i64, temperature_q16: i64) -> u16 {
    let biome_capacity = match biome {
        Biome::Ocean | Biome::Alpine | Biome::Barren => 0,
        Biome::Tundra => 700,
        Biome::Taiga => 5_000,
        Biome::TemperateForest => 14_000,
        Biome::Grassland => 4_500,
        Biome::Wetland => 9_000,
        Biome::Desert => 350,
    };
    let climate_factor = moisture_q16.min(temperature_q16).clamp(0, Q16_ONE);
    ((i64::from(biome_capacity) * climate_factor) / Q16_ONE)
        .clamp(0, i64::from(u16::MAX)) as u16
}

/// Fractal value noise implemented in fixed point so generation does not depend on libm.
#[must_use]
fn fbm_q16(seed: u64, x: i64, y: i64, base_scale: i64, octaves: u32) -> i64 {
    let mut scale = base_scale.max(1);
    let mut amplitude = 32_768_i64;
    let mut weighted_sum = 0_i64;
    let mut weight = 0_i64;

    for octave in 0..octaves {
        let sample = value_noise_q16(seed ^ (u64::from(octave) * 0x9E37_79B9), x, y, scale);
        weighted_sum += sample * amplitude;
        weight += amplitude;
        scale = (scale / 2).max(1);
        amplitude = (amplitude / 2).max(1);
    }

    clamp_q16(weighted_sum / weight.max(1))
}

#[must_use]
fn ridge_q16(value: i64) -> i64 {
    let distance_from_mid = (value - Q16_ONE / 2).abs() * 2;
    clamp_q16(Q16_ONE - distance_from_mid)
}

#[must_use]
fn value_noise_q16(seed: u64, x: i64, y: i64, scale: i64) -> i64 {
    let gx = x.div_euclid(scale);
    let gy = y.div_euclid(scale);
    let fx = x.rem_euclid(scale) * Q16_ONE / scale;
    let fy = y.rem_euclid(scale) * Q16_ONE / scale;
    let sx = smoothstep_q16(fx);
    let sy = smoothstep_q16(fy);

    let n00 = lattice_q16(seed, gx, gy);
    let n10 = lattice_q16(seed, gx + 1, gy);
    let n01 = lattice_q16(seed, gx, gy + 1);
    let n11 = lattice_q16(seed, gx + 1, gy + 1);

    let top = lerp_q16(n00, n10, sx);
    let bottom = lerp_q16(n01, n11, sx);
    lerp_q16(top, bottom, sy)
}

#[must_use]
fn lattice_q16(seed: u64, x: i64, y: i64) -> i64 {
    let x_bits = u64::from_le_bytes(x.to_le_bytes());
    let y_bits = u64::from_le_bytes(y.to_le_bytes());
    (splitmix64(seed ^ x_bits.rotate_left(17) ^ y_bits.rotate_left(41)) >> 48) as i64
}

#[must_use]
fn smoothstep_q16(t: i64) -> i64 {
    let t = clamp_q16(t);
    let t2 = t * t / Q16_ONE;
    let factor = 3 * Q16_ONE - 2 * t;
    clamp_q16(t2 * factor / Q16_ONE)
}

#[must_use]
fn lerp_q16(a: i64, b: i64, t: i64) -> i64 {
    a + (b - a) * clamp_q16(t) / Q16_ONE
}

#[must_use]
const fn clamp_q16(value: i64) -> i64 {
    if value < 0 {
        0
    } else if value > Q16_ONE {
        Q16_ONE
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_same_chunk() {
        let coord = ChunkCoord::new(-2, 7);
        let a = WorldGenerator::new(123).generate_chunk(coord);
        let b = WorldGenerator::new(123).generate_chunk(coord);
        assert_eq!(a.checksum(), b.checksum());
    }

    #[test]
    fn different_seed_changes_chunk() {
        let coord = ChunkCoord::new(3, -4);
        let a = WorldGenerator::new(123).generate_chunk(coord);
        let b = WorldGenerator::new(124).generate_chunk(coord);
        assert_ne!(a.checksum(), b.checksum());
    }

    #[test]
    fn adjacent_chunks_share_continuous_noise_domain() {
        let left = WorldGenerator::new(42).generate_chunk(ChunkCoord::new(0, 0));
        let right = WorldGenerator::new(42).generate_chunk(ChunkCoord::new(1, 0));
        // This is not an equality test: it protects against accidentally re-seeding each chunk.
        let edge_delta = (i32::from(left.tile(63, 32).elevation_dm)
            - i32::from(right.tile(0, 32).elevation_dm))
        .abs();
        assert!(edge_delta < 1_000, "chunk boundary jumped by {edge_delta} dm");
    }
    #[test]
    fn tile_queries_match_chunks_across_negative_coordinates() {
        let generator = WorldGenerator::new(19);
        for coord in [ChunkCoord::new(-1, -1), ChunkCoord::new(0, 0)] {
            let chunk = generator.generate_chunk(coord);
            for (x, y) in [(0, 0), (63, 63), (17, 31)] {
                let at = coord.tile(x, y);
                assert_eq!(*chunk.tile(x, y), generator.generate_tile(at.x, at.y));
            }
        }
    }
    #[test]
    fn generated_ocean_surface_uses_the_elevation_datum() {
        let mut ocean_samples = 0;
        for seed in 0..16 {
            let generator = WorldGenerator::new(seed);
            for x in -16..=16 {
                let tile = generator.generate_tile(x * 1024, 0);
                if tile.biome == Biome::Ocean {
                    ocean_samples += 1;
                    assert_eq!(tile.surface_mm(), 0);
                }
            }
        }
        assert!(ocean_samples > 0, "regression did not exercise an ocean tile");
    }

}
