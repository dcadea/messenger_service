pub mod markup {
    use std::convert::Infallible;

    use axum::{
        body::Body,
        response::{IntoResponse, IntoResponseParts, Response, ResponseParts},
    };
    use maud::{html, Markup, PreEscaped, Render, DOCTYPE};
    use reqwest::header::CONTENT_LENGTH;

    pub const EMPTY: PreEscaped<&'static str> = PreEscaped("");

    struct Head<'a> {
        title: &'a str,
    }

    impl Render for Head<'_> {
        fn render(&self) -> Markup {
            html! {
                head {
                    meta charset="utf-8" {}
                    title { (self.title) }
                    script src="https://unpkg.com/htmx.org@2.0.3"
                        integrity="sha384-0895/pl2MU10Hqc6jd4RvrthNlDiE9U1tWmX7WRESftEDRosgxNsQG/Ze9YMRzHq"
                        crossorigin="anonymous" {}
                    script src="https://unpkg.com/htmx-ext-ws@2.0.2/ws.js" {}
                    script src="https://unpkg.com/htmx-ext-sse@2.2.2/sse.js" {}
                    script src="https://unpkg.com/hyperscript.org@0.9.13" {}

                    script src="https://unpkg.com/@tailwindcss/browser@4" {}
                    script src="/static/scripts.js" {}

                    link rel="stylesheet" href="/static/styles.css" {}
                    link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.6.0/css/all.min.css" {}

                    meta name="htmx-config" content=r#"{"responseHandling": [{"code":".*", "swap": true}]}"# {}
                }
            }
        }
    }

    struct MainContent {
        content: Markup,
    }

    impl Render for MainContent {
        fn render(&self) -> Markup {
            html! {
                div ."main-content"
                    ."max-w-lg h-3/5 md:h-4/5 md:w-4/5 w-full h-full"
                    ."bg-white rounded-2xl p-6"
                    ."relative overflow-hidden"
                {
                    div id="errors" {}
                    (self.content)
                }
            }
        }
    }

    fn base(w: Wrappable) -> Markup {
        let body_class = "h-screen bg-black flex items-center justify-center";
        let content = MainContent { content: w.content };

        html! {
            (DOCTYPE)
            html {
                (Head { title: "AWG Messenger" })

                @if w.sse {
                    body class=(body_class) hx-ext="sse" sse-connect="/sse" {
                        (content)
                    }
                } @else {
                    body class=(body_class) {
                        (content)
                    }
                }
            }
        }
    }

    #[derive(Clone)]
    pub struct Wrappable {
        content: Markup,
        sse: bool,
    }

    impl Wrappable {
        pub fn new(content: Markup) -> Self {
            Self {
                content,
                sse: false,
            }
        }

        pub fn with_sse(mut self) -> Self {
            self.sse = true;
            self
        }
    }

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
            *resp.body_mut() = Body::new(base(w).into_string());
            return resp;
        }

        resp
    }
}

pub mod middleware {
    use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

    const X_REQUEST_ID: &str = "X-Request-Id";

    pub async fn attach_request_id(req: Request, next: Next) -> Response {
        let req_id = req
            .headers()
            .get(X_REQUEST_ID)
            .map(|id| id.to_owned())
            .or_else(|| {
                let req_id = &uuid::Uuid::now_v7().to_string();
                HeaderValue::from_str(req_id).ok()
            });

        let mut resp = next.run(req).await;
        if let Some(req_id) = req_id {
            resp.headers_mut().insert(X_REQUEST_ID, req_id);
        }
        resp
    }
}
