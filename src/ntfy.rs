// src/ntfy.rs
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct NtfyConfig {
    pub base_url: String,
    pub topic: String,
    pub auth: Option<(String, String)>,
}

impl NtfyConfig {
    pub fn from_url(url: &str) -> Self {
        // Parse URL like:
        // - http://localhost:8080/whale-alerts
        // - http://user:pass@localhost:8080/whale-alerts
        // - https://ntfy.sh/whale-alerts
        
        let url_lower = url.to_lowercase();
        let has_auth = url.contains('@');
        
        if has_auth {
            // Extract auth credentials
            let protocol_end = url.find("://").unwrap_or(0) + 3;
            let at_pos = url.find('@').unwrap();
            
            let auth_part = &url[protocol_end..at_pos];
            let (user, pass) = if auth_part.contains(':') {
                let parts: Vec<&str> = auth_part.split(':').collect();
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (auth_part.to_string(), "".to_string())
            };
            
            // Extract base URL and topic
            let rest = &url[at_pos + 1..];
            let slash_pos = rest.find('/').unwrap_or(rest.len());
            
            let base_url = if url_lower.starts_with("https://") {
                format!("https://{}", &rest[..slash_pos])
            } else {
                format!("http://{}", &rest[..slash_pos])
            };
            
            let topic = if slash_pos < rest.len() {
                rest[slash_pos + 1..].to_string()
            } else {
                "whale-alerts".to_string()
            };
            
            NtfyConfig {
                base_url,
                topic,
                auth: Some((user, pass)),
            }
        } else {
            // No auth credentials
            let protocol_end = url.find("://").unwrap_or(0);
            let rest = if protocol_end > 0 {
                &url[protocol_end + 3..]
            } else {
                url
            };
            
            let slash_pos = rest.find('/').unwrap_or(rest.len());
            
            let base_url = if url_lower.starts_with("https://") {
                format!("https://{}", &rest[..slash_pos])
            } else if url_lower.starts_with("http://") {
                format!("http://{}", &rest[..slash_pos])
            } else {
                // Assume it's just a topic on localhost
                return NtfyConfig {
                    base_url: "http://localhost:8080".to_string(),
                    topic: url.to_string(),
                    auth: None,
                };
            };
            
            let topic = if slash_pos < rest.len() {
                rest[slash_pos + 1..].to_string()
            } else {
                "whale-alerts".to_string()
            };
            
            NtfyConfig {
                base_url,
                topic,
                auth: None,
            }
        }
    }
}

pub async fn send_ntfy_alert(
    config: &NtfyConfig,
    platform: &str,
    market_title: Option<&str>,
    outcome: Option<&str>,
    side: &str,
    value: f64,
    price: f64,
    size: f64,
    timestamp: &str,
    wallet_id: Option<&str>,
    wallet_activity: Option<&crate::types::WalletActivity>,
) {
    let is_sell = side.to_uppercase() == "SELL";
    
    // Build title
    let title = if is_sell {
        "🚨 WHALE EXITING POSITION"
    } else {
        "🐋 WHALE ENTRY DETECTED"
    };
    
    // Build message
    let mut message_lines = Vec::new();
    
    message_lines.push(format!("Platform: {}", platform));
    message_lines.push(format!("Market: {}", market_title.unwrap_or("Unknown")));
    
    if let Some(outcome_str) = outcome {
        message_lines.push(format!("Action: {} {}", side.to_uppercase(), outcome_str));
    } else {
        message_lines.push(format!("Action: {}", side.to_uppercase()));
    }
    
    message_lines.push(format!("Amount: ${:.2}", value));
    message_lines.push(format!("Price: ${:.4} ({:.1}%)", price, price * 100.0));
    message_lines.push(format!("Size: {:.0} contracts", size));
    
    if let Some(wallet) = wallet_id {
        // Shorten wallet address for display
        let short_wallet = if wallet.len() > 10 {
            format!("{}...{}", &wallet[..6], &wallet[wallet.len()-4..])
        } else {
            wallet.to_string()
        };
        message_lines.push(format!("Wallet: {}", short_wallet));
    }
    
    // Add wallet activity if available
    if let Some(activity) = wallet_activity {
        message_lines.push("".to_string()); // Empty line
        message_lines.push("Wallet Activity:".to_string());
        message_lines.push(format!("├─ Txns (1h): {}", activity.transactions_last_hour));
        message_lines.push(format!("├─ Txns (24h): {}", activity.transactions_last_day));
        message_lines.push(format!("├─ Volume (1h): ${:.2}", activity.total_value_hour));
        message_lines.push(format!("├─ Volume (24h): ${:.2}", activity.total_value_day));
        
        let status = if activity.is_heavy_actor {
            "HEAVY ACTOR ⚠️"
        } else if activity.is_repeat_actor {
            "REPEAT ACTOR 🔄"
        } else {
            "NEW ACTOR"
        };
        message_lines.push(format!("└─ Status: {}", status));
    }
    
    let message = message_lines.join("\n");
    
    // Create payload
    let mut payload = json!({
        "topic": config.topic,
        "title": title,
        "message": message,
        "priority": if is_sell { 4 } else { 3 }, // 4=high, 3=default
        "tags": if is_sell { vec!["red_circle", "warning"] } else { vec!["whale", "moneybag"] },
    });
    
    // Add click action based on platform
    if platform == "Polymarket" {
        if let Some(market) = market_title {
            let market_slug = market
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
                .collect::<String>()
                .replace("--", "-")
                .trim_matches('-')
                .to_string();
            
            if !market_slug.is_empty() {
                payload["click"] = json!(format!("https://polymarket.com/markets/{}", market_slug));
            }
        }
    } else if platform == "Kalshi" {
        payload["click"] = json!("https://kalshi.com/markets");
    }
    
    // Add timestamp
    payload["time"] = json!(timestamp);
    
    // Send to ntfy
    let url = format!("{}/{}", config.base_url, config.topic);
    
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true) // For self-signed certs
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            eprintln!("{} Failed to create HTTP client: {}", "[NTFY]".red(), e);
            return;
        }
    };
    
    let mut request = client.post(&url).json(&payload);
    
    // Add auth if provided
    if let Some((user, pass)) = &config.auth {
        request = request.basic_auth(user, Some(pass));
    }
    
    match request.send().await {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                eprintln!(
                    "{} Ntfy error: {} - {}",
                    "[NTFY]".yellow(),
                    status,
                    error_text
                );
            } else {
                // Success!
                eprintln!("{} Notification sent to ntfy", "[NTFY]".green());
            }
        }
        Err(e) => {
            eprintln!("{} Failed to send: {}", "[NTFY]".red(), e);
        }
    }
}

// Test function
pub async fn test_ntfy(config: &NtfyConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing ntfy connection to: {}/{}", config.base_url, config.topic);
    
    let test_payload = json!({
        "topic": config.topic,
        "title": "🐋 Whale Watcher Test",
        "message": "This is a test notification from Whale Watcher. If you see this, ntfy is working!",
        "priority": 3,
        "tags": ["test", "whale"]
    });
    
    let url = format!("{}/{}", config.base_url, config.topic);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()?;
    
    let mut request = client.post(&url).json(&test_payload);
    
    if let Some((user, pass)) = &config.auth {
        request = request.basic_auth(user, Some(pass));
    }
    
    let response = request.send().await?;
    
    if response.status().is_success() {
        println!("✅ Ntfy test successful!");
    } else {
        let error_text = response.text().await?;
        return Err(format!("Ntfy test failed: {} - {}", response.status(), error_text).into());
    }
    
    Ok(())
}
