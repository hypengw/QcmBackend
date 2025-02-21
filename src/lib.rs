pub mod service {
    pub mod ncm;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ncm::{Client, api_old::*};

    #[tokio::test]
    async fn test_ncm_client() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new("test-device-id".to_string())?;
        
        // Test API call
        let api = cloud::CloudPubApi {
            input: cloud::CloudPubParams {
                song_id: "12345".to_string()
            }
        };
        
        let result = client.request(&api).await?;
        assert_eq!(result.code, 200);
        
        Ok(())
    }
}
