use reqwest::{Client as ReqwestClient, header};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{types::*, crypto::Crypto};

pub struct Client {
    client: ReqwestClient,
    crypto: Arc<Mutex<Crypto>>,
    device_id: String,
    web_params: HashMap<String, String>,
    device_params: HashMap<String, String>,
}

impl Client {
    pub fn new(device_id: String) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert("Referer", "https://music.163.com".parse()?);
        headers.insert(
            "User-Agent", 
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_6) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.1.2 Safari/605.1.15"
                .parse()?
        );

        let client = ReqwestClient::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            crypto: Arc::new(Mutex::new(Crypto::new()?)),
            device_id,
            web_params: HashMap::new(),
            device_params: HashMap::new(),
        })
    }

    pub async fn request<T: Api>(&self, api: &T) -> Result<T::Output> {
        let url = self.format_url::<T>(api.path());
        let body = self.encrypt::<T>(api.path(), &api.body())?;

        let response = match T::OPERATION {
            Operation::Get => {
                self.client.get(&url)
                    .query(&api.query())
                    .send()
                    .await?
            }
            Operation::Post => {
                self.client.post(&url)
                    .query(&api.query())
                    .body(body)
                    .send()
                    .await?
            }
        };

        T::Output::parse(response, &api.input).await
    }

    fn format_url<T: Api>(&self, path: &str) -> String {
        let prefix = match T::CRYPTO {
            CryptoType::Weapi => "/weapi",
            CryptoType::Eapi => "/eapi", 
            CryptoType::None => "",
        };
        format!("https://music.163.com{}{}", prefix, path)
    }

    // ...helper methods...
}
