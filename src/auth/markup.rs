use maud::html;

use crate::markup::Wrappable;

pub async fn login() -> Wrappable {
    Wrappable(html! {
        div class="h-full flex justify-center items-center" {
            a class="bg-blue-500 hover:bg-blue-400 text-white font-bold py-2 px-4 border-b-4 border-blue-700 hover:border-blue-500 rounded"
                href="/sso/login" {
                    "Login with SSO"
                }
        }
    })
}
