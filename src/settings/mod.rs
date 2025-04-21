use maud::{Markup, Render, html};

pub struct List;

const SETTING_ITEM_CLASS: &str =
    "px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center";

impl Render for List {
    fn render(&self) -> Markup {
        html! {
            header ."text-center mb-4"{
                h2.text-2xl { "Settings" }
            }

            // TODO: align
            ul .space-y-2 {
                li .(SETTING_ITEM_CLASS) {
                    i .mr-2 ."fa-solid fa-arrow-right-from-bracket" {}
                    a href="/logout" { "Logout" }
                }
                li .(SETTING_ITEM_CLASS) {
                    span _="on click askNotificationPermission()" {
                        i .mr-2 ."fa-regular fa-bell-slash" {}
                        "Enable notifications"
                    }
                }
            }
        }
    }
}
