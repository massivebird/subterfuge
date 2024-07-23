use std::borrow;

pub struct User {
    pub steam_id: String,
    pub display_name: String,
    alias: Option<String>,
}

impl User {
    pub fn new(api_key: &str, steam_id: &str, alias: Option<&str>) -> Self {
        let request = reqwest::blocking::Client::new()
            .get("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/")
            .query(&[
                ("key", api_key.trim()),
                ("steamids", steam_id.trim()),
                ("format", "json"),
            ]);

        let response = loop {
            let Ok(resp) = request.try_clone().unwrap().send() else {
                std::thread::sleep(std::time::Duration::from_secs(5));
                log::error!(
                    "Failed to fetch data for user {}. Retrying...",
                    &steam_id[0..5]
                );
                continue;
            };

            break resp;
        };

        let display_name = json::parse(&response.text().unwrap()).unwrap()["response"]["players"]
            [0]["personaname"]
            .to_string();

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
