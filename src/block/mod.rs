use serde::{Deserialize, Serialize};

pub const SECTION_SIZE: usize = 4096;
pub const BIOME_SECTION_SIZE: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionSnapshot {
    pub palette: Vec<u32>,
    pub storage: Option<Vec<u64>>,
    pub bits_per_entry: u8,
    pub biome_palette: Vec<u32>,
    pub biome_storage: Option<Vec<u64>>,
    pub biome_bits_per_entry: u8,
}

impl SectionSnapshot {
    pub fn capture_from_states(states: &[u32; SECTION_SIZE], biomes: &[u32; BIOME_SECTION_SIZE]) -> Self {
        let mut palette = Vec::new();
        let mut indices = [0usize; SECTION_SIZE];
        for (i, &state) in states.iter().enumerate() {
            let pal_idx = match palette.iter().position(|&s| s == state) {
                Some(idx) => idx,
                None => {
                    palette.push(state);
                    palette.len() - 1
                }
            };
            indices[i] = pal_idx;
        }
        let bits = Self::section_states_bits(palette.len());
        let storage = if bits == 0 {
            None
        } else {
            let values_per_long = 64 / usize::from(bits);
            let words = SECTION_SIZE.div_ceil(values_per_long);
            let mut data = vec![0u64; words];
            for (idx, &pal_idx) in indices.iter().enumerate() {
                let word = idx / values_per_long;
                let bit = (idx - word * values_per_long) * usize::from(bits);
                data[word] |= (pal_idx as u64) << bit;
            }
            Some(data)
        };

        let mut biome_palette = Vec::new();
        let mut b_indices = [0usize; BIOME_SECTION_SIZE];
        for (i, &b) in biomes.iter().enumerate() {
            let pal_idx = match biome_palette.iter().position(|&x| x == b) {
                Some(idx) => idx,
                None => {
                    biome_palette.push(b);
                    biome_palette.len() - 1
                }
            };
            b_indices[i] = pal_idx;
        }
        let b_bits = Self::section_biomes_bits(biome_palette.len());
        let biome_storage = if b_bits == 0 {
            None
        } else {
            let values_per_long = 64 / usize::from(b_bits);
            let words = BIOME_SECTION_SIZE.div_ceil(values_per_long);
            let mut data = vec![0u64; words];
            for (idx, &pal_idx) in b_indices.iter().enumerate() {
                let word = idx / values_per_long;
                let bit = (idx - word * values_per_long) * usize::from(b_bits);
                data[word] |= (pal_idx as u64) << bit;
            }
            Some(data)
        };

        Self {
            palette,
            storage,
            bits_per_entry: bits,
            biome_palette,
            biome_storage,
            biome_bits_per_entry: b_bits,
        }
    }

    fn section_states_bits(palette_size: usize) -> u8 {
        if palette_size <= 1 {
            return 0;
        }
        let b = (usize::BITS - (palette_size - 1).leading_zeros()) as u8;
        b.max(4)
    }

    fn section_biomes_bits(palette_size: usize) -> u8 {
        if palette_size <= 1 {
            return 0;
        }
        let b = (usize::BITS - (palette_size - 1).leading_zeros()) as u8;
        b.max(3)
    }

    pub fn get_block(&self, local_idx: usize) -> u32 {
        let Some(storage) = &self.storage else {
            return self.palette.first().copied().unwrap_or(0);
        };
        let bits = usize::from(self.bits_per_entry);
        let values_per_long = 64 / bits;
        let word = local_idx / values_per_long;
        let bit = (local_idx - word * values_per_long) * bits;
        let mask = (1u64 << bits) - 1;
        let pal_idx = ((storage[word] >> bit) & mask) as usize;
        self.palette.get(pal_idx).copied().unwrap_or(0)
    }

    pub fn get_biome(&self, bx: usize, by: usize, bz: usize) -> u32 {
        let Some(storage) = &self.biome_storage else {
            return self.biome_palette.first().copied().unwrap_or(0);
        };
        let idx = (by << 4) | (bz << 2) | bx;
        let bits = usize::from(self.biome_bits_per_entry);
        let values_per_long = 64 / bits;
        let word = idx / values_per_long;
        let bit = (idx - word * values_per_long) * bits;
        let mask = (1u64 << bits) - 1;
        let pal_idx = ((storage[word] >> bit) & mask) as usize;
        self.biome_palette.get(pal_idx).copied().unwrap_or(0)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_diff_to(
        &self,
        live_blocks: &mut [u32; SECTION_SIZE],
        live_biomes: &mut [u32; BIOME_SECTION_SIZE],
        x_base: i32,
        y_base: i32,
        z_base: i32,
        restore_blocks: bool,
        restore_biomes: bool,
        changed_positions: &mut Vec<(i32, i32, i32)>,
        biome_changed_out: &mut bool,
    ) -> usize {
        let mut changed = 0;

        if restore_blocks {
            let mut idx = 0;
            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let want = self.get_block(idx);
                        if live_blocks[idx] != want {
                            live_blocks[idx] = want;
                            changed_positions.push((x_base + x, y_base + y, z_base + z));
                            changed += 1;
                        }
                        idx += 1;
                    }
                }
            }
        }

        if restore_biomes && !self.biome_palette.is_empty() {
            let mut b_idx = 0;
            for by in 0..4 {
                for bz in 0..4 {
                    for bx in 0..4 {
                        let want = self.get_biome(bx, by, bz);
                        if live_biomes[b_idx] != want {
                            live_biomes[b_idx] = want;
                            *biome_changed_out = true;
                        }
                        b_idx += 1;
                    }
                }
            }
        }

        changed
    }
}