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


#![forbid(unsafe_code)]

extern crate core;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use async_ctrlc::CtrlC;
use async_std::{fs, io, task};
use eyre::Result;
use crate::app::App;
use crate::cli::Cli;
use crate::database::Database;
use crate::website::Website;

mod config;
mod method;
mod app;
mod website;
mod cli;
mod database;

fn main() -> core::result::Result<(), eyre::Error> {
    use std::env;

    if let Err(env::VarError::NotPresent) = env::var("RUST_BACKTRACE") {
        env::set_var("RUST_BACKTRACE", "1");
        println!("Enabled RUST_BACKTRACE");
    }
    stable_eyre::install()?;

    task::block_on(async_main())
}

async fn async_main() -> Result<()> {
    fs::create_dir_all("config").await?;
    let config = config::Config::load("config.ron").await?;

    simple_logging::log_to_stderr(config.log_level());

    /*
    let tls = if config.enable_tls {
        let public_key = load_certificates("config/certificate.pem").await?;
        let private_key = load_private_key("config/certificate.rsa").await?;

        let mut cfg = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(public_key, private_key)?;
        // Configure ALPN to accept HTTP/2, HTTP/1.1 in that order.
        cfg.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        async_std::sync::Arc::new(cfg)
    } else {
        todo!()
    };
    */
    let database = Database {
        pool: sqlx::postgres::PgPool::connect_lazy(&config.postgres_url)?
    };

    if let Some(first_arg) = std::env::args().next() {
        if first_arg == "cli" {
            let cli = Cli {
                stdin: io::stdin(),
                stdout: io::stdout(),
                database
            };
            return cli.start().await;
        }
    }
    let app = App {
        database,
        website: Website {
            kayaking_image: include_bytes!("kayaking-background.webp"),
            client: include_bytes!("../../target/debug/libthebestofcmu_client.so")
        }
    };
    app.database.create_schema().await?;
    let socket =  SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), config.port);
    app.start_server(socket, shutdown_signal()).await
}

async fn shutdown_signal() {
    CtrlC::new().expect("Cannot create CTRL+C handler").await;
    log::info!("Shutting down....");
}

/*
async fn load_certificates(path: impl AsRef<Path>) -> Result<Vec<rustls::Certificate>> {
    let certificate = fs::read_to_string(path).await?;
    let mut cert_reader = std::io::Cursor::new(certificate);
    Ok(rustls_pemfile::certs(&mut cert_reader)?
        .into_iter()
        .map(rustls::Certificate)
        .collect())
}

async fn load_private_key(path: impl AsRef<Path>) -> Result<rustls::PrivateKey> {
    let private_key = fs::read_to_string(path).await?;
    let mut private_key_reader = std::io::Cursor::new(private_key);
    let mut keys = rustls_pemfile::rsa_private_keys(&mut private_key_reader)?.into_iter();

    return if let Some(private_key) = keys.next() {
        if let Some(_) = keys.next() {
            Err(eyre::eyre!("Too many keys"))
        } else {
            Ok(PrivateKey(private_key))
        }
    } else {
        Err(eyre::eyre!("No private keys found"))
    }
}
*/

