use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use crate::config::{Config, ConfigError};

const DEFAULT_CONFIG_PATH: &str = "./config.yaml";
const REFRESH_TOKEN_FIELD: &str = "twitch.auth.refresh_token";

pub(crate) struct ConfigFile {
    path: PathBuf,
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_CONFIG_PATH),
        }
    }
}

impl ConfigFile {
    pub fn load(&self) -> Result<Config, ConfigError> {
        Ok(serde_yaml::from_str(&self.read()?)?)
    }

    /// serde_yaml не сохраняет комментарии и форматирование:
    /// после ротации токена config.yaml будет переписан без них.
    pub fn save_refresh_token(&self, token: &str) -> Result<(), ConfigError> {
        let mut document: serde_yaml::Value = serde_yaml::from_str(&self.read()?)?;

        let refresh_token = document
            .get_mut("twitch")
            .and_then(|twitch| twitch.get_mut("auth"))
            .and_then(|auth| auth.get_mut("refresh_token"))
            .ok_or(ConfigError::MissingField(REFRESH_TOKEN_FIELD))?;
        *refresh_token = token.into();

        self.write_atomically(&serde_yaml::to_string(&document)?)
    }

    fn read(&self) -> Result<String, ConfigError> {
        fs::read_to_string(&self.path).map_err(|source| self.io_error(source))
    }

    fn write_atomically(&self, content: &str) -> Result<(), ConfigError> {
        let tmp_path = self.path.with_extension("yaml.tmp");

        let write = || -> io::Result<()> {
            // В файле секреты: права копируются до записи содержимого,
            // иначе временный файл успеет полежать с правами по умолчанию.
            let permissions = fs::metadata(&self.path)?.permissions();
            let mut file = File::create(&tmp_path)?;
            file.set_permissions(permissions)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            fs::rename(&tmp_path, &self.path)
        };

        write().map_err(|source| self.io_error(source))
    }

    fn io_error(&self, source: io::Error) -> ConfigError {
        ConfigError::Io {
            path: self.path.display().to_string(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const CONFIG: &str = r#"
twitch:
  auth:
    broadcaster_id: "1"
    client_id: "id"
    client_secret: "secret"
    refresh_token: "old"
    writer_id: "2"
"#;

    #[test]
    fn save_refresh_token_replaces_token_and_keeps_permissions() {
        let path =
            std::env::temp_dir().join(format!("twitch-bot-config-{}.yaml", std::process::id()));
        fs::write(&path, CONFIG).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let config_file = ConfigFile { path: path.clone() };

        config_file.save_refresh_token("new").unwrap();

        let config = config_file.load().unwrap();
        assert_eq!(config.twitch.auth.refresh_token.as_str(), "new");
        assert_eq!(config.twitch.auth.client_id.as_str(), "id");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );

        fs::remove_file(path).unwrap();
    }
}
