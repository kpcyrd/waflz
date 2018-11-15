use waflz::config;
use waflz::{irc_remote_title, find_link};
use waflz::errors::*;
use env_logger::Env;
use std::default::Default;
use irc::client::prelude::*;

fn main() -> Result<()> {
    env_logger::init_from_env(Env::default()
        .default_filter_or("info"));

    let config = config::load_from("config.toml")
        .context("Failed to load config from config.toml")?;

    let irc_config = Config {
        nickname: Some(config.irc.nickname),
        nick_password: Some(config.irc.password),
        server: Some(config.irc.server),
        port: Some(config.irc.port.unwrap_or(6697)),
        use_ssl: Some(true),
        ..Default::default()
    };

    let server = IrcClient::from_config(irc_config).unwrap();
    println!("[+] connected...");
    server.identify().unwrap();
    println!("[+] authenticating...");

    let channels = config.irc.channels;
    let readonly_channels = config.irc.readonly_channels;

    server.for_each_incoming(|message| {
        debug!("incoming: {:?}", message);

        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                println!("PRIVMSG: {:?}: {:?}", target, msg);

                if readonly_channels.contains(target) {
                    return;
                }

                if msg.starts_with("waflz") {

                    server.send_privmsg(target, ":)").unwrap();

                } else if let Some((protocol, link)) = find_link(msg) {

                    println!("found link: {:?}", link);

                    if let Ok(title) = irc_remote_title(&protocol, &link) {
                        server.send_privmsg(target, &title).unwrap();
                    }
                }
            },
            // JOIN
            // MODE
            Command::TOPIC(ref target, ref msg) => {
                println!("TOPIC: {:?}: {:?}", target, msg);
            },
            Command::NOTICE(ref _target, ref msg) => {
                if let Some(prefix) = message.prefix {
                    if prefix == "NickServ!NickServ@services.hackint.org" && msg.starts_with("You are now identified ") {
                        println!("[+] login successful");
                        for channel in &channels {
                            println!("[*] joining {:?}", channel);
                            server.send_join(channel).unwrap();
                        }
                    }
                }
            },
            _ => (),
        };
    }).expect("msg loop failed");

    Ok(())
}
