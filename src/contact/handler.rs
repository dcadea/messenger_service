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
        contact::{self, StatusTransition, Transition},
        user,
    };

    #[derive(Deserialize)]
    pub struct CreateParams {
        user_id: user::Id,
    }

    pub async fn create(
        auth_user: Extension<auth::User>,
        contact_service: State<contact::Service>,
        params: Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let s = contact_service.add(auth_user.id(), &params.user_id)?;
        Ok(s.render())
    }

    pub async fn delete(
        auth_user: Extension<auth::User>,
        user_id: Query<user::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        contact_service.delete(auth_user.id(), &user_id)?;
        Ok(())
    }

    pub async fn transition(
        Extension(auth_user): Extension<auth::User>,
        Path((id, transition)): Path<(contact::Id, Transition)>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<Markup> {
        let auth_id = auth_user.id();
        let st = match transition {
            Transition::Accept => StatusTransition::Accept { responder: auth_id },
            Transition::Reject => StatusTransition::Reject { responder: auth_id },
            Transition::Block => StatusTransition::Block { initiator: auth_id },
            Transition::Unblock => {
                let c = contact_service.find_by_id(auth_id, &id).await?;

                StatusTransition::Unblock {
                    target: &c.recipient().clone(),
                }
            }
        };

        let new_status = contact_service
            .transition_status(auth_user.id(), &id, st)
            .await?;

        Ok(contact::markup::Icons::new(&id, &new_status, &auth_user).render())
    }

    impl From<contact::Error> for StatusCode {
        fn from(e: contact::Error) -> Self {
            match e {
                contact::Error::NotFound(_) => Self::NOT_FOUND,
                contact::Error::AlreadyExists => Self::CONFLICT,
                contact::Error::SameUsers(_) | contact::Error::StatusTransitionFailed => {
                    Self::BAD_REQUEST
                }
                contact::Error::_R2d2(_) | contact::Error::_Diesel(_) => {
                    Self::INTERNAL_SERVER_ERROR
                }
            }
        }
    }
}
