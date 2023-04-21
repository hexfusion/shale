use std::ops::DerefMut;
use std::ops::Deref;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

pub type SpaceID = u8;

pub trait MemMutGuard<T> {
    type M: DerefMut<Target = T>;
}

pub trait MemGuard<T> {
    type M: Deref<Target = T>;
}

impl<T> MemMutGuard<T> for DynamicMem<T> {
    type M = MutexGuard<T>;
}

impl<T: CachedStore + 'static> MemGuard<T> for DynamicMem<T> {
    type M<'a> = MutexGuard<'a, T>;
}

pub trait CachedStore: MemMutGuard<Self> + MemGuard<Self> + Debug + Send + Sync + Sized
    {
    type Bytes: AsRef<[u8]>;
    /// Returns a handle that pins the `length` of bytes starting from `offset` and makes them
    /// directly accessible.
    fn get_view(
        &self,
        offset: u64,
        length: u64,
    ) -> Option<<Self as MemGuard<Self>>::M>;
    /// Returns a handle that allows shared access to the store.
    fn get_mut(&self) -> <Self as MemMutGuard<Self>>::M;
    /// Write the `change` to the portion of the linear space starting at `offset`. The change
    /// should be immediately visible to all `CachedView` associated to this linear space.
    fn write(&mut self, offset: u64, change: &[u8]);
    /// Returns the identifier of this storage space.
    fn id(&self) -> SpaceID;
}


// Purely volatile, dynamically allocated vector-based implementation for [CachedStore]. This is similar to
/// [PlainMem]. The only difference is, when [write] dynamically allocate more space if original space is
/// not enough.
#[derive(Clone, Debug)]
pub struct DynamicMem<T> {
    space: Arc<Mutex<T>>,
    id: SpaceID,
}

impl<T> DynamicMem<T> {
    pub fn new(id: SpaceID, mem: T) -> Self {
        let space = Arc::new(Mutex::new(mem));
        Self { space, id }
    }

    fn lock(&self) -> MutexGuard<T> {
        self.state.lock().unwrap()
    }
}

impl<T: Debug> CachedStore for DynamicMem<T> {
    type Bytes = T;
    fn get_view(
        &self,
        offset: u64,
        length: u64,
    ) -> Option<<Self as MemGuard<Self::Bytes>>::M> {
        let offset = offset as usize;
        let length = length as usize;
        let size = offset + length;
        // Increase the size if the request range exceeds the current limit.
        if size > self.space.lock().len() {
            self.space.lock().resize(size, 0);
        }
        Some(self.lock())
    }

    fn get_mut(&self) -> Option<<Self as MemMutGuard<Self::Bytes>>::M> {
        Some(self.lock())
    }

    fn write(&mut self, offset: u64, change: &[u8]) {
        let offset = offset as usize;
        let length = change.len();
        let size = offset + length;
        // Increase the size if the request range exceeds the current limit.
        if size > self.space.borrow().len() {
            self.space.borrow_mut().resize(size, 0);
        }
        self.space.borrow_mut()[offset..offset + length].copy_from_slice(change)
    }

    fn id(&self) -> SpaceID {
        self.id
    }
}

#[derive(Debug)]
struct DynamicMemView<T> {
    offset: usize,
    length: usize,
    mem: DynamicMem<T>,
}

struct DynamicMemShared<T>(DynamicMem<T>);

impl<T> Deref for DynamicMemView<T> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
         self.mem.as_ref()
    }
}

impl<T: CachedStore> Deref for DynamicMemShared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: CachedStore> DerefMut for DynamicMemShared<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> CachedView for DynamicMemView<T> {
    type DerefReturn = Vec<u8>;

    fn as_deref(&self) -> Self::DerefReturn {
        self.mem.get_space_mut()[self.offset..self.offset + self.length].to_vec()
    }
}

pub trait CachedView {
    type DerefReturn: Deref<Target = [u8]>;
    fn as_deref(&self) -> Self::DerefReturn;
}

