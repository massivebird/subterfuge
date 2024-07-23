// !! Critical knowledge !!
//
// Steam updates data returned by the API:
// 1) When a game session ends, and
// 2) Every 30 minutes a game session is active.
//
// I'm still unsure if this data is separate from Steam profile page data.

use clap::Arg;
use game::Game;
use std::io::{BufRead, Lines};
use std::{fs::read_to_string, thread, time::Duration};
use user::User;

mod game;
mod user;

fn main() {
    let matches = clap::command!()
        .args([Arg::new("api_key")
            .short('k')
            .long("api-key")
            .alias("key")
            .required(false)
            .value_hint(clap::ValueHint::FilePath)
            .value_name("PATH")
            .help("Path to a file containing a Steam API key.")])
        .get_matches();

    let api_key = {
        let api_key_path = matches
            .get_one::<String>("api_key")
            .map(String::to_owned)
            .or_else(|| Some("steam_api_key.secret".to_string()))
            .unwrap();

        if std::fs::File::open(&api_key_path).is_err() {
            panic!("Provided API key path does not exist.");
        }

        let Ok(api_key) = read_to_string(&api_key_path) else {
            panic!("Failed to read API key file (the file DOES exist though).");
        };

        api_key
    };

    let steam_ids: Lines<_> = {
        let Ok(file) = std::fs::File::open("steam_ids.csv") else {
            panic!("Failed to open Steam IDs file.");
        };

        std::io::BufReader::new(file).lines()
    };

    // Thread scope waits for all children threads to finish.
    // The compiler knows that the variables above will outlive
    // these children threads, allowing us to pass refs to them.
    std::thread::scope(|scope| {
        let api_key_ref = &api_key;

        for id in steam_ids {
            scope.spawn(move || watch_user(api_key_ref, &id.unwrap()));
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

fn watch_user(api_key: &str, steam_id: &str) {
    let user = User::new(api_key, steam_id);
    let display_name = &user.display_name;

    log!("User initialized: {user}");

    let recent_games_request = reqwest::blocking::Client::new()
        .get("http://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/")
        .query(&[
            ("key", api_key.trim()),
            ("steamid", steam_id.trim()),
            ("format", "json"),
        ]);

    // A persistent collection of recently played games.
    // Used to calculate game session length
    let mut games_cache: Vec<Game> = Vec::new();

    loop {
        thread::sleep(Duration::from_secs(90));

        let Ok(response) = recent_games_request.try_clone().unwrap().send() else {
            log!("WARNING: request for {user} failed.");
            continue;
        };

        let response_text: String = response.text().unwrap();

        let Ok(json_values) = json::parse(&response_text) else {
            // JSON parsing fails sometimes because HTML is returned instead.
            // Could be a request timeout. Let's find out!
            dbg!(response_text);
            log!("JSON parsing failed for {display_name}. See above for details.");
            continue;
        };

        let games: Vec<Game> = json_values["response"]["games"]
            .members()
            .map(|game_json| {
                Game::new(
                    game_json["name"].to_string(),
                    game_json["appid"].as_u32().unwrap(),
                    game_json["playtime_forever"].as_u32().unwrap(),
                )
            })
            .collect();

        // If the cache is empty, there is nothing to compare against.
        if games_cache.is_empty() {
            games_cache = games;
            continue;
        }

        // Continue if recently played games have not changed.
        if games.iter().all(|g| games_cache.iter().any(|o| o == g)) {
            continue;
        }

        // Recently played games has changed!
        // Find the game that:
        // (1) Isn't in the cache yet, or
        // (2) Is in the cache, but has a new total playtime.
        let discrepant: &Game = games
            .iter()
            .find(|&g| !games_cache.iter().any(|o| o == g))
            .unwrap();
        let discrepant_name = &discrepant.name;
        let total_playtime = discrepant.playtime_forever;

        // If the discrepant game isn't in the cache, then this is the first
        // session in the last two weeks. Cannot calculate session playtime.
        let Some(discrepant_cached_ver) = games_cache.iter().find(|g| g.app_id == discrepant.app_id)
        else {
            log!("Activity detected for {display_name}. Game: {discrepant_name}. First session in two weeks. Total: {total_playtime} min.");
            games_cache = games;
            continue;
        };

        let prev_playtime = discrepant_cached_ver.playtime_forever;
        let delta_total_playtime = total_playtime - prev_playtime;

        log!("Activity detected for {display_name}. Game: {discrepant_name}. Session: {delta_total_playtime} min. Total: {total_playtime} min.");

        games_cache = games;
    }
}
