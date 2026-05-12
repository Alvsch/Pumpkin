#[derive(Default, Clone, Copy)]
pub struct RandomTickSectionCache {
    pub random_ticking_block_count: u16,
    pub random_ticking_fluid_count: u16,
}

impl RandomTickSectionCache {
    #[must_use]
    pub const fn is_randomly_ticking(self) -> bool {
        self.random_ticking_block_count > 0 || self.random_ticking_fluid_count > 0
    }
}
