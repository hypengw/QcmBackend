use super::tokenizer::FtsTokenizer;
use libsqlite3_sys as api;
use std::ffi::CStr;

#[no_mangle]
pub unsafe extern "C" fn create(
    _arg1: *mut ::std::os::raw::c_void,
    _az_arg: *mut *const ::std::os::raw::c_char,
    n_arg: ::std::os::raw::c_int,
    pp_out: *mut *mut api::Fts5Tokenizer,
) -> ::std::os::raw::c_int {
    let mut rc: ::std::os::raw::c_int = 0;

    if n_arg != 0 {
        rc = api::SQLITE_MISUSE;
    } else {
        let tokenizer = Box::new(FtsTokenizer::new());
        *pp_out = Box::into_raw(tokenizer) as *mut api::Fts5Tokenizer;
    }

    rc
}

#[no_mangle]
pub unsafe extern "C" fn delete(arg1: *mut api::Fts5Tokenizer) {
    if !arg1.is_null() {
        let _ = Box::from_raw(arg1 as *mut FtsTokenizer);
    }
}

#[no_mangle]
pub unsafe extern "C" fn tokenize(
    arg1: *mut api::Fts5Tokenizer,
    p_ctx: *mut ::std::os::raw::c_void,
    flags: ::std::os::raw::c_int,
    p_text: *const ::std::os::raw::c_char,
    _n_text: ::std::os::raw::c_int,
    x_token: ::std::option::Option<
        unsafe extern "C" fn(
            p_ctx: *mut ::std::os::raw::c_void,
            tflags: ::std::os::raw::c_int,
            p_token: *const ::std::os::raw::c_char,
            n_token: ::std::os::raw::c_int,
            i_start: ::std::os::raw::c_int,
            i_end: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
) -> ::std::os::raw::c_int {
    if arg1.is_null() || p_text.is_null() {
        return api::SQLITE_ERROR;
    }

    let tokenizer = &*(arg1 as *const FtsTokenizer);
    let text = CStr::from_ptr(p_text).to_string_lossy();
    let tokens = tokenizer.tokenize(&text);

    for token in tokens {
        if let Some(callback) = x_token {
            let start_pos = text.find(&token).unwrap_or(0) as i32;
            let token_len = token.len() as i32;
            callback(
                p_ctx,
                flags,
                token.as_ptr() as *const i8,
                token_len,
                start_pos,
                start_pos + token_len,
            );
        }
    }

    api::SQLITE_OK
}
