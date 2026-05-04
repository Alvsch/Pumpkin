/// Trait for extracting smelting experience from cooking block entities.
/// This is a separate dyn-compatible trait since `CookingBlockEntityBase` is not.
pub trait ExperienceContainer: Send + Sync {
    /// Extract and reset accumulated experience, returning the total as an integer
    fn extract_experience(&self) -> i32;
}
