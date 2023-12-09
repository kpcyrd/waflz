use env_logger::Env;
use futures::prelude::*;
use irc::client::prelude::*;
use std::default::Default;
use tokio::time;
use waflz::config;
use waflz::errors::*;
use waflz::{find_link, irc_remote_title};

const HTTP_PREVIEW_TIMEOUT: time::Duration = time::Duration::from_secs(10);

async fn join(sender: &Sender, channels: &[String]) -> Result<()> {
    for channel in channels {
        println!("[*] joining {:?}", channel);
        sender.send_join(channel)?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let config =
        config::load_from("config.toml").context("Failed to load config from config.toml")?;

    let irc_config = Config {
        nickname: Some(config.irc.nickname),
        nick_password: config.irc.password.clone(),
        server: Some(config.irc.server),
        port: Some(config.irc.port.unwrap_or(6697)),
        use_tls: Some(true),
        ..Default::default()
    };

    let mut client = Client::from_config(irc_config).await?;
    println!("[+] connected...");
    client.identify().unwrap();
    println!("[+] authenticating...");

    let channels = config.irc.channels;
    let readonly_channels = config.irc.readonly_channels;

    let mut stream = client.stream()?;
    let sender = client.sender();

    while let Some(message) = stream.next().await.transpose()? {
        debug!("incoming: {:?}", message);

        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                let prefix = match &message.prefix {
                    Some(Prefix::ServerName(name)) => name.as_ref(),
                    Some(Prefix::Nickname(name, _, _)) => name.as_ref(),
                    None => "-",
                };
                println!("[>]: {target:?} {prefix:?}: {msg:?}");

                if readonly_channels.contains(target) {
                    continue;
                }

                if msg.starts_with("waflz") {
                    sender.send_privmsg(target, ":)").unwrap();
                } else if let Some((protocol, link)) = find_link(msg) {
                    println!("found link: {:?}", link);

                    let future = irc_remote_title(&protocol, &link);
                    let future = time::timeout(HTTP_PREVIEW_TIMEOUT, future);

                    if let Ok(Ok(title)) = future.await {
                        sender.send_privmsg(target, &title).unwrap();
                    }
                }
            }
            // JOIN
            // MODE
            Command::TOPIC(ref target, ref msg) => {
                println!("TOPIC: {:?}: {:?}", target, msg);
            }
            Command::Response(Response::RPL_ENDOFMOTD, _) => {
                if config.irc.password.is_none() {
                    join(&sender, &channels).await?;
                }
            }
            Command::NOTICE(ref _target, ref msg) => {
                if let Some(prefix) = message.prefix {
                    if prefix.to_string() == "NickServ!NickServ@services.hackint.org"
                        && msg.starts_with("You are now identified ")
                    {
                        println!("[+] login successful");
                        join(&sender, &channels).await?;
                    }
                }
            }
            _ => (),
        }
    }

    Ok(())
}
