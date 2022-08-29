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


use std::fmt::Arguments;
use std::mem;
use eyre::Result;
use async_std::io::{Stdin, Stdout, WriteExt};
use time::format_description::FormatItem;
use time::OffsetDateTime;
use thebestofcmu_common::Invitee;
use crate::Database;

pub struct Cli {
    pub stdin: Stdin,
    pub stdout: Stdout,
    pub database: Database
}

impl Cli {

    pub async fn start(mut self) -> Result<()> {

        let mut buffer = String::new();
        loop {
            self.stdout.write_all(b"Enter command: invite, list-invites").await?;
            self.stdin.read_line(&mut buffer).await?;
            match buffer.as_str() {
                "invite" => {

                    self.stdout.write_all(b"Enter invitee name\n").await?;
                    buffer.clear();
                    self.stdin.read_line(&mut buffer).await?;
                    self.database.insert_invite(&buffer).await?;

                    self.stdout.write_fmt(format_args!("Invited {}\n", &buffer)).await?;
                },
                "list-invites" => {
                    self.list_invites().await?;
                }
                other => {
                    self.stdout.write_fmt(format_args!("Unknown command {}\n", other)).await?;
                }
            }
            buffer.clear();
        }
    }

    async fn list_invites(&mut self) -> Result<()> {
        let stdout = &mut self.stdout;

        stdout.write_all(b"ID | Name | RSVP'd?\n").await?;

        for mut invitee in self.database.select_invites().await? {

            async fn write_rsvp(stdout: &mut Stdout, invitee: Invitee, rsvp: Arguments<'_>) -> Result<()> {
                Ok(stdout.write_fmt(
                    format_args!("{} | {} | {}\n", invitee.id, invitee.first_name, rsvp)
                ).await?)
            }
            match mem::replace(&mut invitee.rsvp, None) {
                None => write_rsvp(&mut *stdout, invitee, format_args!("No")).await,
                Some((details, at_time)) => {
                    let at_time: OffsetDateTime = at_time.into();
                    let at_time = at_time.format(&FormatItem::Literal(b"%d/%m/%Y %T"))?;
                    write_rsvp(&mut *stdout, invitee,
                               format_args!("Yes, at date: {}. Details: \n    {}", at_time, details)).await
                }
            }?;
        }
        Ok(())
    }

}

