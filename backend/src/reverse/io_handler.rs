use super::bus::EventBus;
use super::io::{IoContext, IoEvent, ReadState};
use std::path::PathBuf;

pub fn process_io(
    tx: tokio::sync::mpsc::Sender<EventBus>,
    rx: std::sync::mpsc::Receiver<IoEvent>,
    cache_dir: PathBuf,
) {
    let mut ctx = IoContext::new(&cache_dir);

    loop {
        match rx.recv() {
            Ok(ev) => match ev {
                IoEvent::EndConnection(id) => {
                    ctx.end(id);
                }
                IoEvent::EndWrite(key) => {
                    ctx.end_writer(&key);
                }
                IoEvent::RequestRead(key, id, cursor, has_cache) => {
                    // when two reader and one writer
                    // we keep finished writer
                    // try clean writer after sql finished if no reader
                    // try clean writer when clean reader
                    let mut p = None;

                    if has_cache {
                        p = ctx.get_piece_from_file(&key, id, cursor);
                    }
                    if p.is_none() {
                        p = ctx.get_piece_from_wirter(&key, id, cursor);
                    }
                    if p.is_none() {
                        ctx.add_waiter(id, &key, cursor);
                        let _ = tx.try_send(EventBus::NoCache(id));
                    }
                }
                IoEvent::ReadContinue(id) => {
                    ctx.set_reader_state(id, ReadState::Reading(0));
                }
                IoEvent::DoRead => {
                    ctx.do_read(
                        |id, state, bytes| {
                            let _ = tx.try_send(EventBus::ReadedBuf(id, bytes, state));
                        },
                        |id| {
                            let _ = tx.try_send(EventBus::EndConnection(id));
                        },
                    );
                }
                IoEvent::NewWrite(key, len, cache_type, remote_info, res) => {
                    let ok = ctx.new_write(&key, len, cache_type, remote_info);
                    let _ = res.send(ok);
                }
                IoEvent::DoWrite(key, offset, bytes) => {
                    if let Err(e) = ctx.do_write(
                        key.as_str(),
                        &bytes,
                        offset,
                        |key, id, start| {
                            let _ = tx.try_send(EventBus::RequestRead(
                                key.to_string(),
                                id,
                                start,
                                false,
                            ));
                        },
                        |key, f| {
                            let _ = tx.try_send(EventBus::FinishFile(
                                key.to_string(),
                                f.cache_type,
                                f.remote_info.clone(),
                            ));
                        },
                    ) {
                        log::error!(target:"reverse", "{:?}", e);
                    }
                }
            },
            Err(_) => {
                break;
            }
        }

        for _ in 0..ctx.reading_count() {
            let _ = tx.try_send(EventBus::DoRead);
        }
    }
}
