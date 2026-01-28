use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const REFRESH_BUFFER_SECS: u64 = 600;
const RETRY_DELAY_SECS: u64 = 30;
const MIN_SLEEP_SECS: u64 = 60;

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

pub type OnTokenRotation = Box<dyn Fn(&str) + Send + Sync>;

#[non_exhaustive]
pub struct TokenManager {
    client: Client,
    client_id: String,
    client_secret: String,
    refresh_token: RwLock<String>,
    current_token: RwLock<Option<String>>,
    init_lock: Mutex<()>,
    on_rotation: Option<OnTokenRotation>,
}

impl TokenManager {
    #[must_use]
    pub fn new(client_id: String, client_secret: String, refresh_token: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            refresh_token: RwLock::new(refresh_token),
            current_token: RwLock::new(None),
            init_lock: Mutex::new(()),
            on_rotation: None,
        }
    }

    #[must_use]
    pub fn with_rotation_callback(mut self, callback: OnTokenRotation) -> Self {
        self.on_rotation = Some(callback);
        self
    }

    pub async fn get_token(&self) -> Result<String> {
        if let Some(token) = self.current_token.read().await.as_ref() {
            return Ok(token.clone());
        }

        let _guard = self.init_lock.lock().await;

        if let Some(token) = self.current_token.read().await.as_ref() {
            return Ok(token.clone());
        }

        let (token, _) = self.refresh_now().await?;
        Ok(token)
    }

    pub fn start_background_loop(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            info!("starting token refresh background task");

            loop {
                match self.refresh_now().await {
                    Ok((_, expires_in)) => {
                        let sleep_secs = expires_in
                            .saturating_sub(REFRESH_BUFFER_SECS)
                            .max(MIN_SLEEP_SECS);

                        info!("token refreshed. next refresh in {} seconds", sleep_secs);
                        tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
                    }
                    Err(e) => {
                        error!(
                            "failed to refresh token: {:?}. retrying in {}s...",
                            e, RETRY_DELAY_SECS
                        );
                        tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECS)).await;
                    }
                }
            }
        })
    }

    async fn refresh_now(&self) -> Result<(String, u64)> {
        let current_refresh = self.refresh_token.read().await.clone();

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", current_refresh.as_str()),
        ];

        let res = self
            .client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await?
            .error_for_status()
            .context("token refresh request failed")?
            .json::<TokenResponse>()
            .await
            .context("failed to parse token response")?;

        let full_token = format!("oauth:{}", res.access_token);

        {
            let mut lock = self.current_token.write().await;
            *lock = Some(full_token.clone());
        }

        let Some(new_rt) = res.refresh_token else {
            return Ok((full_token, res.expires_in));
        };

        if new_rt != current_refresh {
            warn!("twitch rotated the refresh token");

            {
                let mut rt_lock = self.refresh_token.write().await;
                *rt_lock = new_rt.clone();
            }

            if let Some(ref callback) = self.on_rotation {
                callback(&new_rt);
            }
        }

        Ok((full_token, res.expires_in))
    }

    /// Set a token directly, bypassing OAuth refresh. For testing only.
    #[cfg(any(test, feature = "test-support"))]
    pub async fn set_token_for_test(&self, token: String) {
        let mut lock = self.current_token.write().await;
        *lock = Some(token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> TokenManager {
        TokenManager::new(
            "test_client_id".to_string(),
            "test_secret".to_string(),
            "test_refresh".to_string(),
        )
    }

    #[tokio::test]
    async fn test_new_creates_empty_token() {
        let manager = make_manager();
        let token = manager.current_token.read().await;
        assert!(token.is_none());
    }

    #[tokio::test]
    async fn test_get_token_returns_cached_when_present() {
        let manager = make_manager();

        manager
            .set_token_for_test("oauth:cached_token".to_string())
            .await;

        let result = manager.get_token().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "oauth:cached_token");
    }

    #[tokio::test]
    async fn test_refresh_token_stored_correctly() {
        let manager = make_manager();
        let rt = manager.refresh_token.read().await;
        assert_eq!(*rt, "test_refresh");
    }

    #[tokio::test]
    async fn test_with_rotation_callback_sets_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let manager = TokenManager::new(
            "id".to_string(),
            "secret".to_string(),
            "refresh".to_string(),
        )
        .with_rotation_callback(Box::new(move |_| {
            called_clone.store(true, Ordering::SeqCst);
        }));

        assert!(manager.on_rotation.is_some());
    }

    #[tokio::test]
    async fn test_race_condition_prevention_with_cached_token() {
        let manager = Arc::new(make_manager());

        manager
            .set_token_for_test("oauth:test123".to_string())
            .await;

        let mut handles = vec![];
        for _ in 0..10 {
            let m = manager.clone();
            handles.push(tokio::spawn(async move { m.get_token().await }));
        }

        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "oauth:test123");
        }
    }
}
