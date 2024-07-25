// !! Critical knowledge !!
//
// Steam updates data returned by the API:
// 1) When a game session ends, and
// 2) Every 30 minutes a game session is active.
//
// I'm still unsure if this data is separate from Steam profile page data.

use clap::Arg;
use game::Game;
use rand::prelude::*;
use std::fs::File;
use std::string;
use std::{fs::read_to_string, thread, time::Duration};
use user::User;
use yaml_rust2::{Yaml, YamlLoader};

mod game;
mod user;

fn main() {
    let matches = clap::command!()
        .args([
            Arg::new("api_key")
                .short('k')
                .long("api-key")
                .alias("key")
                .required(false)
                .value_hint(clap::ValueHint::FilePath)
                .value_name("PATH")
                .help("Path to a file containing a Steam API key."),
            Arg::new("config")
                .short('c')
                .long("config-file")
                .alias("config")
                .required(false)
                .value_hint(clap::ValueHint::FilePath)
                .value_name("PATH")
                .help("Path to the YAML config file."),
            Arg::new("user_ids")
                .long("user-ids")
                .alias("users")
                .conflicts_with("config")
                .required(false)
                .value_name("user-IDs")
                .help("Comma-separated user IDs."),
        ])
        .get_matches();

    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    // Reads data from a file.
    //
    // # Arguments
    //
    // (1) Human readable name
    // (2) clap argument ID corresponding to user-specified path
    // (3) File name of default path relative to $HOME/.config/subterfuge
    let dynamic_read = |human_name: &str, arg_id: &str, file_name: &str| {
        let absolute_path = matches.get_one::<String>(arg_id).map_or_else(
            || {
                let home_dir = std::env::var("HOME").unwrap();
                let default_path = format!("{home_dir}/.config/subterfuge/{file_name}");
                log::warn!("Defaulting to {human_name} path: {default_path}");
                default_path
            },
            string::ToString::to_string,
        );

        if File::open(&absolute_path).is_err() {
            panic!("Provided {human_name} path does not exist.");
        }

        let Ok(file_contents) = read_to_string(&absolute_path) else {
            panic!("Failed to read {human_name} file (the file DOES exist though).");
        };

        log::info!("Loaded {human_name} successfully.");

        file_contents
    };

    let api_key = dynamic_read("API key", "api_key", "steam_api_key.secret");

    let users: Vec<User> = 'block: {
        if let Some(user_ids) = matches.get_one::<String>("user_ids") {
            break 'block user_ids
                .split(&[',', ' '])
                .map(|id| User::new(&api_key, id, None))
                .collect();
        }

        let config_contents = dynamic_read("config", "config", "config.yaml");

        let yaml = YamlLoader::load_from_str(&config_contents)
            .expect("Failed to parse configuration file into YAML.");

        let users_yaml: &Yaml = &yaml[0]["users"];

        assert!(
            !users_yaml.is_badvalue(),
            "Failed to locate `users` key in config file."
        );

        let mut users = Vec::new();

        let Some(defined_users) = users_yaml.as_hash() else {
            log::warn!("Aborting program: no users defined in config.");
            return;
        };

        for (label, properties) in defined_users {
            let Some(label) = label.as_str() else {
                panic!("Failed to process label: {label:?}");
            };

            let steam_id = {
                let Some(raw_id) = properties["id"].as_i64() else {
                    panic!("Failed to process field `id` for user labeled `{label}`");
                };

                let id_str = raw_id.to_string();

                assert!(
                    id_str.len() == 17,
                    "Invalid Steam ID {raw_id}: expected 17 characters"
                );

                id_str
            };

            let alias: Option<&str> = properties["alias"].as_str();

            users.push(User::new(&api_key, &steam_id, alias));
        }

        users
    };

    // Thread scope waits for all children threads to finish.
    // The compiler knows that the variables above will outlive
    // these children threads, allowing us to pass refs to them.
    std::thread::scope(|scope| {
        let api_key_ref = &api_key;

        for user in &users {
            scope.spawn(move || watch_user(api_key_ref, user));
        }
    });
}

fn watch_user(api_key: &str, user: &User) {
    let steam_id = &user.steam_id;
    let display_name = &user.display_name;

    log::info!("Initialized user: {user}");

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

    // Sleeps for a "random" number of seconds.
    // Staggers subroutines so they do not (always) make calls simultaneously.
    let nap = || {
        thread::sleep(Duration::from_secs(rand::thread_rng().gen_range(60..95)));
    };

    loop {
        let Ok(response) = recent_games_request.try_clone().unwrap().send() else {
            log::warn!("Failed to send API request for {user}");
            nap();
            continue;
        };

        let response_text: String = response.text().unwrap();

        let Ok(json_values) = json::parse(&response_text) else {
            // JSON parsing fails sometimes because HTML is returned instead.
            // Could be a request timeout. Let's find out!
            log::error!("Failed parsing response for {display_name}: {response_text}");
            nap();
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
            nap();
            continue;
        }

        // Continue if recently played games have not changed.
        if games.iter().all(|g| games_cache.iter().any(|o| o == g)) {
            nap();
            continue;
        }

        // Recently played games has changed!
        // Find games that:
        // (1) Aren't in the cache yet, or
        // (2) Are in the cache, but have a new total playtime.
        let discrepants: Vec<&Game> = games.iter().filter(|g| !games_cache.contains(g)).collect();

        for discr in discrepants {
            let total_playtime = discr.playtime_forever;

            // If the discrepant game isn't in the cache, then this is the first
            // session in the last two weeks. Cannot calculate session playtime.
            let Some(discr_cached_ver) = games_cache.iter().find(|g| g.app_id == discr.app_id)
            else {
                log::info!("Detected activity for {display_name}. Game: {discr}. First session in two weeks. Total: {total_playtime} min.");
                nap();
                continue;
            };

            let prev_playtime = discr_cached_ver.playtime_forever;
            let delta_total_playtime = total_playtime - prev_playtime;

            log::info!("Detected activity for {display_name}. Game: {discr}. Session: {delta_total_playtime} min. Total: {total_playtime} min.");
        }

        games_cache = games;
        nap();
    }
}
