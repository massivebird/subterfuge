// !! Critical knowledge !!
//
// Steam updates data returned by the API:
// 1) When a game session ends, and
// 2) Every 30 minutes a game session is active.
//
// I'm still unsure if this data is separate from Steam profile page data.

use game::Game;
use std::fs::read_to_string;
use std::thread;
use std::time::Duration;

mod game;

fn main() {
    let api_key = &read_to_string("/home/penguino/sandbox/steam_api_key").unwrap();
    let api_key = api_key.trim();

    // mine: 76561198748465236
    let steam_id = "76561198748465236";

    let request = reqwest::blocking::Client::new()
        .get("http://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/")
        .query(&[("key", api_key), ("steamid", steam_id), ("format", "json")]);

    let mut games_cache: Vec<Game> = Vec::new();

    loop {
        thread::sleep(Duration::new(10 /* secs */, 0 /* nanos */));
        let response = request.try_clone().unwrap().send().unwrap();

        let games: Vec<Game> = json::parse(&response.text().unwrap()).unwrap()["response"]["games"]
            .members()
            .map(|g| {
                Game::new(
                    g["name"].to_string(),
                    g["appid"].as_u32().unwrap(),
                    g["playtime_forever"].as_u32().unwrap(),
                )
            })
            .collect();

        if games_cache.is_empty() {
            games_cache = games;
            continue;
        }

        // games are unchanged since last cache, nothing to report
        if games.iter().all(|g| games_cache.iter().any(|o| o == g)) {
            continue;
        }

        // find the discrepant game that corresponds to none in the cache
        let latest_game: &Game = games
            .iter()
            .find(|&g| !games_cache.iter().any(|o| o == g))
            .unwrap();

        let prev_playtime = games_cache
            .iter()
            .find(|g| g.app_id == latest_game.app_id)
            .unwrap()
            .playtime_forever;

        let game_name = &latest_game.name;
        let new_playtime = latest_game.playtime_forever;
        let delta_playtime = new_playtime - prev_playtime;

        log(&format!(
            "User has been playing {game_name}. Played for {delta_playtime} minute. Total of {new_playtime} minutes."
        ));

        games_cache = games;
    }
}

fn log(msg: &str) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    println!("[{now}]: {msg}");
}
