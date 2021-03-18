#[cfg(test)]
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::mem;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, RwLock};

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

#[cfg(test)]
macro_rules! assert_size_of_val_eq {
    ($value:expr, $expected:expr $(,)*) => {
        assert_size_of_val_eq!($value, $expected, &mut BTreeSet::new());
    };

    ($value:expr, $expected:expr, $tracker:expr $(,)*) => {
        assert_eq!(MemoryUsage::size_of_val(&$value, $tracker), $expected);
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

    ( $( $type:ty ),+ $(,)* ) => {
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

// Slice types.
impl<T: MemoryUsage> MemoryUsage for [T] {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|value| value.size_of_val(tracker) - mem::size_of_val(value))
                .sum::<usize>()
    }
}

impl<T: MemoryUsage> MemoryUsage for &[T] {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + if tracker.track(*self as *const [T] as *const ()) {
                MemoryUsage::size_of_val(*self, tracker)
            } else {
                0
            }
    }
}

#[cfg(test)]
mod test_slice_types {
    use super::*;

    #[test]
    fn test_slice() {
        assert_size_of_val_eq!([1i16], 2 * 1);
        assert_size_of_val_eq!([1i16, 2], 2 * 2);
        assert_size_of_val_eq!([1i16, 2, 3], 2 * 3);
    }

    #[test]
    fn test_slice_dynamically_sized() {
        let slice: &[i16] = &[];
        assert_size_of_val_eq!(slice, 2 * POINTER_BYTE_SIZE + 2 * 0);

        let slice: &[i16] = &[1];
        assert_size_of_val_eq!(slice, 2 * POINTER_BYTE_SIZE + 2 * 1);

        let slice: &[i16] = &[1, 2];
        assert_size_of_val_eq!(slice, 2 * POINTER_BYTE_SIZE + 2 * 2);

        let slice: &[i16] = &[1, 2, 3];
        assert_size_of_val_eq!(slice, 2 * POINTER_BYTE_SIZE + 2 * 3);
    }
}

// Array types.
impl<T: MemoryUsage, const N: usize> MemoryUsage for [T; N] {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|value| value.size_of_val(tracker) - mem::size_of_val(value))
                .sum::<usize>()
    }
}

#[cfg(test)]
mod test_array_types {
    use super::*;

    #[test]
    fn test_array() {
        let array: [i16; 0] = [0; 0];
        assert_size_of_val_eq!(array, 2 * 0);

        let array: [i16; 1] = [0; 1];
        assert_size_of_val_eq!(array, 2 * 1);

        let array: [i16; 2] = [0; 2];
        assert_size_of_val_eq!(array, 2 * 2);

        let array: [i16; 3] = [0; 3];
        assert_size_of_val_eq!(array, 2 * 3);

        let array: [[i16; 3]; 5] = [[0; 3]; 5];
        assert_size_of_val_eq!(array, 2 * 3 * 5);
    }
}

// String types.
impl MemoryUsage for &str {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.as_bytes().size_of_val(tracker)
    }
}

impl MemoryUsage for String {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        self.as_str().size_of_val(tracker)
    }
}

#[cfg(test)]
mod test_string_types {
    use super::*;

    #[test]
    fn test_str() {
        let string: &str = "";
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 0);

        let string: &str = "a";
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 1);

        let string: &str = "ab";
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 2);

        let string: &str = "abc";
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 3);

        let string: &str = "…";
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 3);
    }

    #[test]
    fn test_string() {
        let string: String = "".to_string();
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 0);

        let string: String = "a".to_string();
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 1);

        let string: String = "ab".to_string();
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 2);

        let string: String = "abc".to_string();
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 3);

        let string: String = "…".to_string();
        assert_size_of_val_eq!(string, 2 * POINTER_BYTE_SIZE + 1 * 3);
    }
}

// Tuple types.
macro_rules! impl_memory_usage_for_tuple {
    ( $first_type:ident $(,)* ) => {};

    ( $first_type:ident $( , $types:ident )+ $(,)* ) => {
        impl< $first_type $( , $types )+ > MemoryUsage for ( $first_type $( , $types )+ )
        where
            $first_type: MemoryUsage,
            $( $types: MemoryUsage ),*
        {
            fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
                #[allow(non_snake_case)]
                let ( $first_type $( , $types )+ ) = self;

                mem::size_of_val(self)
                    + $first_type.size_of_val(tracker) - mem::size_of_val($first_type)
                    $( + $types.size_of_val(tracker) - mem::size_of_val($types) )+
            }
        }

        impl_memory_usage_for_tuple!( $( $types ),+ );
    };
}

impl_memory_usage_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

#[cfg(test)]
mod test_tuple_types {
    use super::*;

    #[test]
    fn test_tuple() {
        let tuple: (i8, i8) = (1, 2);
        assert_size_of_val_eq!(tuple, 1 /* i8 */ + 1 /* i8 */);

        let tuple: (i8, i16) = (1, 2);
        assert_size_of_val_eq!(tuple, 1 /* i8 */ + 2 /* i16 */ + 1 /* padding */);

        let tuple: (i8, i16, i32) = (1, 2, 3);
        assert_size_of_val_eq!(
            tuple,
            1 /* i8 */ + 2 /* i16 */ + 4 /* i32 */ + 1, /* padding */
        );

        let tuple: (i32, i32) = (1, 2);
        assert_size_of_val_eq!(tuple, 4 /* i32 */ + 4 /* i32 */);

        let tuple: (&str, &str) = ("", "");
        assert_size_of_val_eq!(
            tuple,
            2 * POINTER_BYTE_SIZE + 1 * 0 /* str */ + 2 * POINTER_BYTE_SIZE + 1 * 0, /* str */
        );

        let tuple: (&str, &str) = ("a", "bc");
        assert_size_of_val_eq!(
            tuple,
            2 * POINTER_BYTE_SIZE + 1 * 1 /* str */ + 2 * POINTER_BYTE_SIZE + 1 * 2, /* str */
        );

        let tuple: (&str, (i64, i64, i8)) = ("abc", (1, 2, 3));
        assert_size_of_val_eq!(
            tuple,
            2 * POINTER_BYTE_SIZE + 1 * 3 /* str */ + 8 /* i64 */ + 8 /* i64 */ + 1 /* i8 */ + 7, /* padding */
        );
    }
}

// Standard library types

// Box types.
impl<T: MemoryUsage + ?Sized> MemoryUsage for Box<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.as_ref().size_of_val(tracker)
    }
}

#[cfg(test)]
mod test_box_types {
    use super::*;

    #[test]
    fn test_box() {
        let b: Box<i8> = Box::new(1);
        assert_size_of_val_eq!(b, POINTER_BYTE_SIZE + 1);

        let b: Box<i32> = Box::new(1);
        assert_size_of_val_eq!(b, POINTER_BYTE_SIZE + 4);

        let b: Box<&str> = Box::new("abc");
        assert_size_of_val_eq!(b, POINTER_BYTE_SIZE + 2 * POINTER_BYTE_SIZE + 1 * 3);

        let b: Box<(i8, i16)> = Box::new((1, 2));
        assert_size_of_val_eq!(
            b,
            POINTER_BYTE_SIZE + 1 /* i8 */ + 2 /* i16 */ + 1, /* padding */
        );
    }

    #[test]
    fn test_boxed_slice() {
        let b: Box<[u8]> = vec![].into_boxed_slice();
        assert_size_of_val_eq!(b, 2 * POINTER_BYTE_SIZE);

        let b: Box<[u8]> = vec![1, 2, 3].into_boxed_slice();
        assert_size_of_val_eq!(b, 2 * POINTER_BYTE_SIZE + 1 * 3);
    }
}

// Option types.
impl<T: MemoryUsage> MemoryUsage for Option<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|value| value.size_of_val(tracker))
                .sum::<usize>()
    }
}

#[cfg(test)]
mod test_option_types {
    use super::*;

    #[test]
    fn test_option() {
        let option: Option<i8> = None;
        assert_size_of_val_eq!(option, 1 /* variant */ + 1 /* padding */);

        let option: Option<i8> = Some(1);
        assert_size_of_val_eq!(option, 1 /* variant */ + 1 /* padding */ + 1 /* i8 */);

        let option: Option<i32> = None;
        assert_size_of_val_eq!(option, 1 /* variant */ + 7 /* padding */);

        let option: Option<i32> = Some(1);
        assert_size_of_val_eq!(option, 1 /* variant */ + 7 /* padding */ + 4 /* i32 */);

        let option: Option<&str> = None;
        assert_size_of_val_eq!(option, 1 /* variant */ + 15 /* padding */);

        let option: Option<&str> = Some("abc");
        assert_size_of_val_eq!(
            option,
            1 /* variant */ + 15 /* padding */ + 2 * POINTER_BYTE_SIZE + 1 * 3, /* &str */
        );
    }
}

// Result types.
impl<T: MemoryUsage, E: MemoryUsage> MemoryUsage for Result<T, E> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + match self.as_ref() {
                Ok(value) => value.size_of_val(tracker),
                Err(value) => value.size_of_val(tracker),
            }
    }
}

#[cfg(test)]
mod test_result_types {
    use super::*;

    #[test]
    fn test_result() {
        let result: Result<i8, i16> = Err(2);
        assert_size_of_val_eq!(result, 1 /* variant */ + 3 /* padding */ + 2 /* i16 */);

        let result: Result<i8, i16> = Ok(1);
        assert_size_of_val_eq!(result, 1 /* variant */ + 3 /* padding */ + 1 /* i8 */);

        let result: Result<i32, ()> = Ok(1);
        assert_size_of_val_eq!(result, 1 /* variant */ + 7 /* padding */ + 4 /* i32 */);
    }
}

// Vector types.
impl<T: MemoryUsage> MemoryUsage for Vec<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|value| value.size_of_val(tracker))
                .sum::<usize>()
    }
}

#[cfg(test)]
mod test_vec_types {
    use super::*;

    #[test]
    fn test_vec() {
        let empty_vec_size = mem::size_of_val(&Vec::<i8>::new());

        let mut vec: Vec<i8> = Vec::new();
        assert_size_of_val_eq!(vec, empty_vec_size + 1 * 0);

        vec.push(1);
        assert_size_of_val_eq!(vec, empty_vec_size + 1 * 1);

        vec.push(2);
        assert_size_of_val_eq!(vec, empty_vec_size + 1 * 2);
    }

    #[test]
    fn test_vec_not_unique() {
        let empty_vec_size = mem::size_of_val(&Vec::<&i32>::new());

        let mut vec: Vec<&i32> = Vec::new();
        assert_size_of_val_eq!(vec, empty_vec_size);

        let one: i32 = 1;
        vec.push(&one);
        assert_size_of_val_eq!(vec, empty_vec_size + POINTER_BYTE_SIZE + 4);

        let two: i32 = 2;
        vec.push(&two);
        assert_size_of_val_eq!(
            vec,
            empty_vec_size + POINTER_BYTE_SIZE + 4 + POINTER_BYTE_SIZE + 4
        );

        // Push a reference to an item that already exists!
        vec.push(&one);
        assert_size_of_val_eq!(
            vec,
            empty_vec_size + POINTER_BYTE_SIZE + 4 + POINTER_BYTE_SIZE + 4 + POINTER_BYTE_SIZE + 0 /* no string content */
        );
    }
}

// Sync types.
impl<T: MemoryUsage + ?Sized> MemoryUsage for Arc<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.as_ref().size_of_val(tracker)
    }
}

impl<T: MemoryUsage + ?Sized> MemoryUsage for Mutex<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.lock().unwrap().size_of_val(tracker)
    }
}

impl<T: MemoryUsage + ?Sized> MemoryUsage for RwLock<T> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.read().unwrap().size_of_val(tracker)
    }
}

#[cfg(test)]
mod test_sync_types {
    use super::*;

    #[test]
    fn test_arc() {
        let empty_arc_size = mem::size_of_val(&Arc::new(()));

        let arc: Arc<i32> = Arc::new(1);
        assert_size_of_val_eq!(arc, empty_arc_size + 4);

        let arc: Arc<Option<i32>> = Arc::new(Some(1));
        assert_size_of_val_eq!(arc, empty_arc_size + POINTER_BYTE_SIZE + 4);
    }

    #[test]
    fn test_mutex() {
        let empty_mutex_size = mem::size_of_val(&Mutex::new(()));

        let mutex: Mutex<i32> = Mutex::new(1);
        assert_size_of_val_eq!(mutex, empty_mutex_size + 4);

        let mutex: Mutex<Option<i32>> = Mutex::new(Some(1));
        assert_size_of_val_eq!(mutex, empty_mutex_size + 2 * POINTER_BYTE_SIZE + 4);
    }

    #[test]
    fn test_rwlock() {
        let empty_rwlock_size = mem::size_of_val(&RwLock::new(()));

        let rwlock: RwLock<i32> = RwLock::new(1);
        assert_size_of_val_eq!(rwlock, empty_rwlock_size + 4);

        let rwlock: RwLock<Option<i32>> = RwLock::new(Some(1));
        assert_size_of_val_eq!(rwlock, empty_rwlock_size + 2 * POINTER_BYTE_SIZE + 4);
    }
}

// Collection types.
impl<K: MemoryUsage, V: MemoryUsage> MemoryUsage for HashMap<K, V> {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
            + self
                .iter()
                .map(|(key, value)| key.size_of_val(tracker) + value.size_of_val(tracker))
                .sum::<usize>()
    }
}

#[cfg(test)]
mod test_collection_types {
    use super::*;

    #[test]
    fn test_hashmap() {
        let mut hashmap: HashMap<i8, i32> = HashMap::new();
        let empty_hashmap_size = mem::size_of_val(&hashmap);
        assert_size_of_val_eq!(hashmap, empty_hashmap_size + 1 * 0 + 4 * 0);

        hashmap.insert(1, 1);
        assert_size_of_val_eq!(hashmap, empty_hashmap_size + 1 * 1 + 4 * 1);

        hashmap.insert(2, 2);
        assert_size_of_val_eq!(hashmap, empty_hashmap_size + 1 * 2 + 4 * 2);
    }

    #[test]
    fn test_hashmap_not_unique() {
        let mut hashmap: HashMap<i8, &i32> = HashMap::new();
        let empty_hashmap_size = mem::size_of_val(&hashmap);
        assert_size_of_val_eq!(
            hashmap,
            empty_hashmap_size + 1 * 0 + (POINTER_BYTE_SIZE + 4) * 0
        );

        let one: i32 = 1;
        hashmap.insert(1, &one);
        assert_size_of_val_eq!(
            hashmap,
            empty_hashmap_size + 1 * 1 + (POINTER_BYTE_SIZE + 4) * 1
        );

        let two: i32 = 2;
        hashmap.insert(2, &two);
        assert_size_of_val_eq!(
            hashmap,
            empty_hashmap_size + 1 * 2 + (POINTER_BYTE_SIZE + 4) * 2
        );

        // Push a reference to an item that already exists!
        hashmap.insert(3, &one);
        assert_size_of_val_eq!(
            hashmap,
            empty_hashmap_size + 1 * 3 + (POINTER_BYTE_SIZE + 4) * 2 + POINTER_BYTE_SIZE + 0 /* no i32 */
        );
    }
}

impl<T> MemoryUsage for PhantomData<T> {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        0
    }
}

// Path types.
impl MemoryUsage for PathBuf {
    fn size_of_val(&self, _: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.capacity()
    }
}

#[cfg(test)]
mod test_path_types {
    use super::*;

    #[test]
    fn test_pathbuf() {
        let mut path = PathBuf::new();
        let empty_path_size = mem::size_of_val(&path);

        path.push("foo");
        assert_size_of_val_eq!(path, empty_path_size + 8);

        path.push("foobar");
        assert_size_of_val_eq!(path, empty_path_size + 16);
    }
}

// TODO:
//
// * Cell
// * Pin (is a Pin always referenceable?)
// * Rc
// * Ref
// * RefCell
// * RefMut
// * UnsafeCell
// * PhantomPinned
