use std::hash::Hash;

use crate::palette::AbstractCube;

#[derive(Clone)]
pub enum PaletteStorage<V, const DIM: usize> {
    Dense(Box<AbstractCube<V, DIM>>),
    Indexed(Box<AbstractCube<u8, DIM>>),
}

#[derive(Clone)]
pub struct HeterogeneousPaletteData<V: Hash + Eq + Copy, const DIM: usize> {
    pub storage: PaletteStorage<V, DIM>,
    pub palette: Vec<V>,
    pub counts: Vec<u16>,
}

impl<V: Hash + Eq + Copy + Default, const DIM: usize> HeterogeneousPaletteData<V, DIM> {
    pub fn get(&self, x: usize, y: usize, z: usize) -> V {
        debug_assert!(x < DIM);
        debug_assert!(y < DIM);
        debug_assert!(z < DIM);

        match &self.storage {
            PaletteStorage::Dense(cube) => cube[y][z][x],
            PaletteStorage::Indexed(indices) => self.palette[indices[y][z][x] as usize],
        }
    }

    /// Returns the Original
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: V) -> V {
        debug_assert!(x < DIM);
        debug_assert!(y < DIM);
        debug_assert!(z < DIM);

        let original = self.get(x, y, z);
        if original == value {
            return original;
        }

        let original_index = self.palette.iter().position(|v| v == &original).unwrap();

        // Find or add the new value to the palette.
        let new_index = if let Some(new_index) = self.palette.iter().position(|v| v == &value) {
            self.counts[new_index] += 1;
            new_index
        } else {
            self.palette.push(value);
            self.counts.push(1);
            self.palette.len() - 1
        };

        // Handle storage upgrades or updates
        let mut upgraded = false;
        match &mut self.storage {
            PaletteStorage::Dense(cube) => {
                cube[y][z][x] = value;
            }
            PaletteStorage::Indexed(indices) => {
                if new_index <= 255 {
                    indices[y][z][x] = new_index as u8;
                } else {
                    // Upgrade to Dense
                    let mut cube = Box::new([[[V::default(); DIM]; DIM]; DIM]);
                    for (i, v) in cube
                        .as_flattened_mut()
                        .as_flattened_mut()
                        .iter_mut()
                        .enumerate()
                    {
                        let y = i / (DIM * DIM);
                        let z = (i / DIM) % DIM;
                        let x = i % DIM;
                        *v = self.palette[indices[y][z][x] as usize];
                    }
                    cube[y][z][x] = value;
                    self.storage = PaletteStorage::Dense(cube);
                    upgraded = true;
                }
            }
        }

        self.counts[original_index] -= 1;

        if self.counts[original_index] == 0 {
            let last_index = self.palette.len() - 1;
            // Remove from palette and counts Vecs if the count hits zero.
            self.palette.swap_remove(original_index);
            self.counts.swap_remove(original_index);

            // If we are indexed, we need to update all indices because swap_remove changed indices
            if !upgraded && let PaletteStorage::Indexed(indices) = &mut self.storage {
                for row in indices.iter_mut() {
                    for col in row.iter_mut() {
                        for idx in col.iter_mut() {
                            if *idx as usize == last_index {
                                *idx = original_index as u8;
                            }
                        }
                    }
                }
            }
        }

        original
    }
}
