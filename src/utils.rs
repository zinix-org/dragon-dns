use chrono::Duration;
use color_eyre::Result;
use std::net::Ipv4Addr;

pub fn format_duration(duration: &Duration) -> String {
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;

    let mut result = String::new();

    if days > 0 {
        result.push_str(&format!("{}d ", days));
    }
    if hours > 0 {
        result.push_str(&format!("{}h ", hours));
    }
    if minutes > 0 {
        result.push_str(&format!("{}m", minutes));
    }

    result.trim_end().to_string()
}

pub async fn get_ip4() -> Result<Ipv4Addr> {
    let ip4 = reqwest::Client::new()
        .get("https://checkip.amazonaws.com")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?
        .text()
        .await?;
    let ip4 = ip4.trim().parse()?;
    Ok(ip4)
}
