use maud::{Render, html};

use crate::{auth, contact::Status, user::model::UserInfo};

use super::model::ContactDto;

pub struct ContactInfos<'a> {
    pub auth_user: &'a auth::User,
    pub contact_infos: &'a [(ContactDto, UserInfo)],
}

impl<'a> ContactInfos<'a> {
    pub fn new(auth_user: &'a auth::User, contact_infos: &'a [(ContactDto, UserInfo)]) -> Self {
        Self {
            auth_user,
            contact_infos,
        }
    }
}

const CONTACT_ITEM_CLASS: &str =
    "px-3 py-2 rounded-md bg-gray-100 hover:bg-gray-200 cursor-pointer flex items-center";

impl Render for ContactInfos<'_> {
    fn render(&self) -> maud::Markup {
        let auth_sub = &self.auth_user.sub;

        html! {
            header ."text-center mb-4"{
                h2.text-2xl { "Contacts" }
            }
            ul ."flex flex-col space-y-2" {
                @for (c, ui) in self.contact_infos {
                    li .(CONTACT_ITEM_CLASS) {
                        img ."w-9 h-9 rounded-full float-left mr-2"
                            src=(ui.picture)
                            alt="User avatar" {}
                        (ui.name)

                        @match &c.status {
                            Status::Pending { initiator } => {
                                @if initiator.eq(auth_sub) {
                                    ."grow text-right text-blue-500" {
                                        i ."fa-solid fa-hourglass-half mr-2" {}
                                        "Pending action"
                                    }
                                } @else {
                                    ."grow text-right text-2xl" {
                                        i ."fa-solid fa-check text-green-500 cursor-pointer"
                                            hx-swap="none" // TODO: remove icons after accept
                                            hx-put={"/api/contacts/" (c.id) "/accept"} {}
                                        i ."fa-solid fa-xmark ml-3 text-red-500 cursor-pointer"
                                            hx-swap="none" // TODO: remove icons after reject
                                            hx-put={"/api/contacts/" (c.id) "/reject"} {}
                                    }
                                }
                            },
                            Status::Accepted => {
                                ."grow text-right text-2xl" {
                                    i ."fa-solid fa-ban ml-3 cursor-pointer"
                                        hx-swap="none" // TODO: remove icon after block
                                        hx-put={"/api/contacts/" (c.id) "/block"} {}
                                }
                            },
                            Status::Rejected => {
                                ."grow text-right text-red-500" {
                                    i ."fa-solid fa-xmark mr-2" {}
                                    "Request rejected"
                                }
                            },
                            Status::Blocked { initiator } => {
                                ."grow text-right text-blue-500" {
                                    i ."fa-solid fa-ban mr-2" {}
                                    @if initiator.eq(auth_sub) {
                                        "Blocked"
                                    } @else {
                                        "Blocked you"
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}
