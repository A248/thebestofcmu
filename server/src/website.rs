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

use hyper::{Body, Uri};
use hyper::http::uri;
use thebestofcmu_common::PostPath;

pub struct Website {
    pub favicon: &'static [u8],
    pub kayaking_image: &'static [u8]
}

fn request_path(request_uri: &uri::Parts) -> &str {
    (&request_uri.path_and_query)
        .as_ref()
        .map(|path| path.path())
        .unwrap_or("/")
}

impl Website {
    pub fn validate_post_path(&self, request_uri: Uri) -> Option<PostPath> {
        let request_uri = request_uri.into_parts();
        let request_path = request_path(&request_uri);
        PostPath::from_str(if request_path.starts_with('/') { &request_path[1..] } else { request_path })
    }

    pub async fn yield_site_body(&self, request_uri: Uri) -> Option<Body> {
        let request_uri = request_uri.into_parts();
        let request_path = request_path(&request_uri);
        Some(match request_path {
            "/" => Body::from(main_page_content()),
            "/favicon.ico" => Body::from(self.favicon),
            "/kayaking-background.webp" => Body::from(self.kayaking_image),
            _ => return None
        })
    }

}

fn main_page_content() -> &'static str {
    r#"
<!DOCTYPE html>
<head></head>
<body>
<h1 style="color: #5e9ca0; text-align: center;">Welcome, to the First Day of Class</h1>
<p style="text-align: center;">You are hereby invited to come kayaking on the pristine waters of River Allegheny. The river, located far off to the north, beyond city limits, is a faraway place of wonder where a CMU student is a rare sight to behold. In a valley rimmed with vibrant treetops, exotic birds fly to and fro while fish dance in the water. Unlike the tumult of academic life, all elements of this valley cohere and are at harmony with one another. The river waters the plants, whose roots in turn hold the earthwork, preventing erosion; while the tree leaves provide shadow to the water and shelter to all that lives within.</p>
<p style="text-align: center;">Yet there can be no serenity without danger, for the river is swift and merciless. From the depths of the current swell monstrous rocks and boulders, creating a continuous challenge of navigation for the few voyagers who chance this way. Those fortunate enough to survive, tell tall tales of adventure.</p>
<p style="text-align: center;">This website is for fun: entirely theatrical. The location, exaggerated. All the same, kayaking is an enjoyable activity, whether you prefer strenous exertion or relaxing vacation. This school year, surely, will be a spectacular one.</p>
<ul>
<li style="text-align: left;"><strong>Date:</strong> 3 September 2022</li>
<li style="text-align: left;"><strong>Time and Place:</strong> Meet at&nbsp;12:15 PM, <em><strong>sharp,</strong></em> at Fifth &amp; Craig intersection (St. Paul's Cathedral)</li>
<li style="text-align: left;"><strong>Cost:</strong> $40, cash only</li>
</ul>
<p style="text-align: left;">To RSVP, please reply by SMS to the coordinator who linked you to this website. If you want to invite anyone else, please ask the coordinator.</p>
<p style="text-align: center;">&nbsp;</p>
<p><img style="display: block; margin-left: auto; margin-right: auto;" src="./kayaking-background.webp" alt="kayaking-image" width="1200" height="795" /></p>
<div id="spinner" style="position: relative;">
  <div class="spinner">Loading...</div>
</div>
<script type="module">
  import init from './pkg/thebestofcmu-client.js';
  init().finally(() => {
    document.getElementById("spinner").remove();
  });
</script>
<p style="text-align: right;">Source code available upon written request.</p>
</body>
</html>
    "#
}

#[cfg(test)]
mod tests {
    use super::*;
    use eyre::Result;
    use hyper::http::uri::PathAndQuery;

    #[test]
    fn post_path() -> Result<()> {
        let website = Website { favicon: &[], kayaking_image: &[] };
        let uri = Uri::builder()
            .path_and_query(PathAndQuery::from_static("/enter-rsvp"))
            .build()?;
        assert_eq!(Some(PostPath::EnterRsvp), website.validate_post_path(uri));
        Ok(())
    }
}
