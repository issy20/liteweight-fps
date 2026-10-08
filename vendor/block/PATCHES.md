# Local patches

Upstream source: [`block` 0.1.6](https://crates.io/crates/block/0.1.6), MIT licensed.

- Model `_NSConcreteStackBlock` as an external byte symbol and cast its address to
  an opaque `Class` pointer. This avoids declaring a static of an uninhabited
  type while preserving the symbol address used by the Blocks ABI.
- Spell out the existing C ABI on extern blocks and callback function pointers
  so current Rust compilers do not emit `missing_abi` warnings.

The dependency is patched in the root `Cargo.toml` because `wgpu` 24 pulls this
crate through `metal` on Apple platforms. Remove this copy when the dependency
chain no longer requires the affected upstream declaration.
