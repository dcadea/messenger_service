use maud::{Render, html};

use crate::{contact::Status, user::model::UserInfo};

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


                        @if Status::Accepted.ne(&c.status) {
                            .contact-controls ."grow text-right text-2xl"  {
                                i ."fa-solid fa-check text-green-500 cursor-pointer"
                                    hx-put={"/api/contacts/" (c.id) "/accept"} {}
                                i ."fa-solid fa-xmark ml-3 text-red-500 cursor-pointer"
                                    hx-put={"/api/contacts/" (c.id) "/reject"} {}
                                i ."fa-solid fa-ban ml-3 cursor-pointer"
                                    hx-put={"/api/contacts/" (c.id) "/block"} {}
                            }
                        }
                    }
                }
            }
        }
    }
}
