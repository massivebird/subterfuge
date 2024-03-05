#[derive(Debug)]
pub struct Game {
    pub name: String,
    pub app_id: u32,
    pub playtime_forever: u32,
}

impl Game {
    pub const fn new(name: String, app_id: u32, playtime_forever: u32) -> Self {
        Self {
            name,
            app_id,
            playtime_forever,
        }
    }
}

impl PartialEq<Self> for Game {
    fn eq(&self, other: &Self) -> bool {
        other.app_id == self.app_id && other.playtime_forever == self.playtime_forever
    }
}
