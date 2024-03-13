pub struct User {
    pub steam_id: String,
    pub persona_name: String,
}

impl User {
    pub fn new(api_key: &str, steam_id: &str) -> Self {
        let request = reqwest::blocking::Client::new()
            .get("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/")
            .query(&[
                ("key", api_key.trim()),
                ("steamids", steam_id.trim()),
                ("format", "json"),
            ]);

        let response = request.try_clone().unwrap().send().unwrap();

        let persona_name = json::parse(&response.text().unwrap()).unwrap()["response"]["players"]
            [0]["personaname"]
            .to_string();

        Self {
            steam_id: steam_id.to_string(),
            persona_name,
        }
    }
}

impl std::fmt::Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.persona_name)
    }
}
