use anyhow::Result;

pub struct DiscordRpc;

impl DiscordRpc {
    pub fn new() -> Self {
        Self
    }

    pub async fn connect(&self) -> Result<bool> {
        Ok(false)
    }

    pub fn is_connected(&self) -> bool {
        false
    }
}
