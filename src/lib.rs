// Copyright 2019 Intel Corporation. All Rights Reserved.
// Copyright 2021 Alibaba Cloud Computing. All rights reserved.
//
// SPDX-License-Identifier: Apache-2.0

//! Struct to maintain state information and manipulate vhost-user queues.

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};


/// Trait for objects returned by `CachedStore::get_ref()`.
pub trait CachedGuard<'a, M> {
    /// Type for guard returned by `CachedStore::get_ref()`.
    type G: Deref<Target = Cached<M>>;
}

/// Trait for objects returned by `CachedStore::get_mut()`.
pub trait CachedMutGuard<'a, M> {
    /// Type for guard returned by `CachedStore::get_mut()`.
    type G: DerefMut<Target = Cached<M>>;
}

pub trait CachedStore<M>: for<'a> CachedGuard<'a, M> + for<'a> CachedMutGuard<'a, M> {
    /// Create a new instance of Vring.
    fn new(mem: M, max_queue_size: u16) -> Self
    where
        Self: Sized;

    /// Get an immutable reference to the kick event fd.
    fn get_ref(&self) -> <Self as CachedGuard<M>>::G;

    /// Get a mutable reference to the kick event fd.
    fn get_mut(&self) -> <Self as CachedMutGuard<M>>::G;
}

/// Struct to maintain raw state information for a vhost-user queue.
///
/// This struct maintains all information of a virito queue, and could be used as an `CachedStore`
/// object for single-threaded context.
pub struct Cached<M> {
    mem: M,
}

impl<M> Cached<M> {
    fn new(mem: M, max_queue_size: u16) -> Self {
        Cached { mem }
    }
}

/// A `Cached` object protected by Mutex for multi-threading context.
#[derive(Clone)]
pub struct CacheMutex<M> {
    state: Arc<Mutex<Cached<M>>>,
}

impl<M> CacheMutex<M> {
    /// Get a mutable guard to the underlying raw `Cached` object.
    fn lock(&self) -> MutexGuard<Cached<M>> {
        self.state.lock().unwrap()
    }
}

impl<'a, M: 'static> CachedGuard<'a, M> for CacheMutex<M> {
    type G = MutexGuard<'a, Cached<M>>;
}

impl<'a, M: 'static> CachedMutGuard<'a, M> for CacheMutex<M> {
    type G = MutexGuard<'a, Cached<M>>;
}

impl<M: 'static> CachedStore<M> for CacheMutex<M> {
    fn new(mem: M, max_queue_size: u16) -> Self {
        Self {
            state: Arc::new(Mutex::new(Cached::new(mem, max_queue_size))),
        }
    }

    fn get_ref(&self) -> <Self as CachedGuard<M>>::G {
        self.state.lock().unwrap()
    }

    fn get_mut(&self) -> <Self as CachedMutGuard<M>>::G {
        self.lock()
    }
}

/// A `Cached` object protected by RwLock for multi-threading context.
#[derive(Clone)]
pub struct CachedInner<M> {
    state: Arc<RwLock<Cached<M>>>,
}

impl<M> CachedInner<M> {
    /// Get a mutable guard to the underlying raw `Cached` object.
    fn write_lock(&self) -> RwLockWriteGuard<Cached<M>> {
        self.state.read().unwrap()
    }
}

impl<'a, M: 'a> CachedGuard<'a, M> for CachedInner<M> {
    type G = RwLockReadGuard<'a, Cached<M>>;
}

impl<'a, M: 'a> CachedMutGuard<'a, M> for CachedInner<M> {
    type G = RwLockWriteGuard<'a, Cached<M>>;
}

impl<M: 'static> CachedStore<M> for CachedInner<M> {
    fn new(mem: M, max_queue_size: u16) -> Self {
        CachedInner {
            state: Arc::new(RwLock::new(Cached::new(mem, max_queue_size))),
        }
    }

    

    fn get_ref(&self) -> <Self as CachedGuard<M>>::G {
        self.state.read().unwrap()
    }

    fn get_mut(&self) -> <Self as CachedMutGuard<M>>::G {
        self.write_lock()
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BytesMut, BufMut};

    use super::*;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_new_vring() {
        let mem = BytesMut::with_capacity(2);
        let vring = CacheMutex::new(mem, 0x1000);

        vring.get_mut().mem.put_u8(0x1);

        let mem = &mut vring.get_mut().mem;
        assert_eq!(mem[0], 0x1);
        mem.put_u8(0x1);

        let binding = vring.get_ref();
        let sth = binding.mem.as_ref();
        assert_eq!(sth[0], 0x1);
    }
}
