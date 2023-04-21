use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bytes::{Bytes, BytesMut};

pub type SpaceID = u8;
pub const INVALID_SPACE_ID: SpaceID = 0xff;

/// A trait for a `Cached` object that can be read.
pub trait CachedGuard<'a, M> {
    type Guard: Deref<Target = Cached<M>>;
}

/// A trait for a `Cached` object that can be mutated.
pub trait CachedMutGuard<'a, M> {
    type Guard: DerefMut<Target = Cached<M>>;
}

pub trait CachedStore<M>: for<'a> CachedGuard<'a, M> + for<'a> CachedMutGuard<'a, M> {
    fn new(mem: M, offset: u64, length: u64) -> Self
    where
        Self: Sized;

    /// Get a view of the underlying memory.
    fn get_view(
        &self,
        offset: usize,
        length: usize,
    ) -> Vec<u8>;

    /// Get a reference to the underlying memory.
    fn get_ref(&self) -> <Self as CachedGuard<M>>::Guard;

    /// Get a mutable reference to the underlying memory.
    fn get_mut(&self) -> <Self as CachedMutGuard<M>>::Guard;

    /// Write a slice of bytes to the underlying memory.
    fn write(&mut self, offset: u64, change: &[u8]);
    
    /// Returns the identifier of this storage space.
    fn id(&self) -> SpaceID;
}

pub struct Cached<M> {
    mem: M,
}

impl<M: AsRef<M>> Cached<M> {
    fn new(mem: M) -> Self {
        Self { mem }
    }
}

#[derive(Clone)]
pub struct DynamicMem<M> {
    state: Arc<RwLock<Cached<M>>>,
    id: SpaceID,
    offset: usize,
    length: usize,
}

impl<'a, M: 'static> CachedGuard<'a, M> for DynamicMem<M> {
    type Guard = RwLockReadGuard<'a, Cached<M>>;
}

impl<'a, M: 'static> CachedMutGuard<'a, M> for DynamicMem<M> {
    type Guard = RwLockWriteGuard<'a, Cached<M>>;
}

/// A handle that pins and provides a readable access to a portion of the linear memory image.
pub trait CachedView {
    type DerefReturn: Deref<Target = [u8]>;
    fn as_deref(&self) -> Self::DerefReturn;
}

impl CachedView for [u8] {
    type DerefReturn = Vec<u8>;
    fn as_deref(&self) -> Self::DerefReturn {
        self.to_vec()
    }
}

impl CachedView for Vec<u8> {
    type DerefReturn = Vec<u8>;
    fn as_deref(&self) -> Self::DerefReturn {
        self.to_vec()
    }
}

impl CachedView for Bytes {
    type DerefReturn = Vec<u8>;
    fn as_deref(&self) -> Self::DerefReturn {
        self.to_vec()
    }
}

impl CachedView for BytesMut{
    type DerefReturn = Vec<u8>;
    fn as_deref(&self) -> Self::DerefReturn {
        self.to_vec()
    }
}

impl<M: AsRef<M> + CachedView + 'static> CachedView for DynamicMem<M> {
    type DerefReturn = Vec<u8>;

    fn as_deref(&self) -> Self::DerefReturn {
        self.get_mut().mem.as_deref()[self.offset..self.offset + self.length].to_vec()
    }
}

impl<M: AsRef<M> + CachedView + 'static> CachedStore<M> for DynamicMem<M> {
    fn new(mem: M, offset: u64, length: u64) -> Self {
        let offset = offset as usize;
        let length = length as usize;
        Self {
            state: Arc::new(RwLock::new(Cached::new(mem))),
            offset,
            length,
            id: SpaceID::default(),
        }
    }

    fn get_view(
        &self,
        offset: usize,
        length: usize,
    ) -> Vec<u8> {
        self.state.read().unwrap().mem.as_deref()[offset..offset + length].to_vec()
    }

    fn get_ref(&self) -> <Self as CachedGuard<M>>::Guard {
        self.state.read().unwrap()
    }

    fn get_mut(&self) -> <Self as CachedMutGuard<M>>::Guard {
        self.state.write().unwrap()
    }

    fn write(&mut self, offset: u64, change: &[u8]) {
        self.get_mut().mem.put_slice(change)
    }

    fn id(&self) -> SpaceID {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BytesMut, BufMut};

    use super::*;

    #[test]
    fn test_cache() {
        let mem = Vec::with_capacity(2);
        let cache = DynamicMem::new(mem, 0, 2);

        let mut buf = vec![];
        buf.push(0x1);
        buf.push(0x2);
        buf.push(0x3);
        cache.get_mut().mem.put_slice(&buf);
        // println!("found: {:?}", w[..].to_vec());

        // let r = &cache.get_ref().mem;
        // println!("found: {:?}", r[..].to_vec());

        let view = cache.get_view(2, 1);
        println!("found: {:?}", view);
        let view2 = cache.get_view(0, 2);
        println!("found: {:?}", view2);
        //  cache.get_mut().mem.put_slice(&buf);



        let mem = &mut cache.get_mut().mem;
        // assert_eq!(mem[0], 0x1);
        mem.put_u8(0x1);
        // assert_eq!(mem[0], 0x1);
    }
}
