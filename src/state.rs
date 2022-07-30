//! A Redis database backed state. Redis service is provided by Upstash with a RESTful API.

use serde::Deserialize;
use wasm_bindgen::JsValue;
use worker::{console_debug, Fetch, Headers, Method, Request, RequestInit, Url};

/// A channel implemented based on Redis List data structure.
#[derive(Debug, Clone)]
pub(crate) struct State {
    /// Url to Upstash Redis endpoint.
    url: Url,

    /// Authentication token to Upstash Redis endpoint.
    headers: Headers,
}

impl State {
    /// Creates a new state.
    pub(crate) fn new(url: &str, token: &str) -> State {
        let url = Url::parse(url).expect("invalid url");

        let mut headers = Headers::new();
        headers
            .set("Authorization", &format!("Bearer {}", token))
            .unwrap();

        State { url, headers }
    }

    /// Executes a specialized set command, the whole command is `set key "" nx`. Only the key matters.
    /// The key should be prefixed with "passphrase" in order to avoid key name collision in Redis.
    /// Returns "OK" if value not exists else Null.
    pub(crate) async fn set_nx(&self, key: &str) -> Response {
        let cmd = ["set", key, "", "nx"];
        self.command(&cmd).await
    }

    /// Mimics a channel send, it's actually a right push on an underlying list.
    /// The key should be prefixed with "channel" in order to avoid key name collision in Redis.
    pub(crate) async fn send(&self, key: &str, element: &str) -> Response {
        let cmd = ["lpush", key, element];
        self.command(&cmd).await
    }

    /// Mimics a channel receive, it's actually a left pop on an underlying list.
    /// The key should be prefixed with "channel" in order to avoid key name collision in Redis.
    pub(crate) async fn receive(&self, key: &str) -> Response {
        let cmd = ["rpop", key];
        self.command(&cmd).await
    }

    /// Deletes all keys relating to the party in a session.
    pub(crate) async fn del_keys(&self, keys: &[&str]) -> Response {
        let cmd = [&["del"], keys].concat();
        self.command(&cmd).await
    }

    /// Executes any command on Redis.
    async fn command(&self, command: &[&str]) -> Response {
        let body = serde_json::to_string(&command).unwrap();
        console_debug!("command: {}", body);

        let mut request_init = RequestInit::new();
        request_init
            .with_method(Method::Post)
            .with_headers(self.headers.clone())
            .with_body(Some(JsValue::from_str(&body)));

        let request = Request::new_with_init(self.url.as_str(), &request_init).unwrap();
        let mut response = Fetch::Request(request).send().await.unwrap();
        if !response.status_code().eq(&200) {
            panic!(
                "request not successful, status code: {}",
                response.status_code()
            )
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
    Str(String),
    Int(u32),
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

        let int_result = r#"{"result": 1}"#;
        serde_json::from_str::<Response>(int_result).unwrap();
    }
}
