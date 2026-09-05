//! Diagnostic CPU pixels. Not the production art pipeline or a windowing backend.
use std::{fs::OpenOptions, io::Write, path::Path};
use world_model::{ActiveWorld, Biome, TileCoord, TileFields};
use worldgen::{WorldGenerator, drainage::Drainage};

type Rgb = [u8; 3];
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)
        .map_err(|e| format!("create {} (existing files are not overwritten): {e}", path.display()))?;
    file.write_all(bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    file.sync_all().map_err(|e| format!("flush {}: {e}", path.display()))
}

pub fn world_bmp(world: &ActiveWorld) -> Result<Vec<u8>, String> {
    let min_x = world.chunks().map(|c| i64::from(c.coord.x) * 64).min().ok_or("empty world")?;
    let min_y = world.chunks().map(|c| i64::from(c.coord.y) * 64).min().ok_or("empty world")?;
    let max_x = world.chunks().map(|c| (i64::from(c.coord.x) + 1) * 64).max().ok_or("empty world")?;
    let max_y = world.chunks().map(|c| (i64::from(c.coord.y) + 1) * 64).max().ok_or("empty world")?;
    let width = usize::try_from(max_x - min_x).map_err(|_| "image width overflow")?;
    let height = usize::try_from(max_y - min_y).map_err(|_| "image height overflow")?;
    if width > 512 || height > 512 { return Err("diagnostic view exceeds 512x512 tiles".into()); }
    let scale = 4; let mut pixels = Vec::with_capacity(width * height * scale * scale);
    for py in 0..height * scale {
        for px in 0..width * scale {
            let pos = TileCoord::new(min_x + (px / scale) as i64, min_y + (py / scale) as i64);
            let mut rgb = world.tile(pos).map_or([18, 24, 30], |&tile| tile_colour(tile));
            if px % scale == 0 || py % scale == 0 {
                for c in &mut rgb { *c = (*c).saturating_sub(9); }
            }
            pixels.push(rgb);
        }
    }
    encode_bmp(width * scale, height * scale, &pixels)
}
fn tile_colour(tile: TileFields) -> Rgb {
    if tile.water_mm > 0 {
        let depth = (tile.water_mm / 10).min(100) as u8;
        return [35, 155_u8.saturating_sub(depth), 210_u8.saturating_sub(depth / 2)];
    }
    if tile.elevation_dm >= 100 && tile.biome == Biome::Grassland { return [134, 151, 97]; }
    match tile.biome {
        Biome::Ocean => [38, 94, 143], Biome::Tundra => [173, 189, 179],
        Biome::Taiga => [53, 92, 78], Biome::TemperateForest => [51, 108, 58],
        Biome::Grassland => [122, 155, 77], Biome::Wetland => [118, 105, 74],
        Biome::Desert => [207, 182, 112], Biome::Alpine => [191, 194, 190], Biome::Barren => [116, 105, 92],
    }
}

/// 8 km regional analysis at 32 m sampling, not 62,500 active gameplay tiles.
pub fn atlas_bmp(seed: u64) -> Result<(Vec<u8>, String), String> {
    const SIDE: usize = 250;
    let generator = WorldGenerator::new(seed);
    let mut tiles = Vec::with_capacity(SIDE * SIDE);
    for y in 0..SIDE { for x in 0..SIDE {
        tiles.push(generator.generate_tile(x as i64 * 32 - 4000, y as i64 * 32 - 4000));
    }}
    let elevation: Vec<i32> = tiles.iter().map(|t| i32::from(t.elevation_dm) * 100).collect();
    // Dimensionless moisture-weighted local runoff proxy, explicitly not cubic metres/second.
    let runoff: Vec<u32> = tiles.iter().map(|t| 1 + u32::from(t.soil_moisture_q16) / 10_000).collect();
    let map = Drainage::build(SIDE, SIDE, &elevation, &runoff)?;
    if map.outlet_runoff() != map.total_runoff { return Err("drainage runoff accounting failed".into()); }
    let mut pixels = Vec::with_capacity(SIDE * SIDE * 16);
    for y in 0..SIDE * 4 { for x in 0..SIDE * 4 {
        let i = y / 4 * SIDE + x / 4;
        let colour = if map.filled_elevation_mm[i] > elevation[i] { [68, 124, 155] }
            else if map.is_river(i, 100) { [94, 178, 214] }
            else { tile_colour(tiles[i]) };
        pixels.push(colour);
    }}
    let info = format!("regional_cells={} runoff_units={} outlet_units={} river_cells={} virtual_lake_cells={}",
        SIDE * SIDE, map.total_runoff, map.outlet_runoff(),
        (0..SIDE * SIDE).filter(|&i| map.is_river(i, 100)).count(),
        (0..SIDE * SIDE).filter(|&i| map.filled_elevation_mm[i] > elevation[i]).count());
    Ok((encode_bmp(SIDE * 4, SIDE * 4, &pixels)?, info))
}

/// Uncompressed 24-bit BMP with explicit dimensions, BGR order and row padding.
fn encode_bmp(width: usize, height: usize, pixels: &[Rgb]) -> Result<Vec<u8>, String> {
    if width == 0 || height == 0 || width > 2048 || height > 2048 || pixels.len() != width * height {
        return Err("invalid diagnostic bitmap dimensions".into());
    }
    let stride = (width * 3 + 3) & !3; let image_size = stride * height;
    let mut out = Vec::with_capacity(54 + image_size); out.extend_from_slice(b"BM");
    out.extend_from_slice(&((54 + image_size) as u32).to_le_bytes()); out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54_u32.to_le_bytes()); out.extend_from_slice(&40_u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes()); out.extend_from_slice(&(height as i32).to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); out.extend_from_slice(&24_u16.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes()); out.extend_from_slice(&(image_size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 16]);
    for y in (0..height).rev() {
        for x in 0..width {
            let [r, g, b] = pixels[y * width + x]; out.extend_from_slice(&[b, g, r]);
        }
        out.resize(out.len() + stride - width * 3, 0);
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bmp_header_padding_and_pixel_orientation() {
        let bmp = encode_bmp(1, 2, &[[1, 2, 3], [4, 5, 6]]).unwrap();
        assert_eq!(&bmp[..2], b"BM"); assert_eq!(bmp.len(), 62);
        assert_eq!(&bmp[54..], &[6, 5, 4, 0, 3, 2, 1, 0]);
        assert_eq!(u32::from_le_bytes(bmp[2..6].try_into().unwrap()) as usize, bmp.len());
    }
    #[test]
    fn bad_image_dimensions_rejected() {
        assert!(encode_bmp(0, 1, &[]).is_err()); assert!(encode_bmp(1, 1, &[]).is_err());
        assert!(encode_bmp(usize::MAX, 1, &[]).is_err());
    }
}
