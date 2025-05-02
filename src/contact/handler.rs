pub(super) mod api {

    use axum::{
        Extension, Form,
        extract::{Path, Query, State},
        http::StatusCode,
    };
    use maud::{Markup, Render};
    use serde::Deserialize;

    use crate::{
        auth,
        contact::{self, Transition, model::Contact},
        user,
    };

    #[derive(Deserialize)]
    pub struct CreateParams {
        sub: user::Sub,
    }

    pub async fn create(
        auth_user: Extension<auth::User>,
        contact_service: State<contact::Service>,
        params: Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let c = Contact::new(auth_user.sub(), &params.sub);
        contact_service.add(&c).await?;
        Ok(c.status().render())
    }

    pub async fn delete(
        auth_user: Extension<auth::User>,
        sub: Query<user::Sub>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        contact_service.delete(auth_user.sub(), &sub).await?;
        Ok(())
    }

    pub async fn transition(
        Extension(auth_user): Extension<auth::User>,
        Path((id, transition)): Path<(contact::Id, Transition)>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<Markup> {
        let st = match transition {
            Transition::Accept => contact::StatusTransition::Accept {
                responder: auth_user.sub(),
            },
            Transition::Reject => contact::StatusTransition::Reject {
                responder: auth_user.sub(),
            },
            Transition::Block => contact::StatusTransition::Block {
                initiator: auth_user.sub(),
            },
            Transition::Unblock => {
                let c = contact_service.find_by_id(auth_user.sub(), &id).await?;

                contact::StatusTransition::Unblock {
                    target: &c.recipient().clone(),
                }
            }
        };

        let new_status = contact_service.transition_status(&id, st).await?;

        Ok(contact::markup::Icons::new(&id, &new_status, &auth_user).render())
    }

    impl From<contact::Error> for StatusCode {
        fn from(e: contact::Error) -> Self {
            match e {
                contact::Error::NotFound(_) => StatusCode::NOT_FOUND,
                contact::Error::AlreadyExists(..) => StatusCode::CONFLICT,
                contact::Error::SameSubs(_) | contact::Error::StatusTransitionFailed => {
                    StatusCode::BAD_REQUEST
                }
                contact::Error::_MongoDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
    }
}
