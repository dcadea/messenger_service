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
        id: Path<contact::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        // TODO: validate

        contact_service
            .transition_status(&id, contact::StatusTransition::Accept)
            .await?;

        Ok(())
    }

    pub async fn reject(
        id: Path<contact::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        // TODO: validate

        contact_service
            .transition_status(&id, contact::StatusTransition::Reject)
            .await?;

        Ok(())
    }

    pub async fn block(
        id: Path<contact::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        // TODO: validate

        contact_service
            .transition_status(&id, contact::StatusTransition::Block)
            .await?;

        Ok(())
    }
}
