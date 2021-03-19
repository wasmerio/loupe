#[cfg(test)]
use crate::{assert_size_of_val_eq, POINTER_BYTE_SIZE};
use crate::{MemoryUsage, MemoryUsageTracker};
use std::mem;
use std::sync::{Arc, Mutex, RwLock};

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
