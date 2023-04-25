# `optivanity`, brought to you by Econia Labs

*Hyper-parallelized vanity address generator for the Aptos blockchain*

- [`optivanity`, brought to you by Econia Labs](#optivanity-brought-to-you-by-econia-labs)
  - [Quickstart](#quickstart)
    - [Setup](#setup)
    - [Quad-core Ed25519 vanity address generation](#quad-core-ed25519-vanity-address-generation)
    - [Octa-core multisig vanity address generation](#octa-core-multisig-vanity-address-generation)
  - [General](#general)
  - [Parallelism](#parallelism)

## Quickstart

### Setup

```zsh
% git clone https://github.com/econia-labs/optivanity.git
% cd optivanity
% cargo build --release
```

### Quad-core Ed25519 vanity address generation

```zsh
# Generate standard account addresses starting with aaaaa, parallelized across 4 cores
% cargo run --release -- aaaaa 4
Standard account address: 0xaaaaab5d00e4ffe9ba3add22f0c62b592b18d6b0401b12f1d0732e180cd45781
Private key:              0x00d8a9b865324baa161925221536f4ea1f6da9a04b88dd717a1f8109d80ef242

Standard account address: 0xaaaaa4637eb8cacd4b8781dae490b219a5bd37ac0241babbe42af73131e8fb28
Private key:              0x234baa20f4f1c29bb0d1d1d760d2a5059dc3eb7d9cd3ac5397fa717234691e18

Standard account address: 0xaaaaaef43df45095219c39e15762fa71c58d147dbeb999646552109bfda53ca7
Private key:              0x8f5e2806c2819159a34e83e78f5967410974964f4e3434fb599689e0891a7b2d

^C
# (Press Ctrl + c to stop the search)
```

### Octa-core multisig vanity address generation

```zsh
# Generate multisig account addresses starting with bbbbbb, parallelized across 8 cores
% cargo run --release -- bbbbbb 8 -m

Multisig account address: 0xbbbbbb8dfacadee4c12094e8477eb1b306e1be5ad6aa355a544953fc8aeeda82
Standard account address: 0x9950b5fd7bfca1e42341e7363aced4400beefdf9c73658b2b346916c6485425b
Private key:              0x331b54bbf7d21fb4064020f391ee81f76bdf21e89c130a91dae18486963e3d74

Multisig account address: 0xbbbbbb29f65eb8d01b0ce0653e0a2ef782f770c1398d5dbb5474746515be337a
Standard account address: 0xa0f77fce095349d2b5a52cda01a831fce8135cbd403956254ca92cc067184a22
Private key:              0x05bc61b9d36a7f984fd57337ac9c5fca784e639c88dd0e1bf65f72186446cb36

^C
# (Press Ctrl + c to stop the search)
```

## General

`optivanity` provides vanity address generation functionality similar to that of the `aptos` CLI, but with assorted performance optimizations:

- Thread count positional argument for configurable execution parallelism
- Byte-wise search, instead of expensive string-wise search like in the `aptos` CLI
- Build enhancements including [linker-time optimization](https://doc.rust-lang.org/cargo/reference/profiles.html#lto) and [code generation unit](https://doc.rust-lang.org/cargo/reference/profiles.html#codegen-units) minimization
- Minimal crate includes for reduced compile times compared with `aptos` CLI

`optivanity` runs in an infinite loop, generating vanity prefixes until the user stops the search by pressing `Ctrl + c`.
Input checking is minimal, so use the exact syntax from the [quickstart](#quickstart):

```bash
# Optional '-m' flag indicates multisig vanity prefix
% cargo run --release -- <PREFIX_WITHOUT_LEADING_0x> <THREAD_COUNT> [-m]
```

Don't forget to use `cargo`'s [`--release` flag](https://doc.rust-lang.org/cargo/reference/profiles.html#release) for maximal build performance, and note that each additional hex character will on average slow down the search by a factor of sixteen (an `aaaaaa` search will on average take 256 times as long as an `aaaa` search for a given thread count).

## Parallelism

The thread count positional argument controls how many independent search threads will be initiated during execution.
Two search threads running in parallel, for example, will on average take $\frac{1}{2}$ as long to generate an address compared with a single search thread, which is what the `aptos` CLI uses.
Three threads will on average take $\frac{1}{3}$ as long, four will on average take $\frac{1}{4}$ as long, and so on, **until the thread count equals the number of available cores**:
if your machine only has four available cores, you will not generate performance increases for a five-thread search because you can only run four threads in parallel.

The algorithms in `optivanity` were developed on a 2021 MacBook Pro with a ten-core [Apple M1 Max chip](https://en.wikipedia.org/wiki/Apple_M1#M1_Pro_and_M1_Max), where the optimal thread count for search speed is ten.
This is probably not the optimal thread count for machine longevity, however, because a ten-thread search results in the fan running full blast to prevent overheating.
Running with only six threads does not result in the fan noticeably turning on and is sufficient, for example, to generate an address with an eight-character vanity prefix overnight.

`optivanity` relies on a main thread with a quasi-infinite delay to prevent the closure of search threads.
Hence in the "Activity Monitor" app for the above machine, `cargo run --release -- aaaaa 6` search results in the following:

| Process Name | % CPU | Threads |
| ------------ | ----- | ------- |
| `optivanity` | 600   | 7       |

Here, six cores are each running a search thread at ~100% capacity, with a seventh non-search thread consuming almost no load.
Hence without other major processes running, this results in a user CPU load of slightly above 60%.

Again, the optimal thread count is architecture specific, and your machine will not be able to accommodate parallelism in excess of the number of available cores:
running a twenty-thread search on the above machine, for example, results in a performance *decrease* over a ten-thread search because the system scheduler has to constantly pause and restart threads.
