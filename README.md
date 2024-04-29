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

Usage: optivanity [OPTIONS]

Options:
  -p, --prefix <PREFIX>    Address prefix to match (no leading `0x`). Each additional character slows search by 16x
  -s, --suffix <SUFFIX>    Address suffix to match. Each additional character slows search by 16x
  -m, --multisig           Use this flag if you want to search for multisig address(es)
  -c, --count <COUNT>      Number of vanity accounts to generate [default: 1]
  -t, --threads <THREADS>  Number of threads to use. Only specify if you want to use fewer cores than available [default: 10]
  -h, --help               Print help
```

### Ed25519 vanity address generation

```zsh
# Generate a single standard account address starting with aaaaa, maximum parallelism
% cargo run --release -- --prefix aaaaa
Standard account address: 0xaab0eba3bc47066f6a3e9d0086f8d816b5590fe3bd8901143da269aa7887e2aa
Private key:              0x640d3c18524a11af3461d2d3252d0b54e09c897e1db53185f904e25dec4c49f9

Elapsed time: 34.772661ms
Total addresses generated: 19214
```

### Octa-core multisig vanity address generation

```zsh
# Generate 3 multisig account addresses starting with bbbbbb, parallelized across 8 cores
% cargo run --release -- --prefix bbbb --multisig --count 3 --threads 8
Multisig account address: 0xbbbb60b209c9115aed317b5e625c00be02cf4759d9a7f0a80ec5713afab1a46d
Standard account address: 0x01cceb1533cd8502bbee964b6f61cf2c97802fe02c1bd566208dec3aeb84b312
Private key:              0x28fceaad60c41da43509fc53646e879e3e0063c814ca01dc607627d6d0c5a7b6

Multisig account address: 0xbbbb44d05e29c1441f0ed2c0cbd51d1e05a933790059d984fb5ef551714e3060
Standard account address: 0x556365fcc5239c5c1b6df2aaea7e05391de657d0fc052dd4a3f193747e66765b
Private key:              0xde9e028a071a4b3de8066294a0d16639853819d56744b01d313e1d58c1ec9b45

Multisig account address: 0xbbbbdcf9d8df88dc5669f4ef970685ba36bcf161d0eecd32db917c9d29102f31
Standard account address: 0xad07cb201013bf3d7947130973bf430be51eabba1313a7adf58a870bc33793f7
Private key:              0x34565d5df3da025423da9719807b552f642dcd1f28621d9b1044db0c83e6a2ec

Elapsed time: 354.077237ms
Total addresses generated: 190621
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
