// !! Critical knowledge !!
//
// My testing demonstrates that Steam doesn't update playtime accessible via
// the web API until _after a session._
//
// This doesn't totally invalidate the use cases for this program; it moreso
// affects how I conduct tests in the future.
//
// Besides, maybe there is are playtime thresholds at which Steam updates
// the web API values, such as every hour a session is live.
//
// Also, I should compare these values to those displayed on the front-facing
// Steam profile page.

use std::fs::read_to_string;

fn main() {
    let api_key = &read_to_string("/home/penguino/sandbox/steam_api_key").unwrap();
    let api_key = api_key.trim();

    // mine: 76561198748465236
    let steam_id = "76561198748465236";

    let client = reqwest::blocking::Client::new();
    let request = client.get("http://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/")
        .query(&[
            ("key", api_key),
            ("steamid", steam_id),
            ("format", "json"),
        ]);

    let mut games_cache: Vec<Game> = Vec::new();

    loop {
        let response = request.try_clone().unwrap().send().unwrap();

        let parsed = json::parse(&response.text().unwrap()).unwrap();

        let games: Vec<Game> = parsed["response"]["games"]
            .members()
            .map(|g| Game::new(
                g["name"].to_string(),
                g["appid"].as_u32().unwrap(),
                g["playtime_forever"].as_u32().unwrap()
            ))
            .collect();

        if games_cache.is_empty() {
            games_cache = games;
            log("Initialized games cache");
            log("Sleeping...");
            std::thread::sleep(std::time::Duration::new(10, 0));
            continue;
        }

        if games.iter().all(|g| games_cache.iter().any(|o| o == g)) {
            log("Games cache is unchanged.");
            log("Sleeping...");
            std::thread::sleep(std::time::Duration::new(10, 0));
            continue;
        }

        log("Detected a change in API response.");

        let latest_game = games.iter().find(|&g| !games_cache.iter().any(|o| o == g)).unwrap();
        let game_name = &latest_game.name;
        let playtime = latest_game.playtime_forever;

        log(&format!("Currently playing: {game_name}: Total playtime: {playtime}"));
        log("Sleeping...");
        std::thread::sleep(std::time::Duration::new(10, 0));
    }
}

fn log(msg: &str) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    println!("[{now}]: {msg}");
}

#[derive(Debug)]
struct Game {
    name: String,
    app_id: u32,
    playtime_forever: u32,
}

impl Game {
    const fn new(name: String, app_id: u32, playtime_forever: u32) -> Self {
        Self { name, app_id, playtime_forever }
    }
}

impl PartialEq<Self> for Game {
    fn eq(&self, other: &Self) -> bool {
        other.app_id == self.app_id && other.playtime_forever == self.playtime_forever
    }
}
