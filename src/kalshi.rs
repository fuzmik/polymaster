use crate::config::Config;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KalshiError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trade {
    #[serde(rename = "trade_id")]
    pub trade_id: String,
    #[serde(rename = "ticker")]
    pub ticker: String,
    #[serde(rename = "price")]
    pub price: f64,
    #[serde(rename = "count")]
    pub count: i32,
    #[serde(rename = "yes_price")]
    pub yes_price: f64,
    #[serde(rename = "no_price")]
    pub no_price: f64,
    #[serde(rename = "taker_side")]
    pub taker_side: String,
    #[serde(rename = "created_time")]
    pub created_time: String,
    #[serde(skip)]
    pub market_title: Option<String>,
    // Note: Kalshi public API doesn't expose account IDs for privacy
    // Use trade_id as proxy for tracking patterns
}

#[derive(Debug, Deserialize)]
struct TradesResponse {
    #[serde(default)]
    trades: Vec<Trade>,
}

pub async fn fetch_recent_trades(config: Option<&Config>) -> Result<Vec<Trade>, KalshiError> {
    let client = reqwest::Client::new();

    // Kalshi's public trades endpoint
    let url = "https://api.elections.kalshi.com/trade-api/v2/markets/trades";

    let mut request = client
        .get(url)
        .query(&[("limit", "100")])
        .header("Accept", "application/json");

    // Add authentication if credentials are provided
    if let Some(cfg) = config {
        if let (Some(key_id), Some(_private_key)) =
            (&cfg.kalshi_api_key_id, &cfg.kalshi_private_key)
        {
            // For simplicity, we'll use basic auth
            // In production, you'd implement proper HMAC signature
            request = request.header("KALSHI-ACCESS-KEY", key_id);
        }
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        return Err(KalshiError::ParseError(format!(
            "API returned status: {}",
            response.status()
        )));
    }

    let text = response.text().await?;

    match serde_json::from_str::<TradesResponse>(&text) {
        Ok(response) => Ok(response.trades),
        Err(e) => {
            // If parsing fails, return empty list to allow tool to continue
            eprintln!("Warning: Failed to parse Kalshi response: {}", e);
            Ok(Vec::new())
        }
    }
}

#[derive(Debug, Deserialize)]
struct MarketResponse {
    market: MarketData,
}

#[derive(Debug, Deserialize)]
struct MarketData {
    title: Option<String>,
    subtitle: Option<String>,
}

pub async fn fetch_market_info(ticker: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.elections.kalshi.com/trade-api/v2/markets/{}",
        ticker
    );

    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            if let Ok(text) = response.text().await {
                if let Ok(market_response) = serde_json::from_str::<MarketResponse>(&text) {
                    return market_response
                        .market
                        .title
                        .or(market_response.market.subtitle);
                }
            }
        }
        _ => {}
    }

    None
}

// ============================================================================
// HELPER FUNCTIONS FOR SPORTS PARSING
// ============================================================================

fn parse_sports_game(ticker: &str, sport: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if parts.len() >= 3 {
        let teams_part = parts[parts.len() - 2];
        let outcome = parts.last().unwrap_or(&"");

        // Extract team codes (last 6 chars of teams_part)
        if teams_part.len() >= 6 {
            let team_codes = &teams_part[teams_part.len() - 6..];
            let away = &team_codes[..3];
            let home = &team_codes[3..];

            if side.to_uppercase() == "YES" {
                return format!("{} wins vs {} ({})", 
                    outcome.to_uppercase(),
                    if outcome.to_uppercase() == away.to_uppercase() {
                        home.to_uppercase()
                    } else {
                        away.to_uppercase()
                    },
                    sport
                );
            } else {
                let other_team = if outcome.to_uppercase() == away.to_uppercase() {
                    home.to_uppercase()
                } else {
                    away.to_uppercase()
                };
                return format!("{} wins vs {} ({})", 
                    other_team, outcome.to_uppercase(), sport);
            }
        }
    }
    format!("Team to win ({})", sport)
}

fn parse_individual_sport(ticker: &str, sport: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(participant) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins ({})", participant.to_uppercase(), sport);
        } else {
            return format!("{} loses ({})", participant.to_uppercase(), sport);
        }
    }
    format!("Match outcome ({})", sport)
}

fn parse_golf(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(player) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins tournament (Golf)", player.to_uppercase());
        } else {
            return format!("{} doesn't win (Golf)", player.to_uppercase());
        }
    }
    format!("Tournament winner (Golf)")
}

fn parse_olympics(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(athlete_or_country) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins gold (Olympics)", athlete_or_country.to_uppercase());
        } else {
            return format!("{} doesn't win gold (Olympics)", athlete_or_country.to_uppercase());
        }
    }
    format!("Gold medal (Olympics)")
}

fn parse_sports_total(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(threshold) = parts.last() {
        if threshold.chars().all(|c| c.is_numeric()) {
            // Extract sport from prefix
            let prefix = parts[0];
            let sport = if prefix.contains("NFL") {
                "NFL"
            } else if prefix.contains("NBA") {
                "NBA"
            } else if prefix.contains("NHL") {
                "NHL"
            } else if prefix.contains("MLB") {
                "MLB"
            } else if prefix.contains("NCAAF") || prefix.contains("CFB") {
                "College Football"
            } else if prefix.contains("NCAAB") || prefix.contains("CBB") {
                "College Basketball"
            } else if prefix.contains("TENNIS") {
                "Tennis"
            } else if prefix.contains("GOLF") {
                "Golf"
            } else if prefix.contains("SOCCER") || prefix.contains("FOOTBALL") {
                "Soccer"
            } else {
                "Game"
            };

            // Extract teams if possible
            if parts.len() >= 3 {
                if let Some(teams_part) = parts.get(parts.len() - 2) {
                    if teams_part.len() >= 6 {
                        let team_codes = &teams_part[teams_part.len() - 6..];
                        let away = &team_codes[..3];
                        let home = &team_codes[3..];
                        return format!(
                            "Total points {} {} | {} @ {} ({})",
                            if side.to_uppercase() == "YES" { "OVER" } else { "UNDER" },
                            threshold,
                            away.to_uppercase(),
                            home.to_uppercase(),
                            sport
                        );
                    }
                }
            }

            return format!("Total points {} {} ({})", 
                if side.to_uppercase() == "YES" { "OVER" } else { "UNDER" },
                threshold, sport);
        }
    }
    format!("Total points bet")
}

fn parse_sports_spread(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(last_part) = parts.last() {
        // Handle formats: "CAR3", "CAR-3", "CAR_N3" (negative), etc.
        let team = last_part
            .chars()
            .take_while(|c| c.is_alphabetic())
            .collect::<String>();
        let spread_str = last_part
            .chars()
            .skip_while(|c| c.is_alphabetic())
            .filter(|c| c.is_numeric() || *c == '.' || *c == '-')
            .collect::<String>();

        if !team.is_empty() && !spread_str.is_empty() {
            let spread_value = spread_str.trim_start_matches('-');
            if side.to_uppercase() == "YES" {
                return format!(
                    "{} wins by {} or more (covers)",
                    team.to_uppercase(),
                    spread_value
                );
            } else {
                return format!(
                    "{} loses or wins by less than {} (doesn't cover)",
                    team.to_uppercase(),
                    spread_value
                );
            }
        }
    }
    format!("Point spread bet")
}

// LEAGUE-SPECIFIC PARSING FUNCTIONS
fn parse_wnba(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(team) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins (WNBA)", team.to_uppercase());
        } else {
            return format!("{} loses (WNBA)", team.to_uppercase());
        }
    }
    format!("WNBA game")
}

fn parse_euroleague(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(team) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins (EuroLeague)", team.to_uppercase());
        } else {
            return format!("{} loses (EuroLeague)", team.to_uppercase());
        }
    }
    format!("EuroLeague game")
}

fn parse_womens_college_basketball(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(team) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins (Women's College Basketball)", team.to_uppercase());
        } else {
            return format!("{} loses (Women's College Basketball)", team.to_uppercase());
        }
    }
    format!("Women's College Basketball game")
}

fn parse_japanese_baseball(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(team) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins (Japanese Baseball)", team.to_uppercase());
        } else {
            return format!("{} loses (Japanese Baseball)", team.to_uppercase());
        }
    }
    format!("Japanese Baseball game")
}

fn parse_korean_baseball(ticker: &str, side: &str) -> String {
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(team) = parts.last() {
        if side.to_uppercase() == "YES" {
            return format!("{} wins (Korean Baseball)", team.to_uppercase());
        } else {
            return format!("{} loses (Korean Baseball)", team.to_uppercase());
        }
    }
    format!("Korean Baseball game")
}

// ============================================================================
// MAIN PARSING FUNCTION
// ============================================================================

pub fn parse_ticker_details(ticker: &str, side: &str) -> String {
    let betting_side = side.to_uppercase();
    
    // FIRST: Check for Kalshi prefix pattern to avoid false matches
    if !ticker.starts_with("KX") {
        // Fallback for non-Kalshi format tickers
        return format!("{} - check market details", betting_side);
    }
    
    let parts: Vec<&str> = ticker.split('-').collect();
    if parts.len() < 2 {
        return format!("{} - check market details", betting_side);
    }
    
    let prefix = parts[0]; // e.g., "KXNHLGAME" or "KXETHD"
    
    // DEBUG: Uncomment to see what tickers are being parsed
    // println!("DEBUG: Parsing ticker '{}' with prefix '{}'", ticker, prefix);
    
    // ========================================================================
    // 1. CRYPTOCURRENCY/STOCK PRICE THRESHOLDS
    // ========================================================================
    if prefix.contains("ETHD") || prefix.contains("BTCD") || prefix.contains("SOLD") || 
       prefix.contains("SPXD") || prefix.contains("TSLAD") {
        if let Some(threshold_part) = parts.last() {
            if threshold_part.starts_with('T') || threshold_part.starts_with('t') {
                let price = &threshold_part[1..];
                let asset = if prefix.contains("ETH") {
                    "Ethereum (ETH)"
                } else if prefix.contains("BTC") {
                    "Bitcoin (BTC)"
                } else if prefix.contains("SOL") {
                    "Solana (SOL)"
                } else if prefix.contains("SPX") {
                    "S&P 500"
                } else if prefix.contains("TSLA") {
                    "Tesla"
                } else {
                    "Asset"
                };

                return format!("{} price {} ${} at expiry", asset, 
                    if betting_side == "YES" { "≥" } else { "<" }, price);
            }
        }
    }
    
    // ========================================================================
    // 2. SPORTS DETECTION - Match complete sport codes with GAME suffix
    // ========================================================================
    
    // Major North American Sports
    if prefix.contains("NHLGAME") {
        return parse_sports_game(ticker, "NHL", side);
    } 
    else if prefix.contains("NFLGAME") {
        return parse_sports_game(ticker, "NFL", side);
    }
    else if prefix.contains("NBAGAME") {
        return parse_sports_game(ticker, "NBA", side);
    }
    else if prefix.contains("MLBGAME") {
        return parse_sports_game(ticker, "MLB", side);
    }
    else if prefix.contains("SOCCERGAME") || prefix.contains("FOOTBALLGAME") {
        return parse_sports_game(ticker, "Soccer", side);
    }
    
    // Expanded Basketball Leagues (with GAME suffix)
    else if prefix.contains("WNBAGAME") {
        return parse_wnba(ticker, side);
    }
    else if prefix.contains("EUROLEAGUEGAME") || prefix.contains("EUROCUPGAME") {
        return parse_euroleague(ticker, side);
    }
    else if (prefix.contains("NCAABGAME") || prefix.contains("CBBGAME")) && 
            (ticker.contains("W") || ticker.contains("WOMEN")) {
        return parse_womens_college_basketball(ticker, side);
    }
    
    // Expanded Baseball Leagues
    else if prefix.contains("NPBGAME") {
        return parse_japanese_baseball(ticker, side);
    }
    else if prefix.contains("KBOGAME") {
        return parse_korean_baseball(ticker, side);
    }
    
    // ========================================================================
    // 3. NEW SPORTS with proper prefix matching
    // ========================================================================
    else if prefix.contains("TENNIS") {
        return parse_individual_sport(ticker, "Tennis", side);
    }
    else if prefix.contains("GOLF") {
        return parse_golf(ticker, side);
    }
    else if prefix.contains("UFC") || prefix.contains("MMA") {
        return parse_individual_sport(ticker, "MMA", side);
    }
    else if prefix.contains("BOXING") {
        return parse_individual_sport(ticker, "Boxing", side);
    }
    else if prefix.contains("OLYMPICS") {
        return parse_olympics(ticker, side);
    }
    
    // ========================================================================
    // 4. SPORTS TOTALS (OVER/UNDER)
    // ========================================================================
    else if prefix.contains("TOTAL") {
        return parse_sports_total(ticker, side);
    }
    
    // ========================================================================
    // 5. SPORTS SPREADS
    // ========================================================================
    else if prefix.contains("SPREAD") {
        return parse_sports_spread(ticker, side);
    }
    
    // ========================================================================
    // 6. PLAYER PROPS (touchdowns, points, etc)
    // ========================================================================
    else if prefix.contains("TD") || prefix.contains("SCORE") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold) = parts.last() {
            if threshold.chars().all(|c| c.is_numeric()) {
                let prop_type = if ticker.contains("TD") {
                    "touchdowns"
                } else {
                    "points"
                };
                return format!(
                    "Player gets {} {} {}",
                    if betting_side == "YES" { "≥" } else { "<" },
                    threshold, prop_type
                );
            }
        }
    }
    
    // ========================================================================
    // 7. TEMPERATURE MARKETS
    // ========================================================================
    else if (prefix.contains("HIGH") || prefix.contains("LOW")) && ticker.contains("T") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold_part) = parts.last() {
            if let Some(temp) = threshold_part.strip_prefix('T') {
                let metric = if ticker.contains("HIGH") {
                    "High"
                } else {
                    "Low"
                };
                return format!(
                    "{} temp {} {}°F",
                    metric,
                    if betting_side == "YES" { "≥" } else { "<" },
                    temp
                );
            }
        }
    }
    
    // ========================================================================
    // 8. PRESIDENTIAL/ELECTION MARKETS
    // ========================================================================
    else if prefix.contains("PRES") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins", outcome.to_uppercase());
            } else {
                return format!("{} doesn't win", outcome.to_uppercase());
            }
        }
    }
    
    // ========================================================================
    // 9. COMBOS/PARLAYS
    // ========================================================================
    else if prefix.contains("COMBO") || prefix.contains("PARLAY") || prefix.contains("MULTI") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(last) = parts.last() {
            return format!(
                "{} {} combo/parlay",
                if betting_side == "YES" { "Wins" } else { "Loses" },
                last.to_uppercase()
            );
        }
    }
    
    // ========================================================================
    // 10. FIRST/LAST/ANYTIME SCORER
    // ========================================================================
    else if prefix.contains("FIRST") || prefix.contains("LAST") || prefix.contains("ANYTIME") {
        let timing = if ticker.contains("FIRST") {
            "first"
        } else if ticker.contains("LAST") {
            "last"
        } else {
            "anytime"
        };
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(player) = parts.last() {
            if betting_side == "YES" {
                return format!("{} scores {} TD", player.to_uppercase(), timing);
            } else {
                return format!("{} doesn't score {} TD", player.to_uppercase(), timing);
            }
        }
    }
    
    // ========================================================================
    // 11. RANKING/PLACEMENT MARKETS
    // ========================================================================
    else if prefix.contains("TOP") || prefix.contains("FINISH") || prefix.contains("PLACE") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            return format!(
                "{} {}",
                outcome.to_uppercase(),
                if betting_side == "YES" { "finishes in position" } else { "doesn't finish in position" }
            );
        }
    }
    
    // ========================================================================
    // 12. FALLBACK: Try to extract outcome from last part
    // ========================================================================
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(outcome) = parts.last() {
        if outcome.len() <= 10 && outcome.chars().all(|c| c.is_alphanumeric()) {
            if betting_side == "YES" {
                return format!("{} happens", outcome.to_uppercase());
            } else {
                return format!("{} doesn't happen", outcome.to_uppercase());
            }
        }
    }
    
    // ========================================================================
    // 13. ABSOLUTE FALLBACK
    // ========================================================================
    if betting_side == "YES" {
        format!("YES - check market details (Ticker: {})", ticker)
    } else {
        format!("NO - check market details (Ticker: {})", ticker)
    }
}
