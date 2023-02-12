use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ChannelPointRedeem {
    pub id: String,
    pub user_login: String,
    pub user_id: String,
    pub user_input: Option<String>,
    pub reward: Reward,
}

#[derive(Debug, Deserialize)]
pub struct Reward {
    pub id: String,
    pub title: String,
}

impl PartialEq for Reward {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Reward {}
