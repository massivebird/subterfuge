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

    let mut response_cache = String::new();

    loop {
        log("Sleeping...");
        std::thread::sleep(std::time::Duration::new(10, 0));
        let response = request.try_clone().unwrap().send().unwrap();
        let response_text = response.text().unwrap();

        if response_cache.is_empty() {
            response_cache = response_text;
            log("Updated response cache");
            continue;
        }

        if response_text == response_cache { continue }

        log("Response has changed!");

        let mut parsed = json::parse(&response_text).unwrap();

        // this ISN'T the latest game rn. I think they are ordered
        // by playtime_forever descending.
        let latest_game = &parsed["response"]["games"].pop();
        let game_name = latest_game["name"].to_string();
        let playtime = latest_game["playtime_forever"].as_u32().unwrap();

        log(&format!("Currently playing: {game_name}: Total playtime: {playtime}"));
    }
}

fn log(msg: &str) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    println!("[{now}]: {msg}");
}
