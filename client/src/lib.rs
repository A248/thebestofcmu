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

use hyper::Client;
use wasm_bindgen::prelude::*;

slint::slint! {
    Survey := Window {
        Text {
            text: "Hello world";
            color: green;
        }
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let tls_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_only()
        .enable_http1()
        .enable_http2()
        .build();
    let client: Client<_, hyper::Body> = Client::builder()
        .build(tls_connector);
    Survey::new().run();
    Ok(())
}
