// !! Critical knowledge !!
//
// Steam updates data returned by the API:
// 1) When a game session ends, and
// 2) Every 30 minutes a game session is active.
//
// I'm still unsure if this data is separate from Steam profile page data.

use clap::Arg;
use game::Game;
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
        ])
        .get_matches();

    simplelog::TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stdout,
        simplelog::ColorChoice::Always,
    )
    .unwrap();

    let api_key = {
        let api_key_path = matches.get_one::<String>("api_key").map_or_else(
            || {
                let default_path =
                    std::env::var("HOME").unwrap() + "/.config/subterfuge/steam_api_key.secret";
                log::warn!("Defaulting to API key path: {default_path}");
                default_path
            },
            string::ToString::to_string,
        );

        if std::fs::File::open(&api_key_path).is_err() {
            panic!("Provided API key path does not exist.");
        }

        let Ok(api_key) = read_to_string(&api_key_path) else {
            panic!("Failed to read API key file (the file DOES exist though).");
        };

        log::info!("Loaded API key successfully.");

        api_key
    };

    let users: Vec<User> = {
        let config_path = matches.get_one::<String>("config").map_or_else(
            || {
                let default_path =
                    std::env::var("HOME").unwrap() + "/.config/subterfuge/config.yaml";
                log::warn!("Defaulting to config path: {default_path}");
                default_path
            },
            string::ToString::to_string,
        );

        if std::fs::File::open(&config_path).is_err() {
            panic!("Failed to locate config file at the provided path.");
        };

        let config_contents = std::fs::read_to_string(config_path).unwrap();

        log::info!("Loaded configuration file successfully.");

        let yaml = YamlLoader::load_from_str(&config_contents)
            .expect("Failed to parse configuration file.");

        let users_yaml: &Yaml = &yaml[0]["users"];

        assert!(
            !users_yaml.is_badvalue(),
            "Failed to locate `users` key in config file."
        );

        let mut users = Vec::new();

        for (label, properties) in users_yaml.as_hash().unwrap() {
            let Some(label) = label.as_str() else {
                panic!("Failed to process label: {label:?}");
            };

            let Some(steam_id) = properties["id"].as_i64() else {
                panic!("Failed to process field `id` for user labeled `{label}`");
            };

            let alias: Option<&str> = properties["alias"].as_str();

            users.push(User::new(&api_key, &steam_id.to_string(), alias));
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

    loop {
        thread::sleep(Duration::from_secs(90));

        let Ok(response) = recent_games_request.try_clone().unwrap().send() else {
            log::warn!("Failed to send API request for {user}");
            continue;
        };

        let response_text: String = response.text().unwrap();

        let Ok(json_values) = json::parse(&response_text) else {
            // JSON parsing fails sometimes because HTML is returned instead.
            // Could be a request timeout. Let's find out!
            log::error!("Failed parsing response for {display_name}: {response_text}");
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
        let discrepants: Vec<&Game> = games
            .iter()
            .filter(|&g| !games_cache.iter().any(|o| o == g))
            .collect();

        for discr in discrepants {
            let total_playtime = discr.playtime_forever;

            // If the discrepant game isn't in the cache, then this is the first
            // session in the last two weeks. Cannot calculate session playtime.
            let Some(discr_cached_ver) = games_cache.iter().find(|g| g.app_id == discr.app_id)
            else {
                log::info!("Detected activity for {display_name}. Game: {discr}. First session in two weeks. Total: {total_playtime} min.");
                continue;
            };

            let prev_playtime = discr_cached_ver.playtime_forever;
            let delta_total_playtime = total_playtime - prev_playtime;

            log::info!("Detected activity for {display_name}. Game: {discr}. Session: {delta_total_playtime} min. Total: {total_playtime} min.");
        }

        games_cache = games;
    }
}
