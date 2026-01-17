// kalshi.rs
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

// Helper function to get team emoji based on abbreviation and sport context
fn get_team_emoji<'a>(team_code: &str, sport_hint: Option<&str>) -> &'a str {
    let code_upper = team_code.to_uppercase();
    let sport = sport_hint.unwrap_or("").to_lowercase();
    
    // Check sport-specific mappings first
    match sport.as_str() {
        "nfl" | "american football" | "football" => match code_upper.as_str() {
            // NFL Teams
            "BUF" | "BUFFALO" => "🏈🦬",
            "MIA" | "MIAMI" => "🏈🐬",
            "NE" | "NWE" | "NEWENGLAND" | "NEW ENGLAND" => "🏈🇺🇸",
            "NYJ" | "JETS" => "🏈✈️",
            "BAL" | "RAVENS" => "🏈🐦‍⬛",
            "CIN" | "BENGALS" => "🏈🐅",
            "CLE" | "BROWNS" => "🏈🐕",
            "PIT" | "STEELERS" => "🏈⚫🟡",
            "HOU" | "TEXANS" => "🏈🤠",
            "IND" | "COLTS" => "🏈🐎",
            "JAX" | "JAGUARS" => "🏈🐆",
            "TEN" | "TITANS" => "🏈🔱",
            "DEN" | "BRONCOS" => "🏈🐴",
            "KC" | "CHIEFS" => "🏈🏹",
            "LV" | "RAIDERS" => "🏈🏴‍☠️",
            "LAC" | "CHARGERS" => "🏈⚡",
            "DAL" | "COWBOYS" => "🏈⭐",
            "NYG" | "GIANTS" => "🏈👨‍👦",
            "PHI" | "EAGLES" => "🏈🦅",
            "WAS" | "COMMANDERS" => "🏈👑",
            "CHI" | "BEARS" => "🏈🐻",
            "DET" | "LIONS" => "🏈🦁",
            "GB" | "PACKERS" => "🏈🧀",
            "MIN" | "VIKINGS" => "🏈⛵",
            "ATL" | "FALCONS" => "🏈🦅",
            "CAR" | "PANTHERS" => "🏈🐆",
            "NO" | "SAINTS" => "🏈⛪",
            "TB" | "BUCCANEERS" => "🏈🏴‍☠️",
            "ARI" | "CARDINALS" => "🏈🐦",
            "LAR" | "RAMS" => "🏈🐏",
            "SF" | "49ERS" => "🏈⛏️",
            "SEA" | "SEAHAWKS" => "🏈🦅",
            _ => "🏈",
        },
        "nba" | "basketball" => match code_upper.as_str() {
            // NBA Teams
            "ATL" | "HAWKS" => "🏀🦅",
            "BOS" | "CELTICS" => "🏀☘️",
            "BKN" | "NETS" => "🏀🌉",
            "CHA" | "HORNETS" => "🏀🐝",
            "CHI" | "BULLS" => "🏀🐂",
            "CLE" | "CAVS" | "CAVALIERS" => "🏀⚔️",
            "DAL" | "MAVS" | "MAVERICKS" => "🏀🐴",
            "DEN" | "NUGGETS" => "🏀⛏️",
            "DET" | "PISTONS" => "🏀🔩",
            "GSW" | "WARRIORS" => "🏀🌉",
            "HOU" | "ROCKETS" => "🏀🚀",
            "IND" | "PACERS" => "🏀🏎️",
            "LAC" | "CLIPPERS" => "🏀⚓",
            "LAL" | "LAKERS" => "🏀💜💛",
            "MEM" | "GRIZZLIES" => "🏀🐻",
            "MIA" | "HEAT" => "🏀🔥",
            "MIL" | "BUCKS" => "🏀🦌",
            "MIN" | "WOLVES" | "TIMBERWOLVES" => "🏀🐺",
            "NOP" | "PELICANS" => "🏀🐦",
            "NYK" | "KNICKS" => "🏀🗽",
            "OKC" | "THUNDER" => "🏀⚡",
            "ORL" | "MAGIC" => "🏀🪄",
            "PHI" | "76ERS" => "🏀⭐",
            "PHX" | "SUNS" => "🏀☀️",
            "POR" | "BLAZERS" => "🏀🌲",
            "SAC" | "KINGS" => "🏀👑",
            "SAS" | "SPURS" => "🏀🌵",
            "TOR" | "RAPTORS" => "🏀🦖",
            "UTA" | "JAZZ" => "🏀🎷",
            "WAS" | "WIZARDS" => "🏀🧙‍♂️",
            _ => "🏀",
        },
        "nhl" | "hockey" => match code_upper.as_str() {
            // NHL Teams
            "ANA" | "DUCKS" => "🏒🦆",
            "ARI" | "YOTES" | "COYOTES" => "🏒🐺",
            "BOS" | "BRUINS" => "🏒🐻",
            "BUF" | "SABRES" => "🏒⚔️",
            "CGY" | "FLAMES" => "🏒🔥",
            "CAR" | "CANES" | "HURRICANES" => "🏒🌀",
            "CHI" | "HAWKS" | "BLACKHAWKS" => "🏒🦅",
            "COL" | "AVS" | "AVALANCHE" => "🏒🏔️",
            "CBJ" | "JACKETS" | "BLUEJACKETS" => "🏒⚓",
            "DAL" | "STARS" => "🏒⭐",
            "DET" | "WINGS" | "REDWINGS" => "🏒✈️",
            "EDM" | "OILERS" => "🏒🛢️",
            "FLA" | "PANTHERS" => "🏒🐆",
            "LAK" | "KINGS" => "🏒👑",
            "MIN" | "WILD" => "🏒🌲",
            "MTL" | "CANADIENS" => "🏒🍁",
            "NSH" | "PREDS" | "PREDATORS" => "🏒🐅",
            "NJD" | "DEVILS" => "🏒😈",
            "NYI" | "ISLANDERS" => "🏒🏝️",
            "NYR" | "RANGERS" => "🏒🗽",
            "OTT" | "SENATORS" => "🏒⚖️",
            "PHI" | "FLYERS" => "🏒✈️",
            "PIT" | "PENS" | "PENGUINS" => "🏒🐧",
            "SJS" | "SHARKS" => "🏒🦈",
            "SEA" | "KRAKEN" => "🏒🐙",
            "STL" | "BLUES" => "🏒🎵",
            "TBL" | "LIGHTNING" => "🏒⚡",
            "TOR" | "LEAFS" | "MAPLELEAFS" => "🏒🍁",
            "VAN" | "CANUCKS" => "🏒🐋",
            "VGK" | "KNIGHTS" | "GOLDENKNIGHTS" => "🏒♟️",
            "WSH" | "CAPS" | "CAPITALS" => "🏒🏛️",
            "WPG" | "JETS" => "🏒✈️",
            _ => "🏒",
        },
        "mlb" | "baseball" => match code_upper.as_str() {
            // MLB Teams
            "ARI" | "DBACKS" | "DIAMONDBACKS" => "⚾🐍",
            "ATL" | "BRAVES" => "⚾🪓",
            "BAL" | "ORIOLES" => "⚾🐦",
            "BOS" | "REDSOX" => "⚾🟥🧦",
            "CHC" | "CUBS" => "⚾🐻",
            "CHW" | "WHITESOX" => "⚾🟥⚾",
            "CIN" | "REDS" => "⚾🔴",
            "CLE" | "GUARDIANS" => "⚾👁️",
            "COL" | "ROCKIES" => "⚾🏔️",
            "DET" | "TIGERS" => "⚾🐅",
            "HOU" | "ASTROS" => "⚾🧡",
            "KC" | "ROYALS" => "⚾👑",
            "LAA" | "ANGELS" => "⚾👼",
            "LAD" | "DODGERS" => "⚾🔵",
            "MIA" | "MARLINS" => "⚾🐟",
            "MIL" | "BREWERS" => "⚾🍺",
            "MIN" | "TWINS" => "⚾👥",
            "NYM" | "METS" => "⚾🌎",
            "NYY" | "YANKEES" => "⚾🗽",
            "OAK" | "ATHLETICS" => "⚾🐘",
            "PHI" | "PHILLIES" => "⚾🔔",
            "PIT" | "PIRATES" => "⚾🏴‍☠️",
            "SD" | "PADRES" => "⚾🧔",
            "SEA" | "MARINERS" => "⚾⚓",
            "SF" | "GIANTS" => "⚾🌉",
            "STL" | "CARDINALS" => "⚾🐦",
            "TB" | "RAYS" => "⚾🌞",
            "TEX" | "RANGERS" => "⚾🤠",
            "TOR" | "BLUEJAYS" => "⚾🐦",
            "WSH" | "NATIONALS" => "⚾🇺🇸",
            _ => "⚾",
        },
        "soccer" => match code_upper.as_str() {
            // Soccer/Football Teams
            "MCI" | "MANCITY" => "⚽🔵",
            "LIV" | "LIVERPOOL" => "⚽🔴",
            "MUN" | "MANUTD" => "⚽👹",
            "ARS" | "ARSENAL" => "⚽🔴⚪",
            "CHE" | "CHELSEA" => "⚽🔵",
            "TOT" | "TOTTENHAM" => "⚽⚪🔵",
            "RM" | "REALMADRID" => "⚽👑",
            "BAR" | "BARCELONA" => "⚽🔵🔴",
            "BAY" | "BAYERN" => "⚽🔴",
            "PSG" => "⚽🔵🔴",
            "JUV" | "JUVENTUS" => "⚽⚫⚪",
            "ACM" | "ACMILAN" => "⚽🔴⚫",
            "INT" | "INTER" => "⚽🔵⚫",
            _ => "⚽",
        },
        "college" | "ncaa" | "cfb" | "cbb" => match code_upper.as_str() {
            // College Sports
            "ALA" | "ALABAMA" => "🐘🎓",
            "CLEM" | "CLEMSON" => "🐅🎓",
            "UGA" | "GEORGIA" => "🐕🎓",
            "LSU" => "🐅🎓",
            "MICH" | "MICHIGAN" => "〽️🎓",
            "OSU" | "OHIOSTATE" => "🅾️🎓",
            "OKLA" | "OKLAHOMA" => "⭕🎓",
            "ORE" | "OREGON" => "🦆🎓",
            "TEXAS" => "🤘🎓",
            "USC" => "✌️🎓",
            _ => "🎓",
        },
        _ => {
            // Generic mappings (when sport isn't specified or doesn't match above)
            match code_upper.as_str() {
                // Crypto/Financial
                "BTC" | "BITCOIN" => "₿",
                "ETH" | "ETHEREUM" => "Ξ",
                "SOL" | "SOLANA" => "🔆",
                "SPX" | "SP500" => "📈🇺🇸",
                "TSLA" => "🚗",
                "AAPL" => "🍎",
                "GOOGL" | "GOOG" => "🔍",
                "META" => "📱",
                "AMZN" => "📦",
                "MSFT" => "🪟",
                "NVDA" => "🎮",
                "BRK" => "🧓",
                
                // Politics
                "DEM" | "DEMOCRAT" => "🐴",
                "GOP" | "REPUBLICAN" => "🐘",
                "BIDEN" => "👴🇺🇸",
                "TRUMP" => "🦅🇺🇸",
                "HARRIS" => "👩🏾‍💼🇺🇸",
                "DESANTIS" => "🦩",
                "HALEY" => "👩🏼‍💼🇺🇸",
                
                // Default fallback
                _ => "🏆",
            }
        }
    }
}

// Helper function to get league/sport emoji
fn get_sport_emoji(sport: &str) -> &'static str {
    match sport.to_lowercase().as_str() {
        "nfl" | "american football" | "football" => "🏈",
        "nba" | "basketball" => "🏀",
        "nhl" | "hockey" => "🏒",
        "mlb" | "baseball" => "⚾",
        "soccer" => "⚽",
        "cfb" | "ncaaf" | "college football" => "🎓🏈",
        "cbb" | "ncaab" | "college basketball" => "🎓🏀",
        "golf" => "⛳",
        "tennis" => "🎾",
        "mma" | "ufc" => "🥊",
        "boxing" => "🥊",
        "racing" | "f1" => "🏎️",
        "olympics" => "🏅",
        "esports" | "gaming" => "🎮",
        "crypto" | "cryptocurrency" => "₿",
        "stocks" | "stock market" => "📈",
        "politics" | "election" => "🗳️",
        "weather" | "temperature" => "🌡️",
        "entertainment" => "🎭",
        "economics" => "💹",
        "technology" => "💻",
        "science" => "🔬",
        "health" => "🏥",
        "food" => "🍔",
        "travel" => "✈️",
        "music" => "🎵",
        "movies" => "🎬",
        _ => "🎯",
    }
}

// Helper function to get side emoji
fn get_side_emoji(side: &str) -> &'static str {
    match side.to_uppercase().as_str() {
        "YES" | "BUY" => "🟢📈",
        "NO" | "SELL" => "🔴📉",
        "BID" => "⬆️",
        "ASK" => "⬇️",
        _ => "➡️",
    }
}

pub fn parse_ticker_details(ticker: &str, side: &str) -> String {
    let betting_side = side.to_uppercase();
    let side_emoji = get_side_emoji(&betting_side);
    
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
        || ticker.contains("AAPL")
        || ticker.contains("GOOGL")
        || ticker.contains("META")
        || ticker.contains("AMZN")
        || ticker.contains("MSFT")
        || ticker.contains("NVDA")
        || ticker.contains("BRK")
    {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold_part) = parts.last() {
            if threshold_part.starts_with('T') || threshold_part.starts_with('t') {
                let price = &threshold_part[1..];
                let (asset, asset_emoji) = if ticker.contains("ETH") {
                    ("Ethereum (ETH)", "Ξ")
                } else if ticker.contains("BTC") {
                    ("Bitcoin (BTC)", "₿")
                } else if ticker.contains("SOL") {
                    ("Solana (SOL)", "🔆")
                } else if ticker.contains("SPX") {
                    ("S&P 500", "📈🇺🇸")
                } else if ticker.contains("TSLA") {
                    ("Tesla", "🚗")
                } else if ticker.contains("AAPL") {
                    ("Apple", "🍎")
                } else if ticker.contains("GOOGL") || ticker.contains("GOOG") {
                    ("Google", "🔍")
                } else if ticker.contains("META") {
                    ("Meta", "📱")
                } else if ticker.contains("AMZN") {
                    ("Amazon", "📦")
                } else if ticker.contains("MSFT") {
                    ("Microsoft", "🪟")
                } else if ticker.contains("NVDA") {
                    ("NVIDIA", "🎮")
                } else if ticker.contains("BRK") {
                    ("Berkshire Hathaway", "🧓")
                } else {
                    ("Asset", "💹")
                };

                return format!(
                    "{} {} {} {} at expiry {}",
                    asset_emoji,
                    side_emoji,
                    asset,
                    if betting_side == "YES" { "≥ $" } else { "< $" },
                    price
                );
            }
        }
    }

    // Check for sports totals (over/under)
    if ticker.contains("TOTAL") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold) = parts.last() {
            if threshold.chars().all(|c| c.is_numeric()) {
                let (sport, sport_emoji) = if ticker.contains("NFL") {
                    ("NFL", "🏈")
                } else if ticker.contains("NBA") {
                    ("NBA", "🏀")
                } else if ticker.contains("NHL") {
                    ("NHL", "🏒")
                } else if ticker.contains("MLB") {
                    ("MLB", "⚾")
                } else if ticker.contains("NCAAF") || ticker.contains("CFB") {
                    ("College Football", "🎓🏈")
                } else if ticker.contains("NCAAB") || ticker.contains("CBB") {
                    ("College Basketball", "🎓🏀")
                } else if ticker.contains("SOCCER") {
                    ("Soccer", "⚽")
                } else {
                    ("Game", "🎯")
                };

                // Extract teams if possible
                if parts.len() >= 3 {
                    if let Some(teams_part) = parts.get(parts.len() - 2) {
                        if teams_part.len() >= 6 {
                            let team_codes = &teams_part[teams_part.len() - 6..];
                            let away = &team_codes[..3];
                            let home = &team_codes[3..];
                            let away_emoji = get_team_emoji(away, Some(&sport.to_lowercase()));
                            let home_emoji = get_team_emoji(home, Some(&sport.to_lowercase()));
                            
                            return format!(
                                "{} Total {} {} {} | {} {} @ {} {} ({})",
                                sport_emoji,
                                side_emoji,
                                if betting_side == "YES" { "OVER" } else { "UNDER" },
                                threshold,
                                away_emoji,
                                away.to_uppercase(),
                                home_emoji,
                                home.to_uppercase(),
                                sport
                            );
                        }
                    }
                }

                return format!(
                    "{} Total {} {} {} ({})",
                    sport_emoji,
                    side_emoji,
                    if betting_side == "YES" { "OVER" } else { "UNDER" },
                    threshold,
                    sport
                );
            }
        }
    }

    if ticker.contains("NHLGAME")
        || ticker.contains("NFLGAME")
        || ticker.contains("NBAGAME")
        || ticker.contains("MLBGAME")
        || ticker.contains("SOCCERGAME")
    {
        // Sports game format
        let parts: Vec<&str> = ticker.split('-').collect();
        if parts.len() >= 3 {
            let outcome = parts.last().unwrap_or(&"");

            // Extract team codes from middle part
            if let Some(teams_part) = parts.get(parts.len() - 2) {
                // Format like "26JAN08ANACAR" - extract last 6 chars for teams
                if teams_part.len() >= 6 {
                    let team_codes = &teams_part[teams_part.len() - 6..];
                    let away = &team_codes[..3];
                    let home = &team_codes[3..];

                    let (sport, sport_emoji) = if ticker.contains("NHL") {
                        ("NHL", "🏒")
                    } else if ticker.contains("NFL") {
                        ("NFL", "🏈")
                    } else if ticker.contains("NBA") {
                        ("NBA", "🏀")
                    } else if ticker.contains("MLB") {
                        ("MLB", "⚾")
                    } else if ticker.contains("SOCCER") {
                        ("Soccer", "⚽")
                    } else {
                        ("Sports", "🎯")
                    };

                    // Show what they're actually betting will happen
                    if betting_side == "YES" {
                        let outcome_emoji = get_team_emoji(outcome, Some(&sport.to_lowercase()));
                        let opponent = if outcome.to_uppercase() == away.to_uppercase() {
                            home.to_uppercase()
                        } else {
                            away.to_uppercase()
                        };
                        let opponent_emoji = if outcome.to_uppercase() == away.to_uppercase() {
                            get_team_emoji(home, Some(&sport.to_lowercase()))
                        } else {
                            get_team_emoji(away, Some(&sport.to_lowercase()))
                        };
                        
                        return format!(
                            "{} {} {} {} wins vs {} {} ({})",
                            sport_emoji,
                            side_emoji,
                            outcome_emoji,
                            outcome.to_uppercase(),
                            opponent_emoji,
                            opponent,
                            sport
                        );
                    } else {
                        // Betting NO means betting the OTHER team wins
                        let other_team = if outcome.to_uppercase() == away.to_uppercase() {
                            home.to_uppercase()
                        } else {
                            away.to_uppercase()
                        };
                        let other_team_emoji = if outcome.to_uppercase() == away.to_uppercase() {
                            get_team_emoji(home, Some(&sport.to_lowercase()))
                        } else {
                            get_team_emoji(away, Some(&sport.to_lowercase()))
                        };
                        let outcome_emoji = get_team_emoji(outcome, Some(&sport.to_lowercase()));
                        
                        return format!(
                            "{} {} {} {} wins vs {} {} ({})",
                            sport_emoji,
                            side_emoji,
                            other_team_emoji,
                            other_team,
                            outcome_emoji,
                            outcome.to_uppercase(),
                            sport
                        );
                    }
                }
            }
        }
    // Check for point spreads
    } else if ticker.contains("SPREAD") {
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
                let sport = if ticker.contains("NFL") { "nfl" }
                    else if ticker.contains("NBA") { "nba" }
                    else if ticker.contains("NHL") { "nhl" }
                    else if ticker.contains("MLB") { "mlb" }
                    else if ticker.contains("NCAAF") || ticker.contains("CFB") { "college football" }
                    else { "sports" };
                
                let team_emoji = get_team_emoji(&team, Some(sport));
                let sport_emoji = get_sport_emoji(sport);
                let spread_value = spread_str.trim_start_matches('-');
                
                if betting_side == "YES" {
                    return format!(
                        "{} {} {} {} wins by {} or more (covers spread)",
                        sport_emoji,
                        side_emoji,
                        team_emoji,
                        team.to_uppercase(),
                        spread_value
                    );
                } else {
                    return format!(
                        "{} {} {} {} loses or wins by less than {} (doesn't cover spread)",
                        sport_emoji,
                        side_emoji,
                        team_emoji,
                        team.to_uppercase(),
                        spread_value
                    );
                }
            }
        }
    // Check for player props (touchdowns, points, etc)
    } else if ticker.contains("TD") || ticker.contains("SCORE") || ticker.contains("POINTS") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(threshold) = parts.last() {
            if threshold.chars().all(|c| c.is_numeric()) {
                let prop_type = if ticker.contains("TD") {
                    "touchdowns 🏈"
                } else if ticker.contains("POINTS") {
                    "points 🏀"
                } else {
                    "goals/scores"
                };
                let sport_emoji = get_sport_emoji(
                    if ticker.contains("NFL") { "nfl" }
                    else if ticker.contains("NBA") { "nba" }
                    else if ticker.contains("NHL") { "nhl" }
                    else if ticker.contains("SOCCER") { "soccer" }
                    else { "sports" }
                );
                
                return format!(
                    "{} {} Player gets {} {} {}",
                    sport_emoji,
                    side_emoji,
                    if betting_side == "YES" { "≥" } else { "<" },
                    threshold,
                    prop_type
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
                        "High 🌡️"
                    } else {
                        "Low 🌡️"
                    };
                    let location_emoji = if ticker.contains("NY") { "🗽" }
                        else if ticker.contains("LA") || ticker.contains("CAL") { "🌴" }
                        else if ticker.contains("CHI") { "🌬️" }
                        else if ticker.contains("MIA") { "☀️" }
                        else if ticker.contains("SEA") { "☔" }
                        else { "📍" };
                    
                    return format!(
                        "{} {} {} temp {} {}°F",
                        location_emoji,
                        side_emoji,
                        metric,
                        if betting_side == "YES" { "≥" } else { "<" },
                        temp
                    );
                }
            }
        }
    } else if ticker.contains("PRES") || ticker.contains("SENATE") || ticker.contains("HOUSE") {
        // Presidential/election markets
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            let outcome_emoji = get_team_emoji(outcome, Some("politics"));
            if betting_side == "YES" {
                return format!("🗳️ {} {} {} wins election", side_emoji, outcome_emoji, outcome.to_uppercase());
            } else {
                return format!("🗳️ {} {} {} doesn't win election", side_emoji, outcome_emoji, outcome.to_uppercase());
            }
        }
    }

    // Check for combos/parlays
    if ticker.contains("COMBO") || ticker.contains("PARLAY") || ticker.contains("MULTI") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(last) = parts.last() {
            return format!(
                "🎰 {} {} {} combo/parlay",
                side_emoji,
                if betting_side == "YES" { "Wins" } else { "Loses" },
                last.to_uppercase()
            );
        }
    }

    // Check for first/last to score
    if ticker.contains("FIRST") || ticker.contains("LAST") || ticker.contains("ANYTIME") {
        let timing = if ticker.contains("FIRST") {
            "first 🥇"
        } else if ticker.contains("LAST") {
            "last 🏁"
        } else {
            "anytime ⏱️"
        };
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(player) = parts.last() {
            if betting_side == "YES" {
                return format!("{} {} {} scores {} TD", get_sport_emoji("nfl"), side_emoji, player.to_uppercase(), timing);
            } else {
                return format!("{} {} {} doesn't score {} TD", get_sport_emoji("nfl"), side_emoji, player.to_uppercase(), timing);
            }
        }
    }

    // Check for ranking/placement markets (TOP, FINISH, PLACE)
    if ticker.contains("TOP") || ticker.contains("FINISH") || ticker.contains("PLACE") {
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(outcome) = parts.last() {
            let sport_emoji = get_sport_emoji(
                if ticker.contains("GOLF") { "golf" }
                else if ticker.contains("RACING") { "racing" }
                else if ticker.contains("OLYMPICS") { "olympics" }
                else { "sports" }
            );
            
            return format!(
                "{} {} {} {}",
                sport_emoji,
                side_emoji,
                outcome.to_uppercase(),
                if betting_side == "YES" { "finishes in position 🏅" } else { "doesn't finish in position ❌" }
            );
        }
    }

    // Check for entertainment awards
    if ticker.contains("OSCAR") || ticker.contains("EMMY") || ticker.contains("GRAMMY") || ticker.contains("TONY") {
        let award_type = if ticker.contains("OSCAR") { "Oscar 🎬" }
            else if ticker.contains("EMMY") { "Emmy 📺" }
            else if ticker.contains("GRAMMY") { "Grammy 🎵" }
            else if ticker.contains("TONY") { "Tony 🎭" }
            else { "Award 🏆" };
        
        let parts: Vec<&str> = ticker.split('-').collect();
        if let Some(winner) = parts.last() {
            return format!(
                "{} {} {} wins {}",
                award_type,
                side_emoji,
                winner.to_uppercase(),
                if betting_side == "YES" { "YES ✅" } else { "NO ❌" }
            );
        }
    }

    // Default: try to extract outcome from last part
    let parts: Vec<&str> = ticker.split('-').collect();
    if let Some(outcome) = parts.last() {
        if outcome.len() <= 10 && outcome.chars().all(|c| c.is_alphanumeric()) {
            let outcome_emoji = get_team_emoji(outcome, None);
            if betting_side == "YES" {
                return format!("🎯 {} {} happens {}", side_emoji, outcome_emoji, outcome.to_uppercase());
            } else {
                return format!("🎯 {} {} doesn't happen {}", side_emoji, outcome_emoji, outcome.to_uppercase());
            }
        }
    }

    // Absolute fallback - show more context with emoji
    if betting_side == "YES" {
        format!("✅ {} YES - check market details", side_emoji)
    } else {
        format!("❌ {} NO - check market details", side_emoji)
    }
}
