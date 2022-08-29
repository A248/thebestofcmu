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

use hyper::{Method, Response, Body, http, StatusCode};
use eyre::Result;
use crate::method::AllowedMethod::{GET, HEAD, POST};

const ALL_ALLOWED: &[AllowedMethod] = &[GET, HEAD, POST];

#[derive(Debug, Copy, Clone)]
pub enum AllowedMethod {
    GET,
    HEAD,
    POST
}

impl From<&AllowedMethod> for Method {

    fn from(allowed_method: &AllowedMethod) -> Self {
        match allowed_method {
            GET => Method::GET,
            HEAD => Method::HEAD,
            POST => Method::POST
        }
    }
}

impl AllowedMethod {

    pub fn find_from(method: &Method) -> Option<Self> {
        Some(match method {
            &Method::GET => GET,
            &Method::HEAD => HEAD,
            &Method::POST => POST,
            _ => return None
        })
    }

    fn value(&self) -> Box<str> {
        let method: Method = self.into();
        Box::from(method.as_str())
    }
}

impl AllowedMethod {

    pub fn method_not_alllowed(version: http::version::Version) -> Result<Response<Body>> {
        let mut response = Response::builder()
            .version(version)
            .status(StatusCode::METHOD_NOT_ALLOWED);
        {
            let headers = response.headers_mut().unwrap();
            for allowed_method in ALL_ALLOWED {
                let method: Method = allowed_method.into();
                headers.append("Allow", method.as_str().parse()?);
            }
        }
        let allowed_methods_display = ALL_ALLOWED
            .iter()
            .map(AllowedMethod::value)
            .collect::<Vec<Box<str>>>()
            .join(", ");
        let message = format!("Only {} requests are allowed to thebestofcmu.", allowed_methods_display);
        Ok(response.body(Body::from(message))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respond_with_405() -> Result<()> {
        AllowedMethod::method_not_alllowed(http::version::Version::HTTP_2)?;
        Ok(())
    }

    #[test]
    fn convert_methods() {
        for method in &[Method::GET, Method::HEAD] {
            let allowed_method = AllowedMethod::find_from(method).unwrap();
            let back: Method = (&allowed_method).into();
            assert_eq!(method, back);
        }
    }
}