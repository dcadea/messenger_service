use maud::{Render, html};

use crate::user::model::UserInfo;

pub struct ContactInfos<'a>(pub &'a [UserInfo]);

impl Render for ContactInfos<'_> {
    fn render(&self) -> maud::Markup {
        html! {
            ul {
                @for u in self.0 {
                    li {
                        img ."w-8 h-8 rounded-full float-left"
                            src=(u.picture)
                            alt="User avatar" {}
                        (u.name)
                    }
                }
            }
        }
    }
}
