# Ethereum-calibrated constants that are wrong on BSC

BSC block time, measured over 5,000 blocks on 2026-08-27: **0.450s** — 2.22 blocks/sec,
about **26x** Ethereum's rate.

Nearly every problem running reth on BSC traced back to a constant that is sensible at 12s
blocks and pathological at 0.45s. They are collected here because the pattern repeats: when
something misbehaves, first ask what the value means in *blocks* rather than seconds.

| constant | Ethereum meaning | BSC meaning | where |
|---|---|---|---|
| ExEx buffer 1024 | ~3.5 hours of blocks | ~8 minutes | reth `DEFAULT_EXEX_MANAGER_CAPACITY` |
| execution `max_duration` 10m | ~50 blocks | ~1,300 blocks | reth `ExecutionConfig` |
| merkle `incremental_threshold` 7,000 | ~23 hours behind | ~54 minutes behind | reth `MerkleConfig` |
| `FETCH_TIMEOUT` 5s | under half a slot | ~11 blocks | reth-bsc `fork_recover` |
| `FAILED_HEAD_COOLDOWN` 30s | 2.5 slots | ~66 blocks | reth-bsc `fork_recover` |

## The two that caused outages

**ExEx buffer.** The ExEx manager takes many notifications per poll, WAL-committing each,
but delivers only one per ExEx per poll. At 0.45s blocks the 1024-slot buffer pinned full
and the node was OOM-killed. Fixed by not using an ExEx at all: the bridge reads
`canonical_state_stream()` from a plain critical task.

**Merkle incremental threshold.** Above a 7,000-block gap, MerkleExecute abandons the
incremental trie update and does a full rebuild. On Ethereum that gap means the node has
been down the better part of a day. On BSC it means it fell behind for under an hour — an
ordinary event. So routine contention triggered a full trie rebuild, which then could not
complete within the memory limit. Raised to 100,000 (~13 hours of BSC).

## The one that was a misreading, not a constant

`PayloadStatusEnum::Syncing` from engine-tree means *"parent missing, payload buffered, I
will attach it when the parent lands"*. reth-bsc read it as *"go fetch the parent"* and
launched a network round-trip plus re-import.

At 12s slots the race never happens — block N is long executed when N+1 arrives. At 0.45s it
is the common case. Measured over six minutes: **1,685 recovery spawns (~4.7/s)**, of which
**99.3% had ancestor-walk depth 0** and **77% targeted a block already at our own tip**. Not
one was a real fork.

The lesson generalises: a status that is effectively unreachable on Ethereum can be the
steady state on BSC.

## Peer fetch latency

Per-peer median latency for block fetches spans **0.12s to 5.63s** — a 47x range. Eight of
eighteen well-sampled peers exceeded a 2s median; between them they served 53% of fetches
and burned **70% of all fetch time**.

Worth recording because the obvious fix is wrong: spreading fetches round-robin across all
peers made things **much** worse (blocks-behind median 0.8 → 27.7, recovery p90 7.55s →
17.61s). Most peers are slower than whichever one you were already using, so any policy that
reassigns work without regard to measured speed loses. Selection has to be latency-aware or
not change at all.

## What the node actually does

Measured 2026-08-27 over 10.7 hours, at 35-45 peers:

- median **1 block** behind — i.e. at the tip
- but **~43% of samples ≥3 blocks behind**, p90 73, worst 291
- excursions ~17/hour, median 60s, worst 9.5 minutes
- median production→commit latency **0.30s**, well inside one block

The shape matters more than the average: this is not a node running steadily behind, which
you could compensate for. It is a node at the tip punctuated by brief blackouts, each one a
block that did not arrive promptly with everything behind it queued.

Root mechanism: ~1.8% of blocks take longer than the 450ms budget in `new_payload` (p90
1.2s, p99 3.7s, worst 15.5s). Those are **ordinary blocks** — 1.04x the gas of random
controls — so it is contention, not heavy work. Unresolved.
