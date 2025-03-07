use crate::msg::model as proto;
use qcm_core as core;

pub trait QcmFrom<T>: Sized {
    fn qcm_from(value: T) -> Self;
}

pub trait QcmInto<T>: Sized {
    fn qcm_into(self) -> T;
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
