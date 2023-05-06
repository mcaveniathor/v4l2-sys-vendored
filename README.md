# v4l2-sys-vendored

This adds the option to natively build and link to libv4l and libv4l2 rather than link to system libraries --this is especially useful for cross-compilation for architectures (looking at you, RISC-V) that don't have many libraries in package managers yet. 

## Usage
By providing the same bindings as the `libv4l2-sys` crate, the exact same utilities and abstractions that `libv4l-rs` provides can be used. 
Simply turn off the default features for that crate and add the `vendored` feature, then use the crate as you normally would.

[dependencies]
v4l = { version = "0.13.1", default-features = false, features = ["vendored"]}


Cross compilation has been tested with cross:

cross build --target riscv64gc-unknown-linux-gnu --features vendored --no-default-features
