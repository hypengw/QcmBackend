use futures_util::SinkExt;
use prost::Message;
use sea_orm::DatabaseConnection;
use sqlx::SqlitePool;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::msg::{self, MessageType, QcmMessage, Rsp, TestReq, TestRsp};

pub async fn handle_message(
    msg: WsMessage,
    db: &DatabaseConnection,
    tx: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        WsMessage,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    use msg::qcm_message::Payload;
    if let WsMessage::Binary(data) = msg {
        let message = QcmMessage::decode(&data[..])?;

        match message.r#type() {
            MessageType::AddProviderReq => match message.payload {
                Some(Payload::AddProviderReq(req)) => if let Some(p) = req.provider {
                },
                _ => {
                    log::warn!("Unexpected payload for TEST message type");
                }
            },
            MessageType::TestReq => match message.payload {
                Some(Payload::TestReq(req)) => {
                    handle_test_message(req, message.id, tx).await?;
                }
                _ => {
                    log::warn!("Unexpected payload for TEST message type");
                }
            },
            _ => {
                log::warn!("Unhandled message type: {:?}", message.r#type());
            }
        }
    } else if let WsMessage::Text(text) = msg {
        log::info!("{}", text);
    }
    Ok(())
}

async fn handle_test_message(
    request: TestReq,
    id: String,
    tx: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        WsMessage,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = TestRsp {
        base: Some(Rsp::default()),
        test_data: format!("Echo: {}", request.test_data),
    };

    let message = QcmMessage {
        r#type: MessageType::TestRsp as i32,
        id,
        payload: Some(msg::qcm_message::Payload::TestRsp(response)),
    };

    let mut buf = Vec::new();
    message.encode(&mut buf)?;
    tx.send(WsMessage::Binary(buf.into())).await?;

    Ok(())
}
