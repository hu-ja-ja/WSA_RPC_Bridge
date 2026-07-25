use anyhow::Result;

pub struct AdbClient;

impl AdbClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn connect(&self) -> Result<bool> {
        Ok(false)
    }

    pub async fn get_media_session(&self) -> Result<String> {
        Ok(String::new())
    }
}
