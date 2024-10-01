use maud::{html, Markup};

use crate::markup::base;

pub(super) async fn login() -> Markup {
    base(html! {
        h2 { "Please Login" }
        a href="/login" { "Login with SSO" }
    })
}
