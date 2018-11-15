use std::env;
use waflz::{irc_remote_title, find_link};

fn main() {
    if let Some(arg) = env::args().nth(1) {
        if let Some((protocol, link)) = find_link(&arg) {
            let title = irc_remote_title(&protocol, &link);
            println!("{:?}", title);
        }
    }
}
