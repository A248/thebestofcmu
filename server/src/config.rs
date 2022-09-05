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
use async_std::fs;
use eyre::Result;
use log::LevelFilter;
use ron::ser::PrettyConfig;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
    pub postgres_url: String,
    pub host: String,
    pub port: u16,
    pub tls: Tls,
    pub log_level: String
}

impl Default for Config {
    fn default() -> Self {
        Self {
            postgres_url: String::new(),
            host: String::from("localhost"),
            port: 8080,
            tls: Default::default(),
            log_level: String::from("DEBUG")
        }
    }
}


#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tls {
    pub enable: bool,
    pub client_auth: bool
}

impl Config {
    pub fn log_level(&self) -> LevelFilter {
        let log_level = &self.log_level;
        LevelFilter::from_str(log_level).unwrap_or_else(|_a| {
            log::warn!("Unknown log level: {}. Using DEBUG", log_level);
            LevelFilter::Debug
        })
    }

    pub async fn load(file: &ConfigFile<'_>) -> Result<Self> {
        let config = file.read_content_with_default(|| {
            let default_conf = Self::default();
            Ok(ron::ser::to_string_pretty(&default_conf, PrettyConfig::default())?)
        }).await?;
        Ok(ron::from_str(&config)?)
    }

}

pub struct ConfigFile<'c> {
    path: &'c str,
    env_var: &'c str
}

impl<'c> ConfigFile<'c> {
    pub fn new(path: &'c str, env_var: &'c str) -> Self {
        Self { path, env_var }
    }

    pub async fn read_content(&self) -> Result<String> {
        fn non_existent() -> Result<String> {
            Err(eyre::eyre!("Should never be called"))
        }
        self.read_content_impl(false, non_existent).await
    }

    pub async fn read_content_with_default<D>(&self, default: D) -> Result<String>
        where D: FnOnce() -> Result<String> {

        self.read_content_impl(true, default).await
    }

    async fn read_content_impl<D>(&self, use_default: bool, default: D) -> Result<String>
        where D: FnOnce() -> Result<String> {

        Ok(if let Some(environment_value) = std::env::var_os(self.env_var) {
            match environment_value.to_str() {
                Some(result) => result.to_string(),
                None => return Err(eyre::eyre!("Not valid UTF-8: {:?}", environment_value))
            }
        } else {
            let path = self.path;
            if use_default {
                let default_content = default()?;
                fs::write(path, &default_content).await?;
                default_content
            } else {
                fs::read_to_string(path).await?
            }
        })
    }
}
