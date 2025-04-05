use maud::{Markup, Render, html};

pub struct List;

impl Render for List {
    fn render(&self) -> Markup {
        html! {
            header ."text-center mb-4"{
                h2.text-2xl { "Settings" }
            }

            // TODO: align
            ul {
                li .mb-3 {
                    a href="/logout" { "Logout" }
                }
                li .mb-3 {
                    span {
                        i #noti-bell .mr-2
                            ."fa-regular fa-bell-slash"
                            _="on click askNotificationPermission()"{}
                        "Enable notifications"
                    }
                }
            }
        }
    }
}
