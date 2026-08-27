//! Minimal BSC stream consumer.
//!
//! Prints one line per canonical notification. Useful for answering "is the node actually
//! feeding us, and is it at the tip" without pulling in `rex-sim`.
//!
//! ```sh
//! cargo run --example subscribe -- http://127.0.0.1:10001
//! ```
//!
//! Note the port: the BSC node publishes on host `10001` because the Ethereum node already
//! owns `10000`. Inside the container both are `10000`.

use rex_cons::{ExExNotification, NotificationStream};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let endpoint =
        std::env::args().nth(1).unwrap_or_else(|| "http://127.0.0.1:10001".to_string());
    println!("subscribing to {endpoint}");

    let mut stream = NotificationStream::new(endpoint);
    let mut last: Option<u64> = None;

    loop {
        let notification = stream.next().await;
        match notification {
            ExExNotification::ChainCommitted { new } => {
                let tip = new.tip().number;
                // A gap here is not necessarily loss: `NotificationStream` reconnects at the
                // live tip, so a dropped connection shows up as a jump. It is still worth
                // printing, because it is indistinguishable from the node falling behind and
                // catching up in a burst -- both matter to a consumer.
                if let Some(prev) = last {
                    if tip > prev + 1 {
                        println!("  gap: {} -> {} ({} blocks)", prev, tip, tip - prev - 1);
                    }
                }
                last = Some(tip);
                println!("commit  {tip}  ({} blocks)", new.blocks().len());
            }
            ExExNotification::ChainReorged { old, new } => {
                println!("reorg   {} -> {}", old.tip().number, new.tip().number);
                last = Some(new.tip().number);
            }
            ExExNotification::ChainReverted { old } => {
                println!("revert  {}", old.tip().number);
            }
        }
    }
}
