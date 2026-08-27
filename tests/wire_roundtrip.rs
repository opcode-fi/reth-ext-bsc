//! Wire-compatibility guard between the BSC producer and the Ethereum consumer.
//!
//! These two are built against different reth generations: `reth-ext` against
//! paradigmxyz `v2.3.0` (crate 2.3.0), `reth-bsc` against bnb-chain `v0.1.2` (crate 2.2.0).
//! The bincode representation happens to be identical across them today, but nothing pins
//! that, and the failure mode is silent -- `rex-cons` skips undecodable messages and keeps
//! the stream open, so drift looks like a healthy connection delivering nothing.
//!
//! So the assertion that matters is not "we connected". It is "we decoded a notification
//! and the contents are structurally sane". A test that only checks connectivity would pass
//! through exactly the failure it exists to catch.
//!
//! Requires a running BSC node, so ignored by default:
//! ```sh
//! cargo test -- --ignored
//! ```

use rex_cons::{ExExNotification, NotificationStream};
use std::time::Duration;

fn endpoint() -> String {
    std::env::var("BSC_REX").unwrap_or_else(|_| "http://127.0.0.1:10001".to_string())
}

/// Decode real notifications off a live BSC producer.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a running BSC node"]
async fn bsc_stream_decodes_with_ethereum_consumer() {
    let mut stream = NotificationStream::new(endpoint());

    // BSC produces a block every ~450ms. Anything past a few seconds means we are not
    // decoding, not that the chain is quiet.
    let first = tokio::time::timeout(Duration::from_secs(30), stream.next())
        .await
        .expect("no decodable notification within 30s -- wire format drift, or node down");

    let tip = match &first {
        ExExNotification::ChainCommitted { new } => new.tip().number,
        ExExNotification::ChainReorged { new, .. } => new.tip().number,
        ExExNotification::ChainReverted { old } => old.tip().number,
    };
    assert!(tip > 0, "decoded a notification but its tip is 0");

    // Decoding one message could succeed on a payload that is empty or truncated, so also
    // require that the chain actually carries blocks and that the execution outcome -- the
    // part `rex-sim` consumes -- is present.
    if let ExExNotification::ChainCommitted { new } = &first {
        assert!(!new.blocks().is_empty(), "committed chain carried no blocks");
        let outcome = new.execution_outcome();
        assert_eq!(
            outcome.first_block,
            *new.blocks().keys().next().expect("non-empty checked above"),
            "execution outcome does not line up with the blocks it accompanies"
        );
    }
}

/// The stream must keep delivering, not just deliver once.
///
/// Catches a producer that emits a valid first message and then wedges -- which is what a
/// broadcast-lag bug or a stalled canonical stream would look like.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a running BSC node"]
async fn bsc_stream_is_continuous() {
    let mut stream = NotificationStream::new(endpoint());
    let mut seen = Vec::new();

    for _ in 0..5 {
        let n = tokio::time::timeout(Duration::from_secs(30), stream.next())
            .await
            .expect("stream stopped delivering");
        if let ExExNotification::ChainCommitted { new } = n {
            seen.push(new.tip().number);
        }
    }

    assert!(seen.len() >= 3, "expected several commits, saw {}", seen.len());
    // Monotonic, not contiguous: a reconnect resumes at the live tip, and the node may
    // commit a range at once when catching up. Both are legitimate; going backwards is not.
    for w in seen.windows(2) {
        assert!(w[1] > w[0], "tip went backwards: {} -> {}", w[0], w[1]);
    }
}
