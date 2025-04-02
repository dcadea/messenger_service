pub(super) mod api {

    use axum::{
        Extension, Form,
        extract::{Query, State},
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

    #[axum::debug_handler]
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
}
