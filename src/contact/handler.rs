pub(super) mod api {

    use axum::{
        Extension, Form,
        extract::{Path, Query, State},
    };
    use serde::Deserialize;

    use crate::{
        auth,
        contact::{self, model::Contact},
        user,
    };

    #[derive(Deserialize)]
    pub struct CreateParams {
        sub: user::Sub,
    }

    pub async fn create(
        Extension(auth_user): Extension<auth::User>,
        contact_service: State<contact::Service>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<()> {
        contact_service
            .add(&Contact::new(auth_user.sub, params.sub))
            .await?;
        Ok(())
    }

    pub async fn delete(
        auth_user: Extension<auth::User>,
        sub: Query<user::Sub>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        contact_service.delete(&auth_user.sub, &sub).await?;
        Ok(())
    }

    pub async fn accept(
        Extension(auth_user): Extension<auth::User>,
        id: Path<contact::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        let responder = auth_user.sub;

        contact_service
            .transition_status(&id, contact::StatusTransition::Accept { responder })
            .await?;

        Ok(())
    }

    pub async fn reject(
        Extension(auth_user): Extension<auth::User>,
        id: Path<contact::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        let responder = auth_user.sub;

        contact_service
            .transition_status(&id, contact::StatusTransition::Reject { responder })
            .await?;

        Ok(())
    }

    pub async fn block(
        Extension(auth_user): Extension<auth::User>,
        id: Path<contact::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        let initiator = auth_user.sub;

        contact_service
            .transition_status(&id, contact::StatusTransition::Block { initiator })
            .await?;

        Ok(())
    }
}
