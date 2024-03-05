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

    let mut response_cache: Option<reqwest::blocking::Response> = None;

    let update_response_cache = |r: reqwest::blocking::Response| {
        response_cache = Some(r);
        log("Updated response cache");
    };

    loop {
        std::thread::sleep(std::time::Duration::new(20, 0));
        let response = request.try_clone().unwrap().send().unwrap();

        if response_cache.is_none() {
            response_cache = Some(response);
            log("Updated response cache");
            continue;
        }

        if &response.text().unwrap() == &response_cache.unwrap().text().unwrap() { continue }

        log("Response has changed!");

        let mut parsed = json::parse(&response.text().unwrap()).unwrap();

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
