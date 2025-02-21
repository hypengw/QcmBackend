use crate::service::ncm::types::*;
use serde::{Deserialize, Serialize};

pub struct Login {
    pub username: String,
    pub password_md5: String,
}

#[derive(Deserialize)]
pub struct LoginResponse {
    pub code: i64,
}

impl ApiInput for Login {}

impl ApiModel for LoginResponse {
    fn parse(response: reqwest::Response, _input: &impl ApiInput) -> Result<Self> {
        response.json().map_err(Error::from)
    }
}

impl Api for Login {
    type Input = Self;
    type Output = LoginResponse;
    
    const OPERATION: Operation = Operation::Post;
    const CRYPTO: CryptoType = CryptoType::Weapi;
    
    fn path(&self) -> String {
        "/login".into()
    }

    fn query(&self) -> Params {
        Params::new()
    }
    
    fn body(&self) -> Params {
        let mut params = Params::new();
        params.insert("username".to_string(), self.username.clone());
        params.insert("password".to_string(), self.password_md5.clone());
        params.insert("rememberLogin".to_string(), "true".to_string());
        params
    }
}
