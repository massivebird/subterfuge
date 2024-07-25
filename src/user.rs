use std::{borrow, process, thread};

pub struct User {
    pub steam_id: String,
    pub display_name: String,
    alias: Option<String>,
}

impl User {
    pub fn new(api_key: &str, steam_id: &str, alias: Option<&str>) -> Self {
        if steam_id.len() != 17 {
            log::error!("Invalid Steam ID \"{steam_id}\": expected 17 numeric characters");
            process::exit(1);
        }

        let request = reqwest::blocking::Client::new()
            .get("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/")
            .query(&[
                ("key", api_key.trim()),
                ("steamids", steam_id.trim()),
                ("format", "json"),
            ]);

        let response_json = loop {
            let Ok(Ok(resp)) = request
                .try_clone()
                .unwrap()
                .send()
                .map(|resp| json::parse(&resp.text().unwrap()))
            else {
                thread::sleep(std::time::Duration::from_secs(5));
                log::error!(
                    "Failed to fetch data for ID ending in {}. Retrying...",
                    &steam_id[13..]
                );
                continue;
            };

            break resp;
        };

        let display_name = response_json["response"]["players"][0]["personaname"].to_string();

        Self {
            steam_id: steam_id.to_string(),
            display_name,
            alias: alias.map(borrow::ToOwned::to_owned),
        }
    }
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.alias
                .clone()
                .unwrap_or_else(|| self.display_name.clone())
        )
    }
}
