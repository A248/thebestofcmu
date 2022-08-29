/*
 * thebestofcmu
 * Copyright © 2022 Anand Beh
 *
 * thebestofcmu is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * thebestofcmu is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with thebestofcmu. If not, see <https://www.gnu.org/licenses/>
 * and navigate to version 3 of the GNU Affero General Public License.
 */

use std::str::FromStr;
use async_std::io::BufWriter;
use async_std::fs;
use async_std::fs::OpenOptions;
use async_std::io::WriteExt;
use async_std::path::Path;
use eyre::Result;
use log::LevelFilter;
use ron::ser::PrettyConfig;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
    pub postgres_url: String,
    pub port: u16,
    pub enable_tls: bool,
    pub log_level: String
}

impl Default for Config {
    fn default() -> Self {
        Self {
            postgres_url: String::new(),
            port: 8080,
            enable_tls: false,
            log_level: String::from("DEBUG")
        }
    }
}

impl Config {
    pub fn log_level(&self) -> LevelFilter {
        let log_level = &self.log_level;
        LevelFilter::from_str(log_level).unwrap_or_else(|_a| {
            log::warn!("Unknown log level: {}. Using DEBUG", log_level);
            LevelFilter::Debug
        })
    }

    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Ok(if path.exists().await {
            let file_content = fs::read_to_string(path).await?;
            ron::from_str(&file_content)?
        } else {
            let default_conf = Self::default();
            default_conf.clone().write_to(path).await?;
            default_conf
        })
    }

    pub async fn write_to(self, path: &Path) -> Result<()> {
        // Write default config
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path).await?;
        let mut writer = BufWriter::new(file);
        writer.write_all(
            ron::ser::to_string_pretty(&self, PrettyConfig::default())?.as_bytes()
        ).await?;
        writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_std::path::PathBuf;
    use tempfile::TempDir;
    use super::*;

    fn temp_file_in(tempdir: &TempDir, filename: &str) -> PathBuf {
        let mut path = PathBuf::from(tempdir.path().as_os_str().to_os_string());
        path.push(filename);
        path
    }

    #[async_std::test]
    async fn write_default_config() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let path = temp_file_in(&tempdir, "config.ron");

        Config::default().write_to(&path).await?;
        Ok(())
    }

    #[async_std::test]
    async fn reload_config() -> Result<()> {
        let tempdir = tempfile::tempdir()?;
        let path = temp_file_in(&tempdir, "config.ron");

        let config = Config {
            postgres_url: String::from("my-url"),
            port: 8080,
            enable_tls: true,
            log_level: String::from("DEBUG")
        };
        config.clone().write_to(&path).await?;
        let reloaded = Config::load(&path).await.expect("Config ought to exist");
        assert_eq!(config, reloaded);
        Ok(())
    }
}