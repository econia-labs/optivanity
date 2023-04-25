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

Usage: optivanity [OPTIONS] <PREFIX>

Arguments:
  <PREFIX>  Address prefix to match (no leading 0x). Each additional character slows search by 16x

Options:
  -m, --multisig           Use this flag if you want to search for multisig address(es)
  -c, --count <COUNT>      Number of vanity accounts to generate [default: 1]
  -t, --threads <THREADS>  Number of threads to use. Only specify if you want to use fewer cores than available [default: 10]
  -h, --help               Print help
```

### Ed25519 vanity address generation

```zsh
# Generate a single standard account address starting with aaaaa, maximum parallelism
% cargo run --release -- aaaaa

Starting search at 2023-04-25T13:13:34.156231-07:00

Standard account address: 0xaaaaa3248e447b8bd61eff40a5a215da10b3c365709aedbe7f391e2c5249d496
Private key:              0x6562b0a50fb07ef8c1c4437c4ae94a31691f9ac89e97d53b75722d38c94fe2fc

Elapsed time: 2.032645459s
```

### Octa-core multisig vanity address generation

```zsh
# Generate 3 multisig account addresses starting with bbbbbb, parallelized across 8 cores
% cargo run --release -- bbbbbb --multisig --count 3 --threads 8

Starting search at 2023-04-25T13:15:04.699368-07:00

Multisig account address: 0xbbbbbb532e62e320454b33819cce3983f6c4575189a7b35f61d8e8c95b87696b
Standard account address: 0xd7fcaf7ae574f038e1d5ae8430b8c46299351a4c943a846a58766e44b8be1b55
Private key:              0xe71063e6b8ef27e46aafed04337ec9660e46677cc57f6b6b0981186f38f5ad09

Multisig account address: 0xbbbbbbb30c013a24e176ef3e9399a85d957cf9e6204487d3b20e33d89c7d7a70
Standard account address: 0x0ba8cccb9701d5806b8ad1e15cc16f13700e976cc8540c1c7192fba539399664
Private key:              0x2d7f1658e71985d5b175b1361aea7aa19bddc6e4d297f8cae8df3956fb79c44d

Multisig account address: 0xbbbbbb4b158d3ea973d6ab2a48b0f33ee4e553460f45db379ab3a96b0fa7ca5c
Standard account address: 0xef072248921951c00eb65b4fe9daa4f1753e853cbc3e07be45c21d5e9bcd2ce0
Private key:              0x9aa178ff8a0afa3ef78944147307c1beb08ac4dc87bd3d1784067fa353575cb7

Elapsed time: 68.436100916s
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
Hence `optivanity` will return with an error if you try to initiate a search with a thread count in excess of available parallelism.

In other words, *only* specify thread count if you want to slow down the search for machine longevity.

## CPU load

The algorithms in `optivanity` were developed on a 2021 MacBook Pro with a ten-core [Apple M1 Max chip](https://en.wikipedia.org/wiki/Apple_M1#M1_Pro_and_M1_Max), where the optimal thread count for search speed is ten.
This is probably not the optimal thread count for machine longevity, however, because a ten-thread search results in the fan running full blast to prevent overheating.
Running with only six threads does not result in the fan noticeably turning on and is sufficient, for example, to generate an address with an eight-character vanity prefix overnight.

`optivanity` relies on a main watchdog thread that closes search threads once enough addresses have been generated.
Hence in the "Activity Monitor" app for the above machine, `cargo run --release -- aaaaa --threads 6 --count 100` results in the following:

| Process Name | % CPU | Threads |
| ------------ | ----- | ------- |
| `optivanity` | 600   | 7       |

Here, six cores are each running a search thread at ~100% capacity, with a seventh non-search thread consuming almost no load.
Hence without other major processes running, this results in a user CPU load of slightly above 60%.
