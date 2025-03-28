use std::ops::Deref;

use crate::error::ProcessError;
use crate::msg::model as proto;
use crate::msg::{self, QcmMessage};
use chrono::Timelike;
use prost::{DecodeError, EncodeError, Message};
use qcm_core as core;
use qcm_core::db::values::StringVec;
use tokio_tungstenite::tungstenite::Message as WsMessage;

pub trait QcmFrom<T>: Sized {
    fn qcm_from(value: T) -> Self;
}

pub trait QcmInto<T>: Sized {
    fn qcm_into(self) -> T;
}

pub trait QcmTryFrom<T>: Sized {
    type Error;

    // Required method
    fn qcm_try_from(value: T) -> Result<Self, Self::Error>;
}
pub trait QcmTryInto<T>: Sized {
    type Error;

    // Required method
    fn qcm_try_into(self) -> Result<T, Self::Error>;
}

// From implies Into
impl<T, U> QcmInto<U> for T
where
    U: QcmFrom<T>,
{
    #[inline]
    fn qcm_into(self) -> U {
        U::qcm_from(self)
    }
}

impl<T, U> QcmTryInto<U> for T
where
    U: QcmTryFrom<T>,
{
    type Error = U::Error;

    #[inline]
    fn qcm_try_into(self) -> Result<U, Self::Error> {
        U::qcm_try_from(self)
    }
}

impl QcmFrom<prost_types::Timestamp> for sea_orm::entity::prelude::DateTimeUtc {
    fn qcm_from(v: prost_types::Timestamp) -> Self {
        let datetime =
            chrono::DateTime::from_timestamp(v.seconds, v.nanos as u32).unwrap_or_default();
        datetime
    }
}

impl QcmFrom<sea_orm::entity::prelude::DateTimeUtc> for prost_types::Timestamp {
    fn qcm_from(v: sea_orm::entity::prelude::DateTimeUtc) -> Self {
        let seconds = v.timestamp();
        let nanos = v.nanosecond() as i32;

        Self { seconds, nanos }
    }
}

impl QcmFrom<proto::auth_info::Method> for core::provider::AuthMethod {
    fn qcm_from(value: proto::auth_info::Method) -> Self {
        use proto::auth_info::Method;
        match value {
            Method::Username(au) => Self::Username {
                username: au.username,
                pw: au.pw,
            },
            Method::Phone(au) => Self::Phone {
                phone: au.phone,
                pw: au.pw,
            },
            Method::Email(au) => Self::Email {
                email: au.email,
                pw: au.pw,
            },
        }
    }
}

impl QcmFrom<Option<proto::auth_info::Method>> for core::provider::AuthMethod {
    fn qcm_from(value: Option<proto::auth_info::Method>) -> Self {
        match value {
            Some(some) => some.qcm_into(),
            None => Self::None,
        }
    }
}
impl QcmFrom<proto::AuthInfo> for core::provider::AuthInfo {
    fn qcm_from(value: proto::AuthInfo) -> Self {
        Self {
            server_url: value.server_url,
            method: value.method.qcm_into(),
        }
    }
}

impl QcmFrom<core::provider::AuthMethod> for Option<proto::auth_info::Method> {
    fn qcm_from(value: core::provider::AuthMethod) -> Self {
        use core::provider::AuthMethod;
        use proto::auth_info::Method;
        match value {
            AuthMethod::Username { username, pw } => {
                Some(Method::Username(proto::UsernameAuth { username, pw }))
            }
            AuthMethod::Phone { phone, pw } => Some(Method::Phone(proto::PhoneAuth { phone, pw })),
            AuthMethod::Email { email, pw } => Some(Method::Email(proto::EmailAuth { email, pw })),
            AuthMethod::None => None,
        }
    }
}

impl QcmFrom<core::provider::AuthInfo> for proto::AuthInfo {
    fn qcm_from(value: core::provider::AuthInfo) -> Self {
        Self {
            server_url: value.server_url,
            method: value.method.qcm_into(),
        }
    }
}

impl QcmFrom<proto::Album> for core::model::album::Model {
    fn qcm_from(v: proto::Album) -> Self {
        Self {
            id: v.id.parse().unwrap_or_default(),
            item_id: v.item_id,
            library_id: v.library_id.parse().unwrap_or_default(),
            name: v.name,
            pic_id: v.pic_url,
            publish_time: v.publish_time.unwrap_or_default().qcm_into(),
            track_count: v.track_count,
            description: v.description,
            company: v.company,
            type_: v.r#type,
            genres: StringVec::default(),
            edit_time: v.edit_time.unwrap_or_default().qcm_into(),
        }
    }
}

impl QcmFrom<core::model::album::Model> for proto::Album {
    fn qcm_from(v: core::model::album::Model) -> Self {
        Self {
            id: v.id.to_string(),
            item_id: v.item_id,
            library_id: v.library_id.to_string(),
            name: v.name,
            pic_url: v.pic_id,
            publish_time: Some(v.publish_time.qcm_into()),
            track_count: v.track_count,
            description: v.description,
            company: v.company,
            r#type: v.type_,
            edit_time: Some(v.edit_time.qcm_into()),
        }
    }
}

impl QcmFrom<core::provider::ProviderMeta> for proto::ProviderMeta {
    fn qcm_from(v: core::provider::ProviderMeta) -> Self {
        Self {
            type_name: v.type_name,
            svg: v.svg.deref().clone(),
            mutable: v.mutable,
            is_script: v.is_script,
            has_server_url: v.has_server_url,
        }
    }
}

impl QcmFrom<ProcessError> for msg::Rsp {
    fn qcm_from(v: ProcessError) -> Self {
        Self {
            code: match v {
                ProcessError::Internal(_) => msg::ErrorCode::Internal.into(),
                ProcessError::Encode(_) => msg::ErrorCode::Encode.into(),
                ProcessError::Decode(_) => msg::ErrorCode::Decode.into(),
                ProcessError::UnsupportedMessageType(_) => {
                    msg::ErrorCode::UnsupportedMessageType.into()
                }
                ProcessError::UnknownMessageType(_) => msg::ErrorCode::UnknownMessageType.into(),
                ProcessError::UnexpectedPayload(_) => msg::ErrorCode::UnexpectedPayload.into(),
                ProcessError::MissingFields(_) => msg::ErrorCode::MissingFields.into(),
                ProcessError::NoSuchProviderType(_) => msg::ErrorCode::NoSuchProviderType.into(),
                ProcessError::Db(_) => msg::ErrorCode::Db.into(),
                ProcessError::NoSuchLibrary(_) => msg::ErrorCode::NoSuchLibrary.into(),
                ProcessError::NoSuchProvider(_) => msg::ErrorCode::NoSuchProvider.into(),
                ProcessError::HyperBody(_) => msg::ErrorCode::HyperBody.into(),
                ProcessError::Infallible(_) => {
                    panic!("Got infallible error!")
                }
                ProcessError::None => msg::ErrorCode::Ok.into(),
            },
            message: v.to_string(),
        }
    }
}

impl QcmTryFrom<QcmMessage> for WsMessage {
    type Error = EncodeError;
    fn qcm_try_from(msg: QcmMessage) -> Result<Self, EncodeError> {
        let mut buf = Vec::new();
        msg.encode(&mut buf)?;
        Ok(WsMessage::Binary(buf.into()))
    }
}

macro_rules! impl_from_for_qcm_msg {
    ($msg_type:ident) => {
        impl QcmFrom<msg::$msg_type> for QcmMessage {
            fn qcm_from(v: msg::$msg_type) -> Self {
                Self {
                    id: 0,
                    r#type: msg::MessageType::$msg_type.into(),
                    payload: Some(msg::qcm_message::Payload::$msg_type(v)),
                }
            }
        }
    };
}
impl_from_for_qcm_msg!(ProviderMetaStatusMsg);
impl_from_for_qcm_msg!(ProviderStatusMsg);
impl_from_for_qcm_msg!(GetProviderMetasRsp);
impl_from_for_qcm_msg!(TestRsp);
impl_from_for_qcm_msg!(GetAlbumsRsp);
impl_from_for_qcm_msg!(Rsp);
