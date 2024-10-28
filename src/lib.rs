pub mod markup {
    use std::convert::Infallible;

    use axum::{
        body::Body,
        response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
    };
    use maud::{html, Markup, PreEscaped, DOCTYPE};
    use reqwest::header::CONTENT_LENGTH;

    pub const EMPTY: PreEscaped<&'static str> = PreEscaped("");

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
                    script src="https://unpkg.com/hyperscript.org@0.9.13" {}

                    script src="https://cdn.tailwindcss.com" {}
                    link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css" {}
                }
                body class="h-screen bg-black flex items-center justify-center"
                    hx-ext="ws"
                    ws-connect="/ws"
                {
                    div class="max-w-lg h-3/5 md:h-4/5 md:w-4/5 bg-white rounded-2xl p-6"
                    {
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
