use std::env;
use waflz::errors::*;
use waflz::{irc_remote_title, find_link};

fn main() -> Result<()> {
    if let Some(arg) = env::args().nth(1) {
        if let Some((protocol, link)) = find_link(&arg) {
            let reply = irc_remote_title(&protocol, &link)?;
            println!("{}", reply);
        }
    }
    Ok(())
}
