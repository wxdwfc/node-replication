// Copyright © 2019-2020 VMware, Inc. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Concurrent Node Replication (CNR) is a library which can be used to implement a
//! NUMA-aware version of any concurrent data structure. It takes in a
//! concurrent implementation of said data structure, and scales it out to
//! multiple cores and NUMA nodes by combining three techniques:
//! commutativity based work partitioning, operation logging, and flat combining..
//!
//! # How does it work
//! To replicate a concurrent data structure, one needs to implement the
//! [Dispatch](trait.Dispatch.html) trait for it. To map the operation to a log,
//! each operation ([ReadOperation](trait.Dispatch.html#associatedtype.ReadOperation)
//! and [WriteOperation](trait.Dispatch.html#associatedtype.WriteOperation))
//! needs to implement [LogMapper](trait.LogMapper.html) trait.
//! The following snippet implements [Dispatch](trait.Dispatch.html) for concurrent
//! [HashMap](https://docs.rs/chashmap/2.2.2/chashmap/struct.CHashMap.html)
//! as an example. A complete example (using [Replica](struct.Replica.html)
//! and [Log](struct.Log.html)) can be found in the
//! [examples](https://github.com/vmware/node-replication/tree/master/cnr/examples/hashmap.rs)
//! folder.
//!
//! ```
//! use cnr::Dispatch;
//! use cnr::LogMapper;
//! use chashmap::CHashMap;
//!
//! /// The replicated hashmap uses a concurrent hashmap internally.
//! pub struct CNRHashMap {
//!    storage: CHashMap<usize, usize>,
//! }
//!
//! /// We support a mutable put operation on the hashmap.
//! #[derive(Debug, PartialEq, Clone)]
//! pub enum Modify {
//!    Put(usize, usize),
//! }
//!
//! /// Application developer implements LogMapper for each mutable operation.
//! /// It is used to map the operation to one of the many logs. Commutative
//! /// operations can map to same or different log and conflicting operations
//! /// must map to same log.
//! impl LogMapper for Modify {
//!    fn hash(&self) -> usize {
//!       match self {
//!          Modify::Put(key, _val) => *key
//!       }
//!    }
//! }
//!
//! /// We support an immutable read operation to lookup a key from the hashmap.
//! #[derive(Debug, PartialEq, Clone)]
//! pub enum Access {
//!    Get(usize),
//! }
//!
//! /// Application developer implements LogMapper for each immutable operation. It
//! /// is used to map the operation to one of the many log. Commutative operations
//! /// can go to same or different log and conflicts operations must map to same log.
//! impl LogMapper for Access {
//!    fn hash(&self) -> usize {
//!       match self {
//!          Access::Get(key) => *key
//!       }
//!    }
//! }
//!
//! /// The Dispatch traits executes `ReadOperation` (our Access enum)
//! /// and `WriteOperation` (our Modify enum) against the replicated
//! /// data-structure.
//! impl Dispatch for CNRHashMap {
//!    type ReadOperation = Access;
//!    type WriteOperation = Modify;
//!    type Response = Option<usize>;
//!
//!    /// The `dispatch` function applies the immutable operations.
//!    fn dispatch(&self, op: Self::ReadOperation) -> Self::Response {
//!        match op {
//!            Access::Get(key) => self.storage.get(&key).map(|v| *v),
//!        }
//!    }
//!
//!    /// The `dispatch_mut` function applies the mutable operations.
//!    fn dispatch_mut(
//!        &self,
//!        op: Self::WriteOperation,
//!    ) -> Self::Response {
//!        match op {
//!            Modify::Put(key, value) => self.storage.insert(key, value),
//!        }
//!    }
//! }
//! ```
#![no_std]
#![feature(new_uninit)]
#![feature(get_mut_unchecked)]
#![feature(negative_impls)]

#[cfg(test)]
extern crate std;

#[macro_use]
extern crate alloc;
extern crate core;

extern crate crossbeam_utils;

#[macro_use]
extern crate log as logging;

#[macro_use]
extern crate static_assertions;

mod context;
mod log;
mod replica;

pub use crate::log::Log;
pub use replica::{Replica, ReplicaToken, MAX_THREADS_PER_REPLICA};

use core::fmt::Debug;

/// Every data structure must implement [LogMapper](trait.LogMapper.html) trait
/// for [ReadOperation](trait.Dispatch.html#associatedtype.ReadOperation) and
/// [WriteOperation](trait.Dispatch.html#associatedtype.WriteOperation).
///
/// Data structure implement `hash` that is used to map each operation to a log.
/// All the conflicting operations must map to a single log and the commutative
/// operations can map to same or different logs based on the operation argument.
///
/// [Replica](struct.Replica.html) internally performs a modulo operation on `hash`
/// return value with the total number of logs. The data structure can implement
/// trait to return a value between 0 and (#logs-1) to avoid the modulo operation.
pub trait LogMapper {
    /// Method to convert the operation and it's arguments to a log number.
    fn hash(&self) -> usize;
}

/// Trait that a data structure must implement to be usable with this library.
///
/// When this library executes a read-only operation against the data structure,
/// it invokes the `dispatch()` method with the operation as an argument.
///
/// When this library executes a write operation against the data structure, it
/// invokes the `dispatch_mut()` method with the operation as an argument.
pub trait Dispatch {
    /// A read-only operation. When executed against the data structure, an operation
    /// of this type must not mutate the data structure in anyway. Otherwise, the
    /// assumptions made by this library no longer hold.
    type ReadOperation: Sized + Clone + PartialEq + Debug + LogMapper;

    /// A write operation. When executed against the data structure, an operation of
    /// this type is allowed to mutate state. The library ensures that this is done so
    /// in a thread-safe manner.
    type WriteOperation: Sized + Clone + PartialEq + Debug + Send + LogMapper;

    /// The type on the value returned by the data structure when a `ReadOperation` or a
    /// `WriteOperation` successfully executes against it.
    type Response: Sized + Clone;

    /// Method on the data structure that allows a read-only operation to be
    /// executed against it.
    fn dispatch(&self, op: Self::ReadOperation) -> Self::Response;

    /// Method on the data structure that allows a write operation to be
    /// executed against it.
    fn dispatch_mut(&self, op: Self::WriteOperation) -> Self::Response;
}
