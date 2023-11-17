use super::enroll::enroll_endpoint;
use crate::server::ServerState;
use axum::{routing::post, Router};

/// Setup `Enrollment` routes
pub(crate) fn enroll_routes(base: &str) -> Router<ServerState> {
    Router::new().route(&format!("{base}/enroll"), post(enroll_endpoint))
}

#[cfg(test)]
mod tests {
    use super::enroll_routes;
    use crate::{server::ServerState, utils::config::read_config};
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use std::{collections::HashMap, path::PathBuf, sync::Arc};
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_enroll_routes() {
        let base = "/endpoint/v1";
        let route = enroll_routes(base);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };

        let res = route
            .with_state(server_state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("{base}/enroll"))
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}
