pub use bytes::Bytes;
use cookie_store::Cookie;
use futures::StreamExt;
use futures::{future::BoxFuture, prelude::Stream, stream::FuturesUnordered, FutureExt};
use openssl::sha;
pub use reqwest::cookie::CookieStore as CookieStoreTrait;
pub use reqwest::header::{HeaderMap, HeaderValue};
pub use reqwest::Client as HttpClient;
pub use reqwest::ClientBuilder as HttpClientBuilder;
pub use reqwest_cookie_store::{CookieStore, CookieStoreRwLock};
use serde::Deserialize;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

use crate::provider::ProviderSession;
use log;
use std::future::Future;
use std::io::Cursor;
use std::ops::Deref;
use std::sync::atomic::{AtomicIsize, AtomicUsize};
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
    match cookie_store::serde::json::load_all(Cursor::new(data)) {
        Ok(loaded) => {
            let mut jar = jar.write().unwrap();
            jar.clear();
            *jar = loaded;
        }
        Err(e) => {
            log::error!("Failed to parse cookie data: {}", e);
        }
    }
}
fn save_(jar: Arc<CookieStoreRwLock>) -> String {
    let jar = jar.read().unwrap();
    let mut cursor = Cursor::new(Vec::new());
    cookie_store::serde::json::save_incl_expired_and_nonpersistent(&jar, &mut cursor)
        .expect("Failed to save cookies to string");

    String::from_utf8(cursor.into_inner()).unwrap_or_default()
}

impl<T: HasCookieJar> ProviderSession for T {
    fn load_cookie(&self, data: &str) {
        load_(self.jar(), data);
    }

    fn save_cookie(&self) -> String {
        save_(self.jar())
    }
}

pub fn client_builder_with_jar(jar: Arc<CookieStoreRwLock>) -> HttpClientBuilder {
    HttpClientBuilder::new().cookie_provider(jar)
}

enum BatchResponseMsg {
    Add(reqwest::Request, reqwest::Client),
    AddRsp(reqwest::Response),
    Wait(oneshot::Sender<Option<Result<Bytes, reqwest::Error>>>),
    Count(oneshot::Sender<usize>),
}
pub struct BatchRequest {
    tx: Sender<BatchResponseMsg>,
}

impl BatchRequest {
    pub fn new() -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<BatchResponseMsg>(64);

        tokio::spawn({
            async move {
                let mut futures = FuturesUnordered::new();

                let new_future = async move |msg: BatchResponseMsg| match msg {
                    BatchResponseMsg::Add(req, client) => {
                        let rsp = client.execute(req).await?;
                        rsp.bytes().await
                    }
                    BatchResponseMsg::AddRsp(rsp) => rsp.bytes().await,
                    _ => unreachable!(),
                };

                loop {
                    match rx.recv().await {
                        Some(BatchResponseMsg::Wait(tx)) => {
                            if let Err(_) = tx.send(futures.next().await) {
                                log::error!("Then recv droped");
                            }
                        }
                        Some(BatchResponseMsg::Count(tx)) => {
                            if let Err(_) = tx.send(futures.len()) {
                                log::error!("Then recv droped");
                            }
                        }
                        Some(msg) => {
                            futures.push(new_future(msg));
                        }
                        None => {
                            return;
                        }
                    }
                }
            }
        });
        Self { tx: tx }
    }

    async fn send_with_count(&self, msg: BatchResponseMsg) -> Option<usize> {
        let _ = self.tx.send(msg).await;
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(BatchResponseMsg::Count(tx)).await;
        rx.await.ok()
    }

    pub async fn add(&self, req: reqwest::Request, client: reqwest::Client) -> Option<usize> {
        self.send_with_count(BatchResponseMsg::Add(req, client))
            .await
    }

    pub async fn add_rsp(&self, rsp: reqwest::Response) -> Option<usize> {
        let msg = BatchResponseMsg::AddRsp(rsp);
        self.send_with_count(msg).await
    }

    pub async fn wait_one(&self) -> Option<Result<Bytes, reqwest::Error>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(BatchResponseMsg::Wait(tx)).await;
        match rx.await {
            Ok(rsp) => rsp,
            Err(_) => {
                log::error!("Failed to receive response");
                None
            }
        }
    }
}
