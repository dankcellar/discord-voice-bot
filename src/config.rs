use std::env;
use std::error::Error;

pub struct Config {
    pub discord_token: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        dotenv::dotenv().ok();
        let discord_token = env::var("DISCORD_TOKEN").map_err(|_| "DISCORD_TOKEN not set")?;
        Ok(Self { discord_token })
    }
}
