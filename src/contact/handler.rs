pub(super) mod api {

    use axum::{
        Extension, Form,
        extract::{Query, State},
    };
    use serde::Deserialize;

    use crate::{
        contact::{self, model::Contact},
        user::{self, model::UserInfo},
    };

    #[derive(Deserialize)]
    pub struct CreateParams {
        sub: user::Sub,
    }

    #[axum::debug_handler]
    pub async fn create(
        Extension(logged_user): Extension<UserInfo>,
        contact_service: State<contact::Service>,
        Form(params): Form<CreateParams>,
    ) -> crate::Result<()> {
        contact_service
            .add(&Contact::new(logged_user.sub, params.sub))
            .await?;
        Ok(())
    }

    pub async fn delete(
        logged_user: Extension<UserInfo>,
        sub: Query<user::Sub>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        contact_service.delete(&logged_user.sub, &sub).await?;
        Ok(())
    }
}
