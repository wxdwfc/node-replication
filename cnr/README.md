# node-replication

CNR (Concurrent Node Replication) library is extension to [Black-box Concurrent Data Structures for NUMA
Architectures](https://dl.acm.org/citation.cfm?id=3037721) paper.

This library can be used to implement a NUMA-aware concurrent version of any
concurrent data structure. It takes in a concurrent implementation of said
data structure, and scales it out to multiple cores and NUMA nodes by combining
three techniques: commutativity based work partitioning, operation logging, and flat combining.

## How does it work

To replicate a concurrent data structure, one needs to implement `Dispatch` and `LogMapper` (from cnr). `LogMapper` implementation dictates the work partitioning across multiple logs for concurrent execution.

`LogMapper` implementation is used to map an operation to a log. Two commutative operations can be mapped to same or different log; however, two conflicting operations must map to the same log. As an example, we implement `Dispatch` for the [CHashMap](https://crates.io/crates/chashmap) and `LogMapper` for the supported operations.

```rust
/// The replicated hashmap uses a concurrent hashmap internally.
pub struct CNRHashMap {
   storage: CHashMap<usize, usize>,
}

/// We support a mutable put operation on the hashmap.
#[derive(Debug, PartialEq, Clone)]
pub enum Modify {
   Put(usize, usize),
}

/// This `LogMapper` implementation distributes the keys amoung multiple logs
/// in a round-robin fashion. One can change the implementation to improve the 
/// data locality based on the data sturucture layout in the memory.
impl LogMapper for Modify {
   fn hash(&self) -> usize {
      match self {
         Modify::Put(key, _val) => *key
      }
   }
}

/// We support an immutable read operation to lookup a key from the hashmap.
#[derive(Debug, PartialEq, Clone)]
pub enum Access {
   Get(usize),
}

/// `Access` follows the same operation to log mapping as the `Modify`. This
/// ensures that the read and write operations for a particular key go to
/// the same log.
impl LogMapper for Access {
   fn hash(&self) -> usize {
      match self {
         Access::Get(key) => *key
      }
   }
}

/// The Dispatch traits executes `ReadOperation` (our Access enum)
/// and `WriteOperation` (our Modify enum) against the replicated
/// data-structure.
impl Dispatch for CNRHashMap {
   type ReadOperation = Access;
   type WriteOperation = Modify;
   type Response = Option<usize>;

   /// The `dispatch` function applies the immutable operations.
   fn dispatch(&self, op: Self::ReadOperation) -> Self::Response {
       match op {
           Access::Get(key) => self.storage.get(&key).map(|v| *v),
       }
   }

   /// The `dispatch_mut` function applies the mutable operations.
   fn dispatch_mut(&self, op: Self::WriteOperation) -> Self::Response {
       match op {
           Modify::Put(key, value) => self.storage.insert(key, value),
       }
   }
}
```

The full example (using `HashMap` as the underlying data-structure) can be found
[here](examples/hashmap.rs). To run, execute: `cargo run --example hashmap`

## Compile the library

The library currently requires a nightly rust compiler (due to the use of
`new_uninit`, and `get_mut_unchecked`, `negative_impls` API). The library works
with `no_std`.

```bash
rustup toolchain install nightly
rustup default nightly
cargo build
```

As a dependency in your `Cargo.toml`:

```toml
cnr = "*"
```

The code should currently be treated as an early release and is still work in
progress. In its current form, the library is only known to work on x86
platforms (other platforms will require some changes and are untested).

## Testing

There are a series of unit tests as part of the implementation and a few
[integration tests](./tests) that check various aspects of the implementation
using a stack.

You can run the tests by executing: `cargo test`

## Benchmarks

The benchmarks (and how to execute them) are explained in more detail in the
[benches](../benches/README.md) folder.

## Contributing

The node-replication project team welcomes contributions from the community. If
you wish to contribute code and you have not signed our contributor license
agreement (CLA), our bot will update the issue when you open a Pull Request. For
any questions about the CLA process, please refer to our
[FAQ](https://cla.vmware.com/faq).
