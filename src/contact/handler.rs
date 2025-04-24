pub(super) mod api {

    use axum::{
        Extension, Form,
        extract::{Path, Query, State},
    };
    use maud::{Markup, Render};
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
        auth_user: Extension<auth::User>,
        contact_service: State<contact::Service>,
        params: Form<CreateParams>,
    ) -> crate::Result<Markup> {
        let c = Contact::new(&auth_user.sub, &params.sub);
        contact_service.add(&c).await?;
        Ok(c.status().render())
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

    pub async fn unblock(
        Extension(auth_user): Extension<auth::User>,
        Path(id): Path<contact::Id>,
        contact_service: State<contact::Service>,
    ) -> crate::Result<()> {
        let c = contact_service.find_by_id(&auth_user.sub, &id).await?;
        let target = c.recipient;
        contact_service
            .transition_status(&id, contact::StatusTransition::Unblock { target })
            .await?;

        Ok(())
    }
}
