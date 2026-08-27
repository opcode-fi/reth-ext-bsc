# reth-ext-bsc

Deployment, integration tests and consumer examples for streaming **BSC** canonical
blocks into the `reth-ext` consumer stack.

## Where things live

Three repos, and the split is not arbitrary — each one is where it is because it is the
only place that *can* hold it.

| repo | holds | why there |
|---|---|---|
| [`opcode-fi/reth-ext`](https://github.com/opcode-fi/reth-ext) | `exex-proto`, `rex-cons`, `rex-sim` | chain-agnostic. The consumer needs no BSC awareness at all (see below). |
| [`opcode-fi/reth-bsc`](https://github.com/opcode-fi/reth-bsc) | the BSC node + the rex bridge (`src/rex.rs`) | the bridge needs `BscPrimitives`, so it cannot live outside the node crate without a dependency cycle. |
| **this repo** | deploy, integration tests, examples, docs | the only repo that can depend on **both** of the above. |

That last row is the whole reason this repo exists. `reth-ext` must not depend on BSC, and
the node fork must not depend on `rex-cons` — so a test that a BSC producer and an Ethereum
consumer still agree on the wire has nowhere else to go.

## The consumer needs no changes

`reth-bsc`'s bridge reshapes each `Chain<BscPrimitives>` into `Chain<EthPrimitives>` before
serialising, so an unmodified `rex-cons` decodes a BSC stream.

It drops exactly one field: `BscBlockBody.sidecars` — the EIP-4844 blob payloads. Blob
*transactions* survive; only the blob contents are lost. Everything else crosses intact,
and receipts need no conversion at all because `BscPrimitives::Receipt` and
`EthPrimitives::Receipt` are the same type.

The conversion is not a convenience. `BscBlockBody`'s RLP emits the `sidecars` field even
when empty, so a consumer decoding with Ethereum primitives fails with
`ListLengthMismatch`.

**If anything downstream ever needs blob sidecars, that conversion has to go and the
consumer must gain BSC-aware primitives.** At that point this repo is where the
BSC-specific consumer would live.

## The hazard this repo exists to catch

The two sides are built against different reth generations:

| | reth fork | tag | crate version |
|---|---|---|---|
| `reth-ext` | paradigmxyz/reth | `v2.3.0` | 2.3.0 |
| `reth-bsc` | bnb-chain/reth | `v0.1.2` | 2.2.0 |

The bincode wire format happens to be identical across them — `exex/types/src/notification.rs`
and the `serde_bincode_compat` sections of `chain.rs` and `execution_outcome.rs` are
byte-identical between the two checkouts. **But nothing pins or tests that**, and the failure
mode is silent: `rex-cons` skips messages it cannot decode and keeps the stream open, so a
format drift presents as a healthy connection delivering nothing, forever.

`tests/wire_roundtrip.rs` is the guard. Run it against a live node after any reth bump on
either side.

## Layout

```
deploy/     compose + reth.toml, with the measured reasoning for every BSC-specific value
docs/       why the BSC constants differ from Ethereum's
examples/   minimal consumer
tests/      wire compatibility guard (needs a running node; #[ignore] by default)
```

## Running

```sh
docker compose -f deploy/docker-compose.yml up -d --build
cargo run --example subscribe -- http://127.0.0.1:10001
cargo test -- --ignored          # wire guard, needs the node up
```
