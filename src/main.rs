use data_encoding::BASE64;
use env_logger::Env;
use futures::prelude::*;
use irc::client::prelude::*;
use irc::proto::command::CapSubCommand;
use std::default::Default;
use tokio::time;
use waflz::config;
use waflz::errors::*;
use waflz::{find_link, irc_remote_title};

const HTTP_PREVIEW_TIMEOUT: time::Duration = time::Duration::from_secs(10);
const IRC_AUTH_TIMEOUT: time::Duration = time::Duration::from_secs(300);

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
        nickname: Some(config.irc.nickname.clone()),
        server: Some(config.irc.server),
        port: Some(config.irc.port.unwrap_or(6697)),
        use_tls: Some(true),
        ..Default::default()
    };

    let mut client = Client::from_config(irc_config).await?;
    println!("[+] connected...");
    println!("[+] authenticating...");

    let channels = config.irc.channels;
    let readonly_channels = config.irc.readonly_channels;

    let mut stream = client.stream()?;
    let sender = client.sender();

    match time::timeout(IRC_AUTH_TIMEOUT, async {
        if let Some(password) = &config.irc.password {
            client.send_cap_req(&[Capability::Sasl])?;
            while let Some(message) = stream.next().await.transpose()? {
                debug!("incoming: {:?}", message);

                match message.command {
                    Command::CAP(_, subcommand, ref args, _) => {
                        if subcommand == CapSubCommand::ACK
                            && let Some(args) = args
                            && args.contains("sasl")
                        {
                            println!("[+] ircd confirmed sasl capability");
                            client.send_sasl_plain()?;
                        }
                    }

                    Command::AUTHENTICATE(ref data) => {
                        if data == "+" {
                            println!("[+] sending credentials");
                            let auth = BASE64.encode(
                                format!("\0{}\0{}", config.irc.nickname, password).as_bytes(),
                            );
                            client.send_sasl(auth)?;
                        }
                    }
                    Command::Response(Response::RPL_SASLSUCCESS, _) => {
                        println!("[+] login successful");
                        client.identify()?;
                    }
                    Command::Response(Response::RPL_WELCOME, _) => {
                        debug!("Received welcome indicator from ircd, ending auth timeout");
                        break;
                    }
                    _ => (),
                }
            }
        } else {
            client.identify()?;
        }
        Result::<(), Error>::Ok(())
    })
    .await
    {
        Ok(Ok(_)) => (),
        Ok(Err(err)) => return Err(err),
        Err(err) => return Err(err).context("Authentication timeout"),
    }

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
                join(&sender, &channels).await?;
            }
            _ => (),
        }
    }

    Ok(())
}
