//! A Redis database backed stateful channel. Redis service is provided by Upstash with a RESTful API.
//! A channel has `send` and `receive` methods, it's simply implemented by a Redis List data structure.

// use serde::Deserialize;
use serde::Deserialize;
use worker::{wasm_bindgen::JsValue, Fetch, Headers, Method, Request, RequestInit, Url};

/// A channel implemented based on Redis List data structure.
#[derive(Debug)]
pub(crate) struct Channel {
    /// Url to Upstash Redis endpoint.
    url: Url,

    /// Authentication token to Upstash Redis endpoint.
    headers: Headers,
}

impl Channel {
    pub(crate) fn new(url: &str, token: &str) -> Channel {
        let mut url = Url::parse(url).expect("invalid url");
        url.set_query(Some(&format!("_token={}", token)));

        let mut headers = Headers::new();
        headers
            .set("Authorization", &format!("Bearer {}", token))
            .unwrap();

        Channel { url, headers }
    }

    pub(crate) async fn send(&self, key: &str, element: &str) {
        let command = ["lpush", key, element];
        let body = serde_json::to_string(&command).unwrap();

        let mut request_init = RequestInit::new();
        request_init
            .with_method(Method::Post)
            .with_headers(self.headers.clone())
            // Probable worker package bug, can't use `JsValue::from_serde`, or runtime error will occur.
            .with_body(Some(JsValue::from_str(&body)));

        let request = Request::new_with_init(self.url.as_str(), &request_init).unwrap();
        let response = Fetch::Request(request).send().await.unwrap();
        if !response.status_code().eq(&200) {
            panic!("request not successful")
        }
    }

    pub(crate) async fn receive(&self, key: &str) -> Response {
        let command = ["rpop", key];
        let body = serde_json::to_string(&command).unwrap();

        let mut request_init = RequestInit::new();
        request_init
            .with_method(Method::Post)
            .with_headers(self.headers.clone())
            .with_body(Some(JsValue::from_str(&body)));

        let request = Request::new_with_init(self.url.as_str(), &request_init).unwrap();
        let mut response = Fetch::Request(request).send().await.unwrap();
        if !response.status_code().eq(&200) {
            panic!("request not successful")
        }

        response.json().await.unwrap()
    }
}

/// Response returned from Upstash Redis api.
#[derive(Debug, Deserialize)]
pub(crate) enum Response {
    #[serde(rename = "result")]
    Result(Result),
    #[serde(rename = "error")]
    Error(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Result {
    Null,
    Value(String),
}

#[cfg(test)]
mod tests {
    use super::Response;

    #[test]
    fn deserialize_response() {
        let null_result = r#"{"result": null}"#;
        serde_json::from_str::<Response>(null_result).unwrap();

        let some_result = r#"{"result": "value"}"#;
        serde_json::from_str::<Response>(some_result).unwrap();

        let error_result = r#"{"error": "some error"}"#;
        serde_json::from_str::<Response>(error_result).unwrap();
    }
}
