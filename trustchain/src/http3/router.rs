// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use http::{Method, Request, Response, StatusCode};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type Handler = Arc<
    dyn Fn(Request<Vec<u8>>) -> Pin<Box<dyn Future<Output = Response<Vec<u8>>> + Send>>
        + Send
        + Sync,
>;

pub struct Router {
    routes: HashMap<(Method, String), Handler>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    pub fn route<F, Fut>(mut self, method: Method, path: &str, handler: F) -> Self
    where
        F: Fn(Request<Vec<u8>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response<Vec<u8>>> + Send + 'static,
    {
        let handler = Arc::new(move |req: Request<Vec<u8>>| -> Pin<Box<dyn Future<Output = Response<Vec<u8>>> + Send>> {
            Box::pin(handler(req))
        });
        self.routes.insert((method, path.to_string()), handler);
        self
    }

    pub fn get<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<Vec<u8>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response<Vec<u8>>> + Send + 'static,
    {
        self.route(Method::GET, path, handler)
    }

    pub fn post<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<Vec<u8>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response<Vec<u8>>> + Send + 'static,
    {
        self.route(Method::POST, path, handler)
    }

    pub fn put<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<Vec<u8>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response<Vec<u8>>> + Send + 'static,
    {
        self.route(Method::PUT, path, handler)
    }

    pub fn delete<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<Vec<u8>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response<Vec<u8>>> + Send + 'static,
    {
        self.route(Method::DELETE, path, handler)
    }

    pub fn options<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request<Vec<u8>>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response<Vec<u8>>> + Send + 'static,
    {
        self.route(Method::OPTIONS, path, handler)
    }

    pub async fn handle(&self, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
        let method = request.method().clone();
        let path = request.uri().path().to_string();

        // Try exact match first
        if let Some(handler) = self.routes.get(&(method.clone(), path.clone())) {
            return handler(request).await;
        }

        // Check for wildcard OPTIONS route for CORS preflight
        if method == Method::OPTIONS {
            if let Some(handler) = self.routes.get(&(Method::OPTIONS, "/*".to_string())) {
                return handler(request).await;
            }
        }

        // Try pattern matching for paths with parameters (e.g., /certificates/{id})
        for ((route_method, route_path), handler) in &self.routes {
            if *route_method == method && Self::path_matches(&path, route_path) {
                return handler(request).await;
            }
        }

        // Return 404 if no route matches
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(b"Not Found".to_vec())
            .expect("valid response builder")
    }

    fn path_matches(request_path: &str, route_pattern: &str) -> bool {
        let request_segments: Vec<&str> = request_path.split('/').collect();
        let pattern_segments: Vec<&str> = route_pattern.split('/').collect();

        if request_segments.len() != pattern_segments.len() {
            return false;
        }

        for (req_seg, pat_seg) in request_segments.iter().zip(pattern_segments.iter()) {
            if pat_seg.starts_with('{') && pat_seg.ends_with('}') {
                // This is a path parameter, it matches any value
                continue;
            }
            if req_seg != pat_seg {
                return false;
            }
        }

        true
    }

    pub fn extract_path_param(path: &str, pattern: &str, param_name: &str) -> Option<String> {
        let path_segments: Vec<&str> = path.split('/').collect();
        let pattern_segments: Vec<&str> = pattern.split('/').collect();

        for (i, seg) in pattern_segments.iter().enumerate() {
            if seg.starts_with('{') && seg.ends_with('}') {
                let param = &seg[1..seg.len() - 1];
                if param == param_name && i < path_segments.len() {
                    return Some(path_segments[i].to_string());
                }
            }
        }
        None
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}