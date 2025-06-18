use crate::models::Password;
use http::header::{AUTHORIZATION, CONTENT_TYPE};

pub struct MoneyBird {
    client: reqwest::Client,
}

impl MoneyBird {
    pub fn new(api_key: Password) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", api_key.danger_as_str())
                .parse()
                .unwrap(),
        );
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client }
    }
    
    pub fn create_sales_link(&self) {
        
    }
}
