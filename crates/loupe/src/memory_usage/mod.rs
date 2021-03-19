mod r#box;
mod cell;
mod collection;
mod marker;
mod option;
mod path;
mod primitive;
mod ptr;
mod remote;
mod result;
mod slice;
mod string;
mod sync;

pub use cell::*;
pub use collection::*;
pub use marker::*;
pub use option::*;
pub use path::*;
pub use primitive::*;
pub use ptr::*;
pub use r#box::*;
pub use remote::*;
pub use result::*;
pub use slice::*;
pub use string::*;
pub use sync::*;

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

// Empty type.
impl MemoryUsage for () {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        0
    }
}

#[macro_export]
macro_rules! assert_size_of_val_eq {
    ($value:expr, $expected:expr $(,)*) => {
        assert_size_of_val_eq!($value, $expected, &mut std::collections::BTreeSet::new());
    };

    ($value:expr, $expected:expr, $tracker:expr $(,)*) => {
        assert_eq!(
            $crate::MemoryUsage::size_of_val(&$value, $tracker),
            $expected
        );
    };
}

// TODO:
//
// * Cell
// * Pin (is a Pin always referenceable?)
// * Rc
// * Ref
// * RefCell
// * RefMut
// * PhantomPinned
