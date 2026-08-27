//! Wire guard: a BSC producer must stay decodable by the Ethereum consumer.
//! Rationale and the obligation to run this after a reth bump: see AGENTS.md.

use rex_cons::{ExExNotification, NotificationStream};
use std::time::Duration;

const DECODE_TIMEOUT: Duration = Duration::from_secs(30);

fn endpoint() -> String {
    std::env::var("BSC_REX").unwrap_or_else(|_| "http://127.0.0.1:10001".to_string())
}

fn tip_of(notification: &ExExNotification) -> u64 {
    match notification {
        ExExNotification::ChainCommitted { new } => new.tip().number,
        ExExNotification::ChainReorged { new, .. } => new.tip().number,
        ExExNotification::ChainReverted { old } => old.tip().number,
    }
}

/// A notification decodes, and carries blocks whose execution outcome lines up with them.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a running BSC node"]
async fn bsc_stream_decodes_with_ethereum_consumer() {
    let mut stream = NotificationStream::new(endpoint());

    let first = tokio::time::timeout(DECODE_TIMEOUT, stream.next())
        .await
        .expect("no decodable notification: wire format drift, or node down");

    assert!(tip_of(&first) > 0);

    if let ExExNotification::ChainCommitted { new } = &first {
        assert!(!new.blocks().is_empty());
        assert_eq!(
            new.execution_outcome().first_block,
            *new.blocks().keys().next().expect("blocks are non-empty"),
        );
    }
}

/// The stream keeps delivering, and the tip never goes backwards.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a running BSC node"]
async fn bsc_stream_is_continuous() {
    let mut stream = NotificationStream::new(endpoint());
    let mut tips = Vec::new();

    for _ in 0..5 {
        let notification = tokio::time::timeout(DECODE_TIMEOUT, stream.next())
            .await
            .expect("stream stopped delivering");
        if let ExExNotification::ChainCommitted { new } = notification {
            tips.push(new.tip().number);
        }
    }

    assert!(tips.len() >= 3, "saw {} commits", tips.len());
    for pair in tips.windows(2) {
        assert!(pair[1] > pair[0], "tip went backwards: {} -> {}", pair[0], pair[1]);
    }
}
