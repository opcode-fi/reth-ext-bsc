# reth-ext-bsc

[`reth-ext`](https://github.com/opcode-fi/reth-ext) keeps chain state in your process so you
don't pay an RPC round trip for every account and slot you touch. That matters more on BSC
than it does on Ethereum.

BSC blocks are 450ms. That's 2.2 per second, about 26x Ethereum. Polling doesn't hold up at
that rate. Poll every second and you've already skipped a block, and what came back is stale
anyway. Polling faster doesn't fix it, you just pay more round trips to be equally behind.

The stream pushes instead. The node sends each commit as it makes it, and on this setup the
median time from a block being produced to us holding it is 0.30s. The consumer is the same
`rex-cons` and `rex-sim` you'd run against Ethereum, no changes.

This repo holds the deploy, the wire tests and an example consumer.

## Where things live

| repo | holds |
|---|---|
| [`opcode-fi/reth-ext`](https://github.com/opcode-fi/reth-ext) | `exex-proto`, `rex-cons`, `rex-sim` |
| [`opcode-fi/reth-bsc`](https://github.com/opcode-fi/reth-bsc) | the BSC node, and the rex bridge in `src/rex.rs` |
| this repo | deploy, wire tests, examples, docs |

The bridge sits in the node repo because it needs `BscPrimitives`. Putting it here would be a
dependency cycle, so don't try.

This repo is the only one that can depend on both of the others. reth-ext shouldn't know
about BSC, and the node fork shouldn't pull in `rex-cons`, so a test that checks the two ends
still agree on the wire has nowhere else to go. That's what it's for.

## The consumer doesn't change

The bridge reshapes `Chain<BscPrimitives>` into `Chain<EthPrimitives>` before it serialises,
so a stock `rex-cons` reads a BSC stream fine.

One field gets dropped: `BscBlockBody.sidecars`, the EIP-4844 blob payloads. Blob transactions
still come through, just not the blob contents. Everything else is intact, and receipts don't
even need converting since `BscPrimitives::Receipt` and `EthPrimitives::Receipt` are the same
type.

It's not there for convenience. `BscBlockBody` RLP-encodes the `sidecars` field even when it's
empty, so decoding with Ethereum primitives blows up with `ListLengthMismatch`.

If something downstream ever needs the blob sidecars, that conversion has to go and the
consumer needs real BSC primitives. Build that here.

## What the wire test is for

The two ends are built against different reth versions:

| | reth fork | tag | crate version |
|---|---|---|---|
| `reth-ext` | paradigmxyz/reth | `v2.3.0` | 2.3.0 |
| `reth-bsc` | bnb-chain/reth | `v0.1.2` | 2.2.0 |

The bincode layout is the same across both right now. `exex/types/src/notification.rs` and the
`serde_bincode_compat` parts of `chain.rs` and `execution_outcome.rs` are byte-identical
between the two checkouts. Nothing enforces that though, and when it breaks it breaks quietly:
`rex-cons` drops messages it can't decode and leaves the connection up, so you get a stream
that looks healthy and delivers nothing.

`tests/wire_roundtrip.rs` catches it. Run it against a live node whenever you bump reth on
either side.

## Layout

```
deploy/     compose + reth.toml, with the reasoning behind the BSC-specific values
docs/       which reth constants are wrong at 450ms blocks, and why
examples/   minimal consumer
tests/      wire guard (needs a running node, #[ignore] by default)
```

## Running

```sh
docker compose -f deploy/docker-compose.yml up -d --build
cargo run --example subscribe -- http://127.0.0.1:10001
cargo test -- --ignored          # wire guard, needs the node up
```
