# faces-pfm

A reference implementation of the `faces::AbsPageFrameManager` trait.

`faces-pfm` provides a concrete, in‑memory page frame manager for
operating system kernels built with the `faces` framework. It uses a
static array of page frame metadata, initialised from the memory map
provided by the Limine boot protocol, and implements all the required
operations to manage physical page frames and their associated flags.

## Overview

The `faces` crate defines a set of traits to unify low‑level system
interfaces. One of these traits, `AbsPageFrameManager`, describes an
abstract page frame manager that can:
- Set, clear and check per‑frame flags.
- Query the minimum and maximum valid page frame numbers.
- Determine whether a given page frame is present.
- Provide mutable access to a page frame’s metadata.

`faces-pfm` implements this trait with a concrete,
spinlock‑protected array of `PageFrame` metadata entries, making it
suitable for use in a real kernel environment that follows the
Limine boot protocol.

## Features

- **Per‑frame metadata**: tracks flags, order, linked‑list pointers,
reference count, and a kernel virtual address.
- **Flag management**: full support for the standard page flags
(`LOCKED`, `DIRTY`, `UPTODATE`, `LRU`, etc.).
- **Limine integration**: uses the Limine memory map to discover
available physical memory and locate a suitable region for the
metadata array.
- **Thread‑safe access**: each `PageFrame` is protected by a
`spin::Mutex`, providing safe concurrent access.
- **`no_std` support**: can be used in freestanding kernel
environments.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
faces-pfm = "0.1.0"
```

If you are building a kernel that does not have access to the
standard library, enable the `no_std` feature:

```toml
[dependencies]
faces-pfm = { version = "0.1.0", default-features = false, features = ["no_std"] }
```

### Initialisation

Before any page frame operations can be performed, the manager must
be initialised once, typically early in the kernel boot process:

```rust
use faces_pfm::PFM;

fn kernel_main() {
    // The PFM singleton must be initialised before use
    PFM.init();

    // From now on, the manager is ready to use
}
```

The `init` function will:
1. Detect total physical memory and compute the number of page frames.
2. Find a suitable region of physical memory for the metadata array.
3. Map that region into the kernel’s virtual address space.
4. Initialise each metadata entry with default values.
5. Mark all frames as `RESERVED`, then clear that flag for every usable memory
region reported by Limine.
6. Finally, mark the frames occupied by the metadata array itself as `RESERVED`.

### Managing page frames

Once initialised, you can use the `PFM` singleton to perform operations on
page frames by their physical frame number (PFN):

```rust
use faces_pfm::{PageFlags, PFM};
use faces::to;

fn example() {
    let pfn = to(42);   // PFN 42 – a 4 KiB page at physical address 0x2a000

    // Set some flags
    PFM.set_flags(pfn, PageFlags::LOCKED);
    PFM.set_flags(pfn, PageFlags::DIRTY);

    // Check if a flag is set
    if PFM.check_flags(pfn, PageFlags::LOCKED) {
        // ...
    }

    // Clear a flag
    PFM.clear_flags(pfn, PageFlags::DIRTY);

    // Obtain a mutable guard to the page frame metadata
    {
        let mut guard = PFM.get(pfn);
        guard.order = 2;          // allocate a 16 KiB block (2^2 pages)
        guard.rc = 1;
    }
}
```

## Test suite

`faces-pfm` includes a unit test suite that runs without hardware
dependencies. The tests set up a mock frame array and verify the flag
and guard operations. To run the tests:

```bash
cargo test
```

## Dependencies

- `faces` – provides the abstract trait interfaces.
- `limine` – used to obtain the memory map from the bootloader.
- `spin` – provides the `Mutex` type for synchronised access to each `PageFrame`.
- `log` – used for debug output during initialisation.
- `bitflags` – supports the definition of `PageFlags`.

## License

This crate is licensed under either the MIT License or the Apache License, Version 2.0, at your option.
