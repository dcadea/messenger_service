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

            ul .space-y-2 {
                li .(SETTING_ITEM_CLASS) {
                    a .flex-grow href="/logout" {
                        i .mr-2 ."fa-solid fa-arrow-right-from-bracket" {}
                        "Logout"
                    }
                }
                li .(SETTING_ITEM_CLASS)
                    _="on click askNotificationPermission()"
                {
                    i .mr-2 ."fa-regular fa-bell-slash" {}
                    "Enable notifications"
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use maud::Render;

    use super::List;

    #[test]
    fn should_render_settings_list() {
        let expected = concat!(
            r#"<header class="text-center mb-4">"#,
            r#"<h2 class="text-2xl">Settings</h2>"#,
            "</header>",
            r#"<ul class="space-y-2">"#,
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center">"#,
            r#"<a class="flex-grow" href="/logout">"#,
            r#"<i class="mr-2 fa-solid fa-arrow-right-from-bracket"></i>"#,
            "Logout",
            "</a>",
            "</li>",
            r#"<li class="px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center" _="on click askNotificationPermission()">"#,
            r#"<i class="mr-2 fa-regular fa-bell-slash"></i>"#,
            "Enable notifications",
            "</li>",
            "</ul>",
        );

        let actual = List.render().into_string();

        assert_eq!(expected, actual);
    }
}
