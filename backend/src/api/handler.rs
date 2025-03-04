use futures_util::SinkExt;
use prost::Message;
use sqlx::SqlitePool;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::msg::{self, MessageType, QcmMessage, TestRequest, TestResponse};

pub async fn handle_message(
    msg: WsMessage,
    pool: &SqlitePool,
    tx: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        WsMessage,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    if let WsMessage::Binary(data) = msg {
        let message = QcmMessage::decode(&data[..])?;

        match message.r#type() {
            MessageType::TestRequest => {
                if let Some(payload) = message.payload {
                    match payload {
                        msg::qcm_message::Payload::TestRequest(req) => {
                            handle_test_message(req, message.request_id, tx).await?;
                        }
                        _ => {
                            log::warn!("Unexpected payload for TEST message type");
                        }
                    }
                }
            }
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
    request: TestRequest,
    request_id: String,
    tx: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        WsMessage,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = TestResponse {
        test_data: format!("Echo: {}", request.test_data),
        success: true,
    };

    let message = QcmMessage {
        r#type: MessageType::TestResponse as i32,
        request_id,
        payload: Some(msg::qcm_message::Payload::TestResponse(response)),
    };

    let mut buf = Vec::new();
    message.encode(&mut buf)?;
    tx.send(WsMessage::Binary(buf.into())).await?;

    Ok(())
}
