pub mod middleware {
    use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

    const X_REQUEST_ID: &str = "X-Request-Id";

    pub async fn attach_request_id(req: Request, next: Next) -> Response {
        let req_id = req
            .headers()
            .get(X_REQUEST_ID)
            .map(ToOwned::to_owned)
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
