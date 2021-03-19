#[cfg(test)]
use crate::assert_size_of_val_eq;
use crate::{MemoryUsage, MemoryUsageTracker, POINTER_BYTE_SIZE};
use std::mem;
use std::ptr::NonNull;

// Pointer types.
impl<T> MemoryUsage for *const T {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        if tracker.track(*self as *const ()) {
            POINTER_BYTE_SIZE
        } else {
            0
        }
    }
}

impl<T> MemoryUsage for *mut T {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        if tracker.track(*self as *const _ as *const ()) {
            POINTER_BYTE_SIZE
        } else {
            0
        }
    }
}

impl<T> MemoryUsage for NonNull<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        if tracker.track(self.as_ptr() as *const _ as *const ()) {
            POINTER_BYTE_SIZE
        } else {
            0
        }
    }
}

#[cfg(test)]
mod test_pointer_types {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_pointer() {
        let mut tracker = BTreeSet::new();

        let x = 1i8;
        let ptr = &x as *const _;
        assert_size_of_val_eq!(ptr, POINTER_BYTE_SIZE, &mut tracker);

        let ptr = &x as *const _;
        assert_size_of_val_eq!(ptr, 0, &mut tracker);
    }

    #[test]
    fn test_mutable_pointer() {
        let mut tracker = BTreeSet::new();

        let mut x = 1i8;
        let ptr = &mut x as *mut _;
        assert_size_of_val_eq!(ptr, POINTER_BYTE_SIZE, &mut tracker);

        let ptr = &mut x as *mut _;
        assert_size_of_val_eq!(ptr, 0, &mut tracker);
    }

    #[test]
    fn test_nonnull_pointer() {
        let mut x = 1i8;
        let ptr = NonNull::new(&mut x as *mut _).unwrap();
        assert_size_of_val_eq!(ptr, POINTER_BYTE_SIZE);
    }
}

// Reference types.
impl<T: MemoryUsage> MemoryUsage for &T {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + if tracker.track(*self as *const T as *const ()) {
                (*self).size_of_val(tracker)
            } else {
                0
            }
    }
}

impl<T: MemoryUsage> MemoryUsage for &mut T {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + if tracker.track(*self as *const T as *const ()) {
                MemoryUsage::size_of_val(*self, tracker)
            } else {
                0
            }
    }
}

#[cfg(test)]
mod test_reference_types {
    use super::*;

    #[test]
    fn test_reference() {
        assert_size_of_val_eq!(&1i8, POINTER_BYTE_SIZE + 1);
        assert_size_of_val_eq!(&1i64, POINTER_BYTE_SIZE + 8);
    }

    #[test]
    fn test_mutable_reference() {
        assert_size_of_val_eq!(&mut 1i8, POINTER_BYTE_SIZE + 1);
        assert_size_of_val_eq!(&mut 1i64, POINTER_BYTE_SIZE + 8);
    }
}
