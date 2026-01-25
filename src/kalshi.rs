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

pub fn parse_ticker_details(ticker: &str, side: &str) -> String {
    let betting_side = side.to_uppercase();
    // Parse Kalshi ticker to extract bet details
    // Format examples:
    // KXNHLGAME-26JAN08ANACAR-CAR = NHL game, Carolina wins
    // KXNCAAFTOTAL-26JAN08MIAMISS-51 = NCAA football total points over 51
    // KXHIGHNY-24DEC-T63 = NYC high temp threshold
    // KXETHD-26JAN0818-T3109.99 = ETH price threshold

    // Cryptocurrency/Stock price thresholds
    if ticker.contains("ETH")
        || ticker.contains("BTC")
        || ticker.contains("SOL")
        || ticker.contains("SPX")
        || ticker.contains("TSLA")
    {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold_part) = parts.last() {
            if threshold_part.starts_with('T') || threshold_part.starts_with('t') {
                let price = &threshold_part[1..];
                let asset = if ticker.contains("ETH") {
                    "Ethereum (ETH)"
                } else if ticker.contains("BTC") {
                    "Bitcoin (BTC)"
                } else if ticker.contains("SOL") {
                    "Solana (SOL)"
                } else if ticker.contains("SPX") {
                    "S&P 500"
                } else if ticker.contains("TSLA") {
                    "Tesla"
                } else {
                    "Asset"
                };

                return format!("{} price {} ${} at expiry", asset, 
                    if betting_side == "YES" { "≥" } else { "<" }, price);
            }
        }
    }

    // Check for sports totals (over/under)
    if ticker.contains("TOTAL") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold) = parts.last() {
            if threshold.chars().all(|c| c.is_numeric()) {
                // Determine sport with expanded league support
                let sport = if ticker.contains("NFL") {
                    "NFL"
                } else if ticker.contains("NBA") {
                    "NBA"
                } else if ticker.contains("WNBA") {
                    "WNBA"
                } else if ticker.contains("NCAAB") || ticker.contains("CBB") {
                    "College Basketball"
                } else if ticker.contains("EUROLEAGUE") || ticker.contains("EUROCUP") {
                    "EuroLeague"
                } else if ticker.contains("MLB") {
                    "MLB"
                } else if ticker.contains("NPB") {
                    "Japanese Baseball"
                } else if ticker.contains("KBO") {
                    "Korean Baseball"
                } else if ticker.contains("NHL") {
                    "NHL"
                } else if ticker.contains("TENNIS") {
                    "Tennis"
                } else if ticker.contains("GOLF") {
                    "Golf"
                } else if ticker.contains("UFC") || ticker.contains("MMA") {
                    "MMA"
                } else if ticker.contains("BOXING") {
                    "Boxing"
                } else if ticker.contains("OLYMPICS") {
                    "Olympics"
                } else if ticker.contains("SOCCER") || ticker.contains("FOOTBALL") {
                    "Soccer"
                } else if ticker.contains("NCAAF") || ticker.contains("CFB") {
                    "College Football"
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
                                if betting_side == "YES" { "OVER" } else { "UNDER" },
                                threshold,
                                away.to_uppercase(),
                                home.to_uppercase(),
                                sport
                            );
                        }
                    }
                }

                return format!("Total points {} {} ({})", 
                    if betting_side == "YES" { "OVER" } else { "UNDER" },
                    threshold, sport);
            }
        }
    }

    // Expanded game detection for all sports leagues
    if ticker.contains("NHLGAME")
        || ticker.contains("NFLGAME")
        // ALL BASKETBALL LEAGUES
        || ticker.contains("NBAGAME")
        || ticker.contains("WNBAGAME")
        || ticker.contains("NCAABGAME") 
        || ticker.contains("CBBGAME")
        || ticker.contains("EUROLEAGUEGAME")
        || ticker.contains("EUROCUPGAME")
        || ticker.contains("FIBAGAME")
        || ticker.contains("CBAGAME")
        || ticker.contains("KBLGAME")
        || ticker.contains("NBLGAME")
        || ticker.contains("LNGGAME")
        // ALL BASEBALL LEAGUES
        || ticker.contains("MLBGAME")
        || ticker.contains("NPBGAME")
        || ticker.contains("KBOGAME")
        || ticker.contains("BASEBALLGAME")
        // OTHER SPORTS
        || ticker.contains("SOCCERGAME")
        || ticker.contains("FOOTBALLGAME")
        // NEW SPORTS
        || ticker.contains("TENNIS")
        || ticker.contains("GOLFMATCH") || ticker.contains("GOLFTOUR")
        || ticker.contains("UFC") || ticker.contains("MMA")
        || ticker.contains("BOXING")
        || ticker.contains("OLYMPICS")
    {
        // Sports game format
        let parts: Vec<&str> = ticker.split('-').collect();
        if parts.len() >= 3 {
            let outcome = parts.last().unwrap_or(&"");

            // Extract team/player codes from middle part
            if let Some(teams_part) = parts.get(parts.len() - 2) {
                // Format like "26JAN08ANACAR" - extract last 6 chars for teams
                if teams_part.len() >= 6 {
                    let team_codes = &teams_part[teams_part.len() - 6..];
                    let away = &team_codes[..3];
                    let home = &team_codes[3..];

                    // Determine sport with expanded league support
                    let sport = if ticker.contains("NHL") {
                        "NHL"
                    } else if ticker.contains("NFL") {
                        "NFL"
                    } 
                    // BASKETBALL LEAGUES - EXPANDED
                    else if ticker.contains("NBA") {
                        "NBA"
                    } else if ticker.contains("NCAAB") || ticker.contains("CBB") {
                        "College Basketball"
                    } else if ticker.contains("WNBA") {
                        "WNBA"
                    } else if ticker.contains("EUROLEAGUE") || ticker.contains("EUROCUP") {
                        "EuroLeague Basketball"
                    } else if ticker.contains("FIBA") {
                        "FIBA Basketball"
                    } else if ticker.contains("CBA") && ticker.contains("BASKET") {
                        "Chinese Basketball Association"
                    } else if ticker.contains("KBL") {
                        "Korean Basketball League"
                    } else if ticker.contains("NBL") && (ticker.contains("AUS") || ticker.contains("BASKET")) {
                        "Australian NBL"
                    } else if ticker.contains("LNB") {
                        "French Basketball (LNB)"
                    } else if ticker.contains("BASKET") {
                        "Basketball"
                    }
                    // BASEBALL LEAGUES - EXPANDED
                    else if ticker.contains("MLB") {
                        "MLB"
                    } else if ticker.contains("NPB") {
                        "Japanese Baseball (NPB)"
                    } else if ticker.contains("KBO") {
                        "Korean Baseball (KBO)"
                    } else if ticker.contains("BASEBALL") {
                        "Baseball"
                    }
                    // NEW SPORTS
                    else if ticker.contains("TENNIS") {
                        "Tennis"
                    } else if ticker.contains("GOLF") {
                        "Golf"
                    } else if ticker.contains("UFC") || ticker.contains("MMA") {
                        "MMA"
                    } else if ticker.contains("BOXING") {
                        "Boxing"
                    } else if ticker.contains("OLYMPICS") {
                        "Olympics"
                    } else if ticker.contains("SOCCER") || ticker.contains("FOOTBALL") {
                        "Soccer"
                    } else if ticker.contains("NCAAF") || ticker.contains("CFB") {
                        "College Football"
                    } else {
                        "Game"
                    };

                    // Show what they're actually betting will happen
                    if betting_side == "YES" {
                        return format!(
                            "{} wins vs {} ({})",
                            outcome.to_uppercase(),
                            if outcome.to_uppercase() == away.to_uppercase() {
                                home.to_uppercase()
                            } else {
                                away.to_uppercase()
                            },
                            sport
                        );
                    } else {
                        // Betting NO means betting the OTHER team/player wins
                        let other_team = if outcome.to_uppercase() == away.to_uppercase() {
                            home.to_uppercase()
                        } else {
                            away.to_uppercase()
                        };
                        return format!(
                            "{} wins vs {} ({})",
                            other_team,
                            outcome.to_uppercase(),
                            sport
                        );
                    }
                }
            }
        }
    }

    // LEAGUE-SPECIFIC DETECTION
    // WNBA Detection
    if ticker.contains("WNBA") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(team) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins (WNBA)", team.to_uppercase());
            } else {
                return format!("{} loses (WNBA)", team.to_uppercase());
            }
        }
    }

    // EuroLeague Basketball Detection
    if ticker.contains("EUROLEAGUE") || ticker.contains("EUROCUP") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(team) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins (EuroLeague)", team.to_uppercase());
            } else {
                return format!("{} loses (EuroLeague)", team.to_uppercase());
            }
        }
    }

    // College Basketball (Women's) Detection
    if ticker.contains("NCAAB") && (ticker.contains("W") || ticker.contains("WOMEN")) {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(team) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins (Women's College Basketball)", team.to_uppercase());
            } else {
                return format!("{} loses (Women's College Basketball)", team.to_uppercase());
            }
        }
    }

    // International Baseball Leagues
    if ticker.contains("NPB") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(team) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins (Japanese Baseball)", team.to_uppercase());
            } else {
                return format!("{} loses (Japanese Baseball)", team.to_uppercase());
            }
        }
    }

    if ticker.contains("KBO") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(team) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins (Korean Baseball)", team.to_uppercase());
            } else {
                return format!("{} loses (Korean Baseball)", team.to_uppercase());
            }
        }
    }

    // NEW SPORTS SPECIAL HANDLING
    
    // Tennis specific parsing
    if ticker.contains("TENNIS") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(player) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins (Tennis)", player.to_uppercase());
            } else {
                return format!("{} loses (Tennis)", player.to_uppercase());
            }
        }
    }

    // Golf specific parsing  
    if ticker.contains("GOLF") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(player) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins tournament (Golf)", player.to_uppercase());
            } else {
                return format!("{} doesn't win (Golf)", player.to_uppercase());
            }
        }
    }

    // MMA/UFC specific parsing
    if ticker.contains("UFC") || ticker.contains("MMA") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(fighter) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins fight (MMA)", fighter.to_uppercase());
            } else {
                return format!("{} loses fight (MMA)", fighter.to_uppercase());
            }
        }
    }

    // Boxing specific parsing
    if ticker.contains("BOXING") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(fighter) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins fight (Boxing)", fighter.to_uppercase());
            } else {
                return format!("{} loses fight (Boxing)", fighter.to_uppercase());
            }
        }
    }

    // Olympics specific parsing
    if ticker.contains("OLYMPICS") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(athlete_or_country) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins gold (Olympics)", athlete_or_country.to_uppercase());
            } else {
                return format!("{} doesn't win gold (Olympics)", athlete_or_country.to_uppercase());
            }
        }
    }

    // Check for point spreads
    if ticker.contains("SPREAD") {
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
                if betting_side == "YES" {
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
    }

    // Check for player props (touchdowns, points, etc)
    if ticker.contains("TD") || ticker.contains("SCORE") {
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
    } else if ticker.contains("HIGH") || ticker.contains("LOW") {
        // Temperature markets
        if ticker.contains("T") {
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
    } else if ticker.contains("PRES") {
        // Presidential/election markets
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            if betting_side == "YES" {
                return format!("{} wins", outcome.to_uppercase());
            } else {
                return format!("{} doesn't win", outcome.to_uppercase());
            }
        }
    }

    // Check for combos/parlays
    if ticker.contains("COMBO") || ticker.contains("PARLAY") || ticker.contains("MULTI") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(last) = parts.last() {
            return format!(
                "{} {} combo/parlay",
                if betting_side == "YES" { "Wins" } else { "Loses" },
                last.to_uppercase()
            );
        }
    }

    // Check for first/last to score
    if ticker.contains("FIRST") || ticker.contains("LAST") || ticker.contains("ANYTIME") {
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

    // Check for ranking/placement markets (TOP, FINISH, PLACE)
    if ticker.contains("TOP") || ticker.contains("FINISH") || ticker.contains("PLACE") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            return format!(
                "{} {}",
                outcome.to_uppercase(),
                if betting_side == "YES" { "finishes in position" } else { "doesn't finish in position" }
            );
        }
    }

    // Default: try to extract outcome from last part
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

    // Absolute fallback - show more context
    if betting_side == "YES" {
        String::from("YES - check market details")
    } else {
        String::from("NO - check market details")
    }
}
