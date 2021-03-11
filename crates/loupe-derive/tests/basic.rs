use loupe::{MemoryUsage, MemoryUsageVisited};
use loupe_derive::MemoryUsage;

use std::collections::BTreeSet;

#[test]
fn test_struct_flat() {
    #[derive(MemoryUsage)]
    struct Point {
        x: i32,
        y: i32,
    }

    let p = Point { x: 1, y: 2 };
    assert_eq!(8, MemoryUsage::size_of_val(&p, &mut BTreeSet::new()));
}

#[test]
fn test_tuple() {
    #[derive(MemoryUsage)]
    struct Tuple(i32, i32);

    let p = Tuple(1, 2);
    assert_eq!(8, MemoryUsage::size_of_val(&p, &mut BTreeSet::new()));
}

#[test]
fn test_struct_generic() {
    #[derive(MemoryUsage)]
    struct Generic<T>
    where
        T: MemoryUsage,
    {
        x: T,
        y: T,
    }

    let g = Generic { x: 1i64, y: 2i64 };
    assert_eq!(16, MemoryUsage::size_of_val(&g, &mut BTreeSet::new()));
}

#[test]
fn test_struct_empty() {
    #[derive(MemoryUsage)]
    struct Empty;

    let e = Empty;
    assert_eq!(0, MemoryUsage::size_of_val(&e, &mut BTreeSet::new()));
}

#[test]
fn test_struct_padding() {
    // This struct is packed in order <x, z, y> because 'y: i32' requires 32-bit
    // alignment but x and z do not. It starts with bytes 'x...yyyy' then adds 'z' in
    // the first place it fits producing 'xz..yyyy' and not 12 bytes 'x...yyyyz...'.
    #[derive(MemoryUsage)]
    struct Padding {
        x: i8,
        y: i32,
        z: i8,
    }

    let p = Padding { x: 1, y: 2, z: 3 };
    assert_eq!(8, MemoryUsage::size_of_val(&p, &mut BTreeSet::new()));
}

#[test]
fn test_enum() {
    #[derive(MemoryUsage)]
    enum Things {
        A,
        B(),
        C(i32),
        D { x: i32 },
        E(i32, i32),
        F { x: i32, y: i32 },
    }
}
