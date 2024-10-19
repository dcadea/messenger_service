pub mod serde {
    use mongodb::bson::oid::ObjectId;
    use serde::Serializer;

    pub fn serialize_object_id<S>(
        message_id: &Option<ObjectId>,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match message_id {
            Some(ref message_id) => serializer.serialize_some(message_id.to_hex().as_str()),
            None => serializer.serialize_none(),
        }
    }
}

pub mod markup {
    use std::convert::Infallible;

    use axum::{
        body::Body,
        response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
    };
    use maud::{html, Markup, DOCTYPE};
    use reqwest::header::CONTENT_LENGTH;

    fn base(content: Markup) -> Markup {
        html! {
            (DOCTYPE)
            html {
                head {
                    meta charset="utf-8" {}
                    title { "AWG Messenger" }
                    script src="https://unpkg.com/htmx.org@2.0.3"
                        integrity="sha384-0895/pl2MU10Hqc6jd4RvrthNlDiE9U1tWmX7WRESftEDRosgxNsQG/Ze9YMRzHq"
                        crossorigin="anonymous" {}
                    script src="https://unpkg.com/htmx.org@2.0.3/dist/ext/ws.js" {}
                    script src="https://cdn.tailwindcss.com" {}
                    link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css" {}
                }
                body."h-screen bg-black flex items-center justify-center" {
                    ."max-w-lg h-3/5 md:h-4/5 md:w-4/5 bg-white rounded-2xl p-6" {
                        (content)
                    }
                }
            }
        }
    }

    #[derive(Clone)]
    pub struct Wrappable(pub Markup);

    impl IntoResponseParts for Wrappable {
        type Error = Infallible;

        fn into_response_parts(
            self,
            mut res: ResponseParts,
        ) -> core::result::Result<ResponseParts, Self::Error> {
            res.extensions_mut().insert(self);
            Ok(res)
        }
    }

    impl IntoResponse for Wrappable {
        fn into_response(self) -> axum::response::Response {
            (self, ()).into_response()
        }
    }

    pub async fn wrap_in_base(mut resp: Response) -> impl IntoResponse {
        if let Some(w) = resp.extensions_mut().remove::<Wrappable>() {
            resp.headers_mut().remove(CONTENT_LENGTH);
            *resp.body_mut() = Body::new(base(w.0).into_string());
            return resp;
        }

        resp
    }
}
