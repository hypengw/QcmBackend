pub use reqwest::header::{HeaderMap, HeaderValue};
pub use reqwest::Client as HttpClient;
pub use reqwest::ClientBuilder as HttpClientBuilder;
pub use reqwest_cookie_store::{CookieStore, CookieStoreRwLock};

use crate::provider::ProviderSession;
use log;
use std::io::Cursor;
use std::sync::Arc;

fn wrap_iter<'a, T, I>(iter: I) -> impl Iterator<Item = Result<T, i32>> + 'a
where
    I: Iterator<Item = &'a T> + 'a,
    T: Clone + 'a,
{
    iter.map(|item| Ok(item.clone()))
}

pub trait HasCookieJar {
    fn jar(&self) -> Arc<CookieStoreRwLock>;
}

fn load_(jar: Arc<CookieStoreRwLock>, data: &str) {
    let mut cursor = Cursor::new(data);
    match CookieStore::load_all(&mut cursor, |cookies| serde_json::from_str(cookies)) {
        Ok(loaded) => {
            let mut jar = jar.write().unwrap();
            jar.clear();
            if let Ok(loaded) = CookieStore::from_cookies(wrap_iter(loaded.iter_any()), true) {
                *jar = loaded;
            }
        }
        Err(err) => {
            log::error!("{}", err);
        }
    }
}
fn save_(jar: Arc<CookieStoreRwLock>) -> String {
    let jar = jar.read().unwrap();

    let mut cursor = Cursor::new(Vec::new());
    jar.save_incl_expired_and_nonpersistent(&mut cursor, ::serde_json::to_string_pretty)
        .expect("Failed to save cookies to string");

    String::from_utf8(cursor.into_inner()).unwrap_or_default()
}

impl<T: HasCookieJar> ProviderSession for T {
    fn load(&self, data: &str) {
        load_(self.jar(), data);
    }

    fn save(&self) -> String {
        save_(self.jar())
    }
}

pub fn client_builder_with_jar(jar: Arc<CookieStoreRwLock>) -> HttpClientBuilder {
    HttpClientBuilder::new().cookie_provider(jar)
}
