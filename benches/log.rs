// Copyright © 2019 VMware, Inc. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Defines all default criterion benchmarks we run.
#![allow(unused)]
#![feature(test)]

#[macro_use]
extern crate log;
extern crate zipf;

use std::sync::Arc;

use node_replication::log::Log;
use node_replication::replica::Replica;
use node_replication::Dispatch;
use rand::distributions::Distribution;
use rand::{Rng, RngCore};

use zipf::ZipfDistribution;

mod mkbench;
mod utils;

use mkbench::ReplicaTrait;

use utils::benchmark::*;
use utils::Operation;

extern crate jemallocator;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub struct Nop(usize);

impl Dispatch for Nop {
    type ReadOperation = ();
    type WriteOperation = usize;
    type Response = ();
    type ResponseError = ();

    fn dispatch(&self, _op: Self::ReadOperation) -> Result<Self::Response, Self::ResponseError> {
        unreachable!();
    }

    fn dispatch_mut(
        &mut self,
        _op: Self::WriteOperation,
    ) -> Result<Self::Response, Self::ResponseError> {
        unreachable!();
    }
}

/// Compare scale-out behaviour of log.
fn log_scale_bench(c: &mut TestHarness) {
    env_logger::try_init();

    /// Log size (needs to be big as we don't have GC in this case but high tput)
    const LOG_SIZE_BYTES: usize = 12 * 1024 * 1024 * 1024;

    /// Benchmark #operations per iteration
    const NOP: usize = 50_000;

    let mut operations = Vec::new();
    for e in 0..NOP {
        operations.push(Operation::WriteOperation(e));
    }

    mkbench::ScaleBenchBuilder::<Replica<Nop>, Nop>::new(operations)
        .machine_defaults()
        .log_size(LOG_SIZE_BYTES)
        .add_batch(8)
        .reset_log()
        .disable_sync()
        .configure(
            c,
            "log-append",
            |_cid,
             rid,
             log: &Arc<Log<<Nop as Dispatch>::WriteOperation>>,
             _replica: &Arc<Replica<Nop>>,
             op: &utils::Operation<
                <Nop as Dispatch>::ReadOperation,
                <Nop as Dispatch>::WriteOperation,
            >,
             batch_size| match op {
                Operation::WriteOperation(o) => {
                    let _r = log.append(
                        &vec![*o],
                        rid,
                        |_o: <Nop as Dispatch>::WriteOperation, _i: usize| {},
                    );
                }
                _ => unreachable!(),
            },
        );
}

fn main() {
    let mut harness = TestHarness::new(std::time::Duration::from_secs(3));
    log_scale_bench(&mut harness);
}