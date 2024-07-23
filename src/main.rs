// !! Critical knowledge !!
//
// Steam updates data returned by the API:
// 1) When a game session ends, and
// 2) Every 30 minutes a game session is active.
//
// I'm still unsure if this data is separate from Steam profile page data.

use game::Game;
use std::io::BufRead;
use std::{fs::read_to_string, thread, time::Duration};
use user::User;

mod game;
mod user;

fn main() {
    let Ok(api_key) = read_to_string("steam_api_key.secret") else {
        eprintln!("ERROR: missing API key file.");
        return;
    };

    let Ok(file) = std::fs::File::open("steam_ids.csv") else {
        eprintln!("ERROR: failed to open Steam IDs file.");
        return;
    };

    let steam_ids = std::io::BufReader::new(file).lines();

    // Thread scope waits for all children threads to finish.
    // The compiler knows that the variables above will outlive
    // these children threads, allowing us to pass refs to them.
    std::thread::scope(|scope| {
        let api_key_ref = &api_key;

        for id in steam_ids {
            scope.spawn(move || analyze_user(api_key_ref, &id.unwrap()));
        }
    });
}

macro_rules! log {
    ($($msg:tt)*) => {
        let date = chrono::Local::now().format("%H:%M:%S").to_string();
        let msg = format!($($msg)*);
        println!("[{date}] {msg}");
    };
}

fn analyze_user(api_key: &str, steam_id: &str) {
    let request = reqwest::blocking::Client::new()
        .get("http://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/")
        .query(&[
            ("key", api_key.trim()),
            ("steamid", steam_id.trim()),
            ("format", "json"),
        ]);

    let user = User::new(api_key, steam_id);
    let persona_name = &user.persona_name;

    log!("User initialized: {user}");

    // Used to calculate game session length
    let mut games_cache: Vec<Game> = Vec::new();

    loop {
        thread::sleep(Duration::new(90 /* secs */, 0 /* nanos */));

        let Ok(response) = request.try_clone().unwrap().send() else {
            log!("WARNING: request for {user} failed.");
            continue;
        };

        let response_text: String = response.text().unwrap();

        let Ok(parsed) = json::parse(&response_text) else {
            // JSON parsing fails sometimes because HTML is returned instead.
            // Could be a request timeout. Let's find out!
            dbg!(response_text);
            log!("JSON parsing failed for {persona_name}. See above for details.");
            continue;
        };

        let games: Vec<Game> = parsed["response"]["games"]
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

        // if games are unchanged since last cache, nothing to report
        if games.iter().all(|g| games_cache.iter().any(|o| o == g)) {
            continue;
        }

        // find the discrepant game that corresponds to none in the cache
        let latest_game: &Game = games
            .iter()
            .find(|&g| !games_cache.iter().any(|o| o == g))
            .unwrap();
        let game_name = &latest_game.name;
        let total_playtime = latest_game.playtime_forever;

        // this game was cached only if it has been played in the last two weeks;
        // otherwise, we have no previous playtime to compare to.
        let Some(latest_game_cached) = games_cache.iter().find(|g| g.app_id == latest_game.app_id)
        else {
            log!("Activity detected for {persona_name}. Game: {game_name}. First session in two weeks. Total: {total_playtime} min.");
            games_cache = games;
            continue;
        };

        let prev_playtime = latest_game_cached.playtime_forever;
        let delta_total_playtime = total_playtime - prev_playtime;

        log!("Activity detected for {persona_name}. Game: {game_name}. Session: {delta_total_playtime} min. Total: {total_playtime} min.");

        games_cache = games;
    }
}
