pub mod config;
pub mod errors;

use crate::errors::*;
use kuchiki::traits::TendrilSink;
use maxminddb::geoip2;
use regex::Regex;

const DOWNLOAD_THRESHOLD: u64 = 1024 * 1024 * 5;
const LINK_REGEX: &str =
    r"(http|https)://([\w_-]+(?:(?:\.[\w_-]+)+))([\w.,@?^=%&:/~+#-]*[\w@?^=%&/~+#-])?";

fn prepare_title(title: &str) -> String {
    let title = title.trim();

    let mut last_was_whitespace = true;

    title.chars().fold(String::new(), |mut acc, x| {
        if x.is_whitespace() {
            if !last_was_whitespace {
                acc.push(' ');
                last_was_whitespace = true;
            }
        } else {
            acc.push(x);
            last_was_whitespace = false;
        }
        acc
    })
}

fn find_title(r: &str) -> Option<String> {
    let doc = kuchiki::parse_html().one(r);

    let mut nodes = match doc.select("title") {
        Ok(nodes) => nodes,
        Err(_) => return None,
    };

    let title = nodes.next()?;

    let as_node = title.as_node();

    let text_node = match as_node.first_child() {
        Some(node) => node,
        None => return None,
    };
    let text = match text_node.as_text() {
        Some(node) => node.borrow(),
        None => return None,
    };

    Some(prepare_title(&text.to_owned()))
}

pub async fn irc_remote_title(protocol: &str, link: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)")
        .build()?;

    let r = client.get(link).send().await?;

    let headers = r.headers();
    let content_type = headers
        .get("content-type")
        .map(|s| s.to_str().unwrap_or("application/octet-stream"))
        .unwrap_or("application/octet-stream")
        .to_string();

    let extra = {
        let mut title = String::new();

        if protocol == "http" {
            title += " \x02\x034[http]\x0f";
        };

        if headers.get("strict-transport-security").is_some() {
            title += " \x02\x033[hsts]\x0f";
        };

        if headers.get("content-security-policy").is_some() {
            title += " \x02\x032[csp]\x0f";
        };

        if headers.get("content-security-policy-report-only").is_some() {
            title += " \x02\x032[csp(ro)]\x0f";
        };

        if let Some(remote) = r.remote_addr() {
            let ip = remote.ip();

            let geoip_db_path = "./GeoLite2-Country.mmdb";
            let reader = maxminddb::Reader::open_readfile(geoip_db_path)
                .with_context(|| anyhow!("Failed to open geoip database: {:?}", geoip_db_path))?;
            if let Ok(geoip) = reader.lookup::<geoip2::Country>(ip) {
                if let Some(country) = geoip.country {
                    if let Some(code) = country.iso_code {
                        title += &format!(" ({})", code);
                    }
                }
            }
        }

        title
    };

    if let Some(len) = r.content_length() {
        if len >= DOWNLOAD_THRESHOLD {
            let title = preview_download(&content_type, len);
            return Ok(format!("{}{}", title, extra));
        }
    }

    let body = r.text().await?;

    let title = if let Some(title) = find_title(&body) {
        format!("{:?}", title) // TODO: nicer escaping
    } else {
        preview_download(&content_type, body.len() as u64)
    };

    Ok(format!("{}{}", title, extra))
}

fn preview_download(content_type: &str, size: u64) -> String {
    let size = humansize::format_size(size, humansize::BINARY);
    format!("{:?} - {}", content_type, size)
}

pub fn find_link(msg: &str) -> Option<(String, String)> {
    let re = Regex::new(LINK_REGEX).unwrap();

    let cap = re.captures(msg)?;
    let link = String::from(&cap[0]);
    let protocol = String::from(&cap[1]);

    Some((protocol, link))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_title() {
        let title = irc_remote_title("https", "https://github.com/")
            .await
            .unwrap();
        assert_eq!(title.as_str(),
            "\"GitHub: Let’s build from here · GitHub\" \u{2}\u{3}3[hsts]\u{f} \u{2}\u{3}2[csp]\u{f} (US)");
    }
}
