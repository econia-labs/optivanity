# `optivanity`, brought to you by Econia Labs

*Hyper-parallelized vanity address generator for the Aptos blockchain*

- [`optivanity`, brought to you by Econia Labs](#optivanity-brought-to-you-by-econia-labs)
  - [Quickstart](#quickstart)
    - [Setup](#setup)
    - [Ed25519 vanity address generation](#ed25519-vanity-address-generation)
    - [Octa-core multisig vanity address generation](#octa-core-multisig-vanity-address-generation)
  - [General](#general)
  - [Parallelism](#parallelism)
  - [CPU load](#cpu-load)

## Quickstart

### Setup

```zsh
% git clone https://github.com/econia-labs/optivanity.git
% cd optivanity
% cargo run --release -- --help

Optivanity: hyper-parallelized vanity address generator for the Aptos blockchain, brought to you by Econia Labs

Usage: optivanity [OPTIONS] --prefix <PREFIX>

Options:
  -p, --prefix <PREFIX>    Address prefix to match (no leading `0x`). Each additional character slows search by 16x
  -m, --multisig           Use this flag if you want to search for multisig address(es)
  -c, --count <COUNT>      Number of vanity accounts to generate [default: 1]
  -t, --threads <THREADS>  Number of threads to use. Only specify if you want to use fewer cores than available [default: 10]
  -h, --help               Print help
```

### Ed25519 vanity address generation

```zsh
# Generate a single standard account address starting with aaaaa, maximum parallelism
% cargo run --release -- --prefix aaaaa

Starting search at 2023-04-26T12:06:11.583917-07:00

Standard account address: 0xaaaaa08d6dd99050567e11eb5ac338c8b7976a94d8dc07d2c346fc60e12a5d32
Private key:              0x3912b341c3763f9193dab75df849dbe232ca1e0234dec690e78578e30ebc8e20

Elapsed time: 1.059834208s
```

### Octa-core multisig vanity address generation

```zsh
# Generate 3 multisig account addresses starting with bbbbbb, parallelized across 8 cores
% cargo run --release -- --prefix bbbbbb --multisig --count 3 --threads 8

Starting search at 2023-04-26T12:06:57.444782-07:00

Multisig account address: 0xbbbbbbf1061840cccc8542b98ace03b73a2ffc7df609fbaf99c546982a8b7dc8
Standard account address: 0xfc05df546a70c2fafeb8b9b19637fc9b70c2300e1e6f570c6369798f4afd6c77
Private key:              0xd220c6fb9df3836d8e1463435cb427ff270b3ce417769adaf5903ebd49560da4

Multisig account address: 0xbbbbbb4ab601a7c40b96384ef63a357609ad843056fbab6158bb396dd3193878
Standard account address: 0x490f50f84013452a6660d99e8018acc1f20241da8321e9ddfb0d2b70c48742ef
Private key:              0x7cd4bece605833845b00da1015e4615d48dffdf5714ce62980d305934f13df80

Multisig account address: 0xbbbbbbfba754ccf12a05bf087051fe190928e41bafbeaa20ca94b6b066cfbed8
Standard account address: 0x9f770fb89a85ce2916f127d3280d261234fc572d9949d7a9ffbe9b44251add20
Private key:              0xa35f8534e883c1cf60b6c0d96863ac3eb54a9ce437708a1ac81762e268875312

Elapsed time: 61.497349083s
```

## General

`optivanity` provides vanity address generation functionality similar to that of the `aptos` CLI, but with assorted performance optimizations:

- Thread count argument for configurable execution parallelism (defaults to maximum possible parallelism)
- Byte-wise search, instead of expensive string-wise search like in the `aptos` CLI
- Build enhancements including [linker-time optimization](https://doc.rust-lang.org/cargo/reference/profiles.html#lto) and [code generation unit](https://doc.rust-lang.org/cargo/reference/profiles.html#codegen-units) minimization
- Minimal crate includes for reduced compile times compared with `aptos` CLI

Don't forget to use `cargo`'s [`--release` flag](https://doc.rust-lang.org/cargo/reference/profiles.html#release) for maximal build performance!

## Parallelism

The optional thread count argument controls how many independent search threads will be initiated during execution, and defaults to the maximum amount possible on your machine.
Two search threads running in parallel, for example, will on average take $\frac{1}{2}$ as long to generate an address compared with a single search thread, which is what the `aptos` CLI uses.
Three threads will on average take $\frac{1}{3}$ as long, four will on average take $\frac{1}{4}$ as long, and so on, **until the thread count equals the number of available cores**:
if your machine only has four available cores, you will not see performance increases for a five-thread search because you can only run four threads in parallel.

In other words, *only* specify thread count if you want to slow down the search for machine longevity.

## CPU load

The algorithms in `optivanity` were developed on a 2021 MacBook Pro with a ten-core [Apple M1 Max chip](https://en.wikipedia.org/wiki/Apple_M1#M1_Pro_and_M1_Max), where the optimal thread count for search speed is ten.
This is probably not the optimal thread count for machine longevity, however, because a ten-thread search results in the fan running full blast to prevent overheating.
Running with only six threads does not result in the fan noticeably turning on and is sufficient, for example, to generate an address with an eight-character vanity prefix overnight.

`optivanity` relies on a main watchdog thread that closes search threads once enough addresses have been generated.
Hence for the "Activity Monitor" app on the above machine, the following command results in the following readout:

```zsh
cargo run --release -- --prefix aaaaa --threads 6 --count 100
```

| Process Name | % CPU | Threads |
| ------------ | ----- | ------- |
| `optivanity` | 600   | 7       |

Here, six cores are each running a search thread at ~100% capacity, with a seventh non-search thread consuming almost no load.
Hence without other major processes running, this results in a user CPU load of about 60%.
