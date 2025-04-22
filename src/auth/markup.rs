use maud::{Render, html};

pub struct Login;

impl Render for Login {
    fn render(&self) -> maud::Markup {
        html! {
            div ."h-full flex justify-center items-center" {
                a ."bg-blue-500 hover:bg-blue-400 text-white font-bold py-2 px-4 border-b-4 border-blue-700 hover:border-blue-500 rounded"
                    href="/sso/login"
                {
                    "Login with SSO"
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use maud::Render;

    use super::Login;

    #[test]
    fn should_render_login() {
        let expected = concat!(
            r#"<div class="h-full flex justify-center items-center">"#,
            r#"<a class="bg-blue-500 hover:bg-blue-400 text-white font-bold py-2 px-4 border-b-4 border-blue-700 hover:border-blue-500 rounded" href="/sso/login">"#,
            "Login with SSO",
            "</a>",
            "</div>"
        );

        let actual = Login.render().into_string();

        assert_eq!(expected, actual);
    }
}
