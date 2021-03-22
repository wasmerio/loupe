mod memory_usage;

#[cfg(feature = "derive")]
pub use loupe_derive::*;
pub use memory_usage::*;

use std::collections::BTreeSet;

pub fn size_of_val<T: MemoryUsage>(value: &T) -> usize {
    <T as MemoryUsage>::size_of_val(value, &mut BTreeSet::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_of_val_helper() {
        assert_eq!(size_of_val(&"abc"), 2 * POINTER_BYTE_SIZE + 1 * 3);
    }
}
