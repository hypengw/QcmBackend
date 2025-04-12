use crate::event::{BackendContext, BackendEvent, Event};
use qcm_core::event::Event as CoreEvent;
use qcm_core::event::SyncCommit;
use qcm_core::model as sqlm;
use qcm_core::provider::Provider;
use qcm_core::{self, provider};
use qcm_core::{global, Result};
use sea_orm::{EntityTrait, QueryFilter, QuerySelect};
use std::collections::BTreeMap;
use std::os::linux::raw::stat;
use std::sync::Arc;
use tokio::sync::oneshot;

use crate::convert::*;
use crate::msg::{
    self, model::ProviderMeta, model::ProviderStatus, ProviderMetaStatusMsg, ProviderStatusMsg,
    QcmMessage,
};

pub struct ProcessContext {
    sync_status: BTreeMap<i64, msg::model::ProviderSyncStatus>,
}

impl ProcessContext {
    pub fn new() -> Self {
        Self {
            sync_status: BTreeMap::new(),
        }
    }
}

pub async fn process_event(ev: Event, ctx: Arc<BackendContext>) -> Result<bool> {
    match ev {
        Event::ProviderSync { id, oneshot } => {
            if let Some(p) = global::provider(id) {
                ctx.oper.spawn({
                    let ctx = ctx.provider_context.clone();
                    log::warn!("spawn sync");
                    |tid| async move {
                        log::warn!("start sync");
                        if let Some(tx) = oneshot {
                            let _ = tx.send(tid);
                        }
                        let id = p.id().unwrap();
                        let _ = ctx.ev_sender.try_send(CoreEvent::SyncCommit {
                            id,
                            commit: SyncCommit::Start,
                        });

                        match p.sync(&ctx).await {
                            Err(err) => {
                                log::error!("{:?}", err);
                            }
                            _ => {}
                        }
                        log::warn!("sync end");
                        let _ = ctx.ev_sender.try_send(CoreEvent::SyncCommit {
                            id,
                            commit: SyncCommit::End,
                        });
                    }
                });
            }
        }
        Event::SyncCommit { id, commit } => {
            let _ = ctx
                .bk_ev_sender
                .try_send(BackendEvent::SyncCommit { id, commit });
        }
        Event::End => return Ok(true),
    }
    return Ok(false);
}

pub async fn process_backend_event(
    ev: BackendEvent,
    ctx: Arc<BackendContext>,
    pctx: &mut ProcessContext,
) -> Result<bool> {
    let ev_sender = &ctx.provider_context.ev_sender;
    match ev {
        BackendEvent::Frist => {
            send_provider_meta_status(ctx.as_ref()).await?;
            let _ = send_provider_status(ctx.as_ref(), &pctx.sync_status).await?;
            // for p in providers {
            //     if let Some(id) = p.id() {
            //         ev_sender.send(Event::ProviderSync { id: id }).await?;
            //     }
            // }
        }
        BackendEvent::NewProvider { id } => {
            send_provider_status(ctx.as_ref(), &pctx.sync_status).await?;
            let (tx, rx) = oneshot::channel::<i64>();
            ev_sender
                .send(Event::ProviderSync {
                    id,
                    oneshot: Some(tx),
                })
                .await?;
            let sync_status = pctx.sync_status.clone();
            tokio::spawn(async move {
                if let Ok(id) = rx.await {
                    ctx.oper.wait(id).await;
                    let _ = send_provider_status(ctx.as_ref(), &sync_status).await;
                }
            });
        }
        BackendEvent::SyncCommit { id, commit } => {
            let status = {
                match pctx.sync_status.get_mut(&id) {
                    Some(v) => {
                        merge_sync_status(v, commit);
                        v.clone()
                    }
                    None => {
                        let mut p = msg::model::ProviderSyncStatus::default();
                        merge_sync_status(&mut p, commit);
                        p.id = id;
                        pctx.sync_status.insert(id, p);
                        p
                    }
                }
            };
            let msg: QcmMessage = msg::ProviderSyncStatusMsg {
                status: Some(status),
                statuses: Vec::new(),
            }
            .qcm_into();
            ctx.ws_sender.send(msg.qcm_try_into()?).await?;
        }
        BackendEvent::End => return Ok(true),
    }
    return Ok(false);
}

fn merge_sync_status(v: &mut msg::model::ProviderSyncStatus, m: SyncCommit) {
    match m {
        SyncCommit::Start => {
            let id = v.id;
            *v = msg::model::ProviderSyncStatus::default();
            v.id = id;
            v.state = msg::model::SyncState::Syncing as i32;
        }
        SyncCommit::End => {
            v.state = msg::model::SyncState::Finished as i32;
        }
        SyncCommit::AddAlbum(n) => {
            v.album += n;
        }
        SyncCommit::AddArtist(n) => {
            v.artist += n;
        }
        SyncCommit::AddSong(n) => {
            v.song += n;
        }
    }
}

async fn send_provider_meta_status(ctx: &BackendContext) -> Result<()> {
    let msg: Option<QcmMessage> = {
        let mut msg = ProviderMetaStatusMsg::default();
        global::with_provider_metas(|metas| {
            for (_, v) in metas {
                msg.metas.push(v.clone().qcm_into());
            }
        });
        msg.full = true;
        if msg.metas.len() > 0 {
            Some(msg.qcm_into())
        } else {
            None
        }
    };
    if let Some(msg) = msg {
        ctx.ws_sender.send(msg.qcm_try_into()?).await?;
    }
    Ok(())
}

async fn send_provider_status(
    ctx: &BackendContext,
    sync_status: &BTreeMap<i64, msg::model::ProviderSyncStatus>,
) -> Result<Vec<Arc<dyn Provider>>> {
    let db = &ctx.provider_context.db;
    let providers = global::providers();
    let msg: QcmMessage = {
        let mut msg = ProviderStatusMsg::default();

        let libraries = sqlm::library::Entity::find().all(db).await?;

        msg.statuses = providers
            .iter()
            .map(|p| {
                let mut status = ProviderStatus::default();
                status.id = p.id().unwrap_or(-1);
                status.name = p.name();
                status.type_name = p.type_name().to_string();

                for lib in &libraries {
                    if Some(lib.provider_id) == p.id() {
                        status.libraries.push(lib.clone().qcm_into());
                    }
                }
                if let Some(s) = sync_status.get(&status.id) {
                    status.sync_status = Some(s.clone());
                }
                return status;
            })
            .collect();

        msg.full = true;
        msg.qcm_into()
    };
    ctx.ws_sender.send(msg.qcm_try_into()?).await?;
    Ok(providers)
}
