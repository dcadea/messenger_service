use maud::{Render, html};

use crate::user::model::UserInfo;

use super::model::ContactDto;

pub struct ContactInfos<'a>(pub &'a [(ContactDto, UserInfo)]);

impl Render for ContactInfos<'_> {
    fn render(&self) -> maud::Markup {
        html! {
            header ."text-center mb-4"{
                h2.text-2xl { "Contacts" }
            }
            ul ."flex flex-col" {
                @for (c, ui) in self.0 {
                    li ."flex items-center mb-3" {
                        img ."w-9 h-9 rounded-full float-left mr-2"
                            src=(ui.picture)
                            alt="User avatar" {}
                        (ui.name)

                        (c.status)
                    }
                }
            }
        }
    }
}
