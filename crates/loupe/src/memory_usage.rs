#[cfg(test)]
use std::collections::BTreeSet;
use std::mem;

pub const POINTER_BYTE_SIZE: usize = if cfg!(target_pointer_width = "16") {
    2
} else if cfg!(target_pointer_width = "32") {
    4
} else {
    8
};

pub trait MemoryUsageTracker {
    /// When first called on a given address returns true, else returns false.
    fn track(&mut self, address: *const ()) -> bool;
}

impl MemoryUsageTracker for std::collections::BTreeSet<*const ()> {
    fn track(&mut self, address: *const ()) -> bool {
        self.insert(address)
    }
}

impl MemoryUsageTracker for std::collections::HashSet<*const ()> {
    fn track(&mut self, address: *const ()) -> bool {
        self.insert(address)
    }
}

pub trait MemoryUsage {
    /// Returns the size of the referenced value in bytes.
    ///
    /// Recursively visits the value and any children returning the sum of their
    /// sizes. The size always includes any tail padding if applicable.
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize;
}

#[cfg(test)]
macro_rules! assert_size_of_val_eq {
    ($value:expr, $expected:expr) => {
        assert_eq!(
            MemoryUsage::size_of_val(&$value, &mut BTreeSet::new()),
            $expected
        );
    };
}

// Primitive types
macro_rules! impl_memory_usage_for_primitive {
    ( $type:ty ) => {
        impl MemoryUsage for $type {
            fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
                mem::size_of_val(self)
            }
        }
    };

    ( $( $type:ty ),+ ) => {
        $( impl_memory_usage_for_primitive!( $type ); )+
    }
}

impl_memory_usage_for_primitive!(
    bool, char, f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize
);

#[cfg(test)]
mod test_primitive_types {
    use super::*;

    macro_rules! test_memory_usage_for_primitive {
        ($test_name:ident: ($value:expr) == $expected:expr) => {
            #[test]
            fn $test_name() {
                assert_size_of_val_eq!($value, $expected);
            }
        };

        ( $( $test_name:ident: ($value:expr) == $expected:expr );+ $(;)* ) => {
            $( test_memory_usage_for_primitive!( $test_name: ($value) == $expected); )+
        }
    }

    test_memory_usage_for_primitive!(
        test_bool: (true) == 1;
        test_char: ('a') == 4;
        test_f32: (4.2f32) == 4;
        test_f64: (4.2f64) == 8;
        test_i8: (1i8) == 1;
        test_i16: (1i16) == 2;
        test_i32: (1i32) == 4;
        test_i64: (1i64) == 8;
        test_isize: (1isize) == POINTER_BYTE_SIZE;
        test_u8: (1u8) == 1;
        test_u16: (1u16) == 2;
        test_u32: (1u32) == 4;
        test_u64: (1u64) == 8;
        test_usize: (1usize) == POINTER_BYTE_SIZE;
    );
}

// pointer
// Pointers aren't necessarily safe to dereference, even if they're nonnull.

// Reference types.
impl<T: MemoryUsage> MemoryUsage for &T {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + if tracker.track(*self as *const T as *const ()) {
                MemoryUsage::size_of_val(*self, tracker)
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
    }

    #[test]
    fn test_mutable_reference() {
        assert_size_of_val_eq!(&mut 1i8, POINTER_BYTE_SIZE + 1);
    }
}

// slices
impl<T: MemoryUsage> MemoryUsage for [T] {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|v| MemoryUsage::size_of_val(v, tracker) - mem::size_of_val(v))
                .sum::<usize>()
    }
}

// arrays
impl<T: MemoryUsage, const N: usize> MemoryUsage for [T; N] {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|v| MemoryUsage::size_of_val(v, tracker) - mem::size_of_val(v))
                .sum::<usize>()
    }
}

// strs
/*
impl MemoryUsage for str {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        self.as_bytes().size_of()
    }
}
*/

// TODO: tuples

/// Standard library types

// TODO: Arc

//impl<T: MemoryUsage> MemoryUsage for Box<T> {
//}

// Cell

// Is a Pin always dereferenceable?
//impl<T: MemoryUsage> MemoryUsage for Pin<T> {
//}

// TODO: Mutex

// TODO: NonNull might be possible when '*const T' is MemoryUsage.

impl<T: MemoryUsage> MemoryUsage for Option<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|v| MemoryUsage::size_of_val(v, tracker))
                .sum::<usize>()
    }
}

// TODO: Rc

// TODO: Ref, RefCell, RefMut

//impl<T: MemoryUsage, E: MemoryUsage> MemoryUsage for Result<T, E> {
//}

// TODO: RwLock

// string?

// TODO: UnsafeCell

impl<T: MemoryUsage> MemoryUsage for Vec<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|v| MemoryUsage::size_of_val(v, tracker))
                .sum::<usize>()
    }
}

impl<T> MemoryUsage for std::marker::PhantomData<T> {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        0
    }
}

// TODO: PhantomPinned?

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[derive(Copy, Clone)]
    struct TestMemoryUsage {
        // Must be greater than or equal to mem::size_of::<TestMemoryUsage>() or else MemoryUsage may overflow.
        pub size_to_report: usize,
    }
    impl MemoryUsage for TestMemoryUsage {
        fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
            // Try to prevent buggy tests before they're hard to debug.
            assert!(self.size_to_report >= mem::size_of::<TestMemoryUsage>());
            self.size_to_report
        }
    }

    #[test]
    fn test_ints() {
        assert_eq!(MemoryUsage::size_of_val(&32, &mut BTreeSet::new()), 4);
        assert_eq!(32.size_of_val(&mut BTreeSet::new()), 4);
    }

    #[test]
    fn test_arrays() {
        let x: [[u8; 7]; 13] = [[0; 7]; 13];
        assert_eq!(7 * 13, mem::size_of_val(&x));
        assert_eq!(7 * 13, MemoryUsage::size_of_val(&x, &mut BTreeSet::new()));
    }

    #[test]
    fn test_slice_no_static_size() {
        {
            let x: [u8; 13] = [0; 13];
            let y: &[u8] = &x;
            assert_eq!(13, mem::size_of_val(y));
            assert_eq!(13, MemoryUsage::size_of_val(y, &mut BTreeSet::new()));
        }

        {
            let mut x: [TestMemoryUsage; 13] = [TestMemoryUsage {
                size_to_report: mem::size_of::<TestMemoryUsage>(),
            }; 13];
            x[0].size_to_report += 7;
            let y: &[TestMemoryUsage] = &x;
            assert_eq!(13 * mem::size_of::<TestMemoryUsage>(), mem::size_of_val(y));
            assert_eq!(
                13 * mem::size_of::<TestMemoryUsage>() + 7,
                MemoryUsage::size_of_val(y, &mut BTreeSet::new())
            );
        }
    }

    #[test]
    fn test_vecs() {
        let mut x = vec![];
        let empty_vec_size = mem::size_of_val(&x);
        let tmu_size = mem::size_of::<TestMemoryUsage>();
        x.push(TestMemoryUsage {
            size_to_report: tmu_size + 3,
        });
        x.push(TestMemoryUsage {
            size_to_report: tmu_size + 7,
        });
        assert_eq!(empty_vec_size, mem::size_of_val(&x));
        assert_eq!(
            empty_vec_size + 2 * tmu_size + 3 + 7,
            MemoryUsage::size_of_val(&x, &mut BTreeSet::new())
        );
    }

    #[test]
    fn test_double_counting() {
        let tmu_size = mem::size_of::<TestMemoryUsage>();
        let x = TestMemoryUsage {
            size_to_report: tmu_size + 7,
        };
        let y = TestMemoryUsage {
            size_to_report: tmu_size + 7,
        };
        let mut v = vec![];
        let empty_vec_size = mem::size_of_val(&v);
        v.push(&x);
        assert_eq!(
            empty_vec_size + 1 * mem::size_of_val(&x) + 1 * (tmu_size + 7),
            MemoryUsage::size_of_val(&v, &mut BTreeSet::new())
        );
        v.push(&x);
        assert_eq!(
            empty_vec_size + 2 * mem::size_of_val(&x) + 1 * (tmu_size + 7),
            MemoryUsage::size_of_val(&v, &mut BTreeSet::new())
        );
        v.push(&y);
        assert_eq!(mem::size_of_val(&x), mem::size_of_val(&y));
        assert_eq!(
            empty_vec_size + 3 * mem::size_of_val(&x) + 2 * (tmu_size + 7),
            MemoryUsage::size_of_val(&v, &mut BTreeSet::new())
        );
    }
}
