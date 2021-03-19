#[cfg(test)]
use crate::assert_size_of_val_eq;
use crate::{MemoryUsage, MemoryUsageTracker, POINTER_BYTE_SIZE};
use std::cell::UnsafeCell;

// Cell types.
impl<T> MemoryUsage for UnsafeCell<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        if tracker.track(self.get() as *const ()) {
            POINTER_BYTE_SIZE
        } else {
            0
        }
    }
}

#[cfg(test)]
mod test_cell_types {
    use super::*;

    #[test]
    fn test_unsafecell() {
        let cell = UnsafeCell::<i8>::new(1);
        assert_size_of_val_eq!(cell, POINTER_BYTE_SIZE);
    }
}
