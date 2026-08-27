//! `cargo run --example subscribe -- http://127.0.0.1:10001`

use rex_cons::{ExExNotification, NotificationStream};

const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:10001";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let endpoint = std::env::args().nth(1).unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());
    println!("subscribing to {endpoint}");

    let mut stream = NotificationStream::new(endpoint);
    let mut previous_tip: Option<u64> = None;

    loop {
        match stream.next().await {
            ExExNotification::ChainCommitted { new } => {
                let tip = new.tip().number;
                if let Some(previous) = previous_tip {
                    if tip > previous + 1 {
                        println!("  gap {previous} -> {tip} ({} blocks)", tip - previous - 1);
                    }
                }
                previous_tip = Some(tip);
                println!("commit  {tip}  ({} blocks)", new.blocks().len());
            }
            ExExNotification::ChainReorged { old, new } => {
                previous_tip = Some(new.tip().number);
                println!("reorg   {} -> {}", old.tip().number, new.tip().number);
            }
            ExExNotification::ChainReverted { old } => {
                println!("revert  {}", old.tip().number);
            }
        }
    }
}
