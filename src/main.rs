use std::fs::read_to_string;

fn main() {
    let api_key = &read_to_string("/home/penguino/sandbox/steam_api_key").unwrap();
    let api_key = api_key.trim();

    // mine: 76561198748465236
    let steam_id = "76561198748465236";

    let client = reqwest::blocking::Client::new();
    let response = client.get("http://api.steampowered.com/IPlayerService/GetRecentlyPlayedGames/v0001/")
        .query(&[
            ("key", api_key),
            ("steamid", steam_id),
            ("format", "json"),
        ])
        .send()
        .unwrap();

    dbg!(response.text());
}
