use std::env;
use waflz::errors::*;
use waflz::{find_link, irc_remote_title};

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(arg) = env::args().nth(1) {
        if let Some((protocol, link)) = find_link(&arg) {
            let reply = irc_remote_title(&protocol, &link).await?;
            println!("{}", reply);
        }
    }
    Ok(())
}
