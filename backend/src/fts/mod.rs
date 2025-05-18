mod tokenizer;
mod entry;

use libsqlite3_sys as api;
use std::os::raw::{c_char, c_int};

extern "C" fn hello_rust(
    ctx: *mut api::sqlite3_context,
    _argc: c_int,
    _argv: *mut *mut api::sqlite3_value,
) {
    let result = "Hello from Rust!";
    unsafe {
        api::sqlite3_result_text(
            ctx,
            result.as_ptr() as *const i8,
            result.len() as c_int,
            api::SQLITE_TRANSIENT(),
        );
    }
}

unsafe fn fts5_api_from_db(db: *mut api::sqlite3, pp_api: *mut *mut api::fts5_api) -> c_int {
    let mut p_stmt: *mut api::sqlite3_stmt = std::ptr::null_mut();
    let mut rc: c_int;

    *pp_api = std::ptr::null_mut();
    rc = api::sqlite3_prepare_v2(
        db,
        "SELECT fts5(?1)\0".as_ptr() as *const i8,
        -1,
        &mut p_stmt,
        std::ptr::null_mut(),
    );

    if rc == api::SQLITE_OK {
        api::sqlite3_bind_pointer(
            p_stmt,
            1,
            pp_api as *mut _,
            "fts5_api_ptr\0".as_ptr() as *const i8,
            None,
        );
        api::sqlite3_step(p_stmt);
        rc = api::sqlite3_finalize(p_stmt);
    }

    rc
}

unsafe extern "C" fn sqlite3_qcm_init(
    db: *mut api::sqlite3,
    _pz_errmsg: *mut *mut c_char,
    _p_api: *const api::sqlite3_api_routines,
) -> c_int {
    let mut rc = api::sqlite3_create_function_v2(
        db,
        b"hello_rust\0".as_ptr() as *const _,
        0,
        api::SQLITE_UTF8,
        std::ptr::null_mut(),
        Some(hello_rust),
        None,
        None,
        None,
    );

    if rc != api::SQLITE_OK {
        log::error!("sqlite ec: {}", rc);
        return rc;
    }

    let mut fts_api_p = std::ptr::null_mut();
    let mut fts_api = {
        rc = fts5_api_from_db(db, &mut fts_api_p);
        if rc != api::SQLITE_OK {
            log::error!("sqlite ec: {}", rc);
            return rc;
        }
        fts_api_p.as_mut().unwrap()
    };

    let mut tokenizer = api::fts5_tokenizer {
        xCreate: Some(entry::create),
        xDelete: Some(entry::delete),
        xTokenize: Some(entry::tokenize),
    };

    rc = fts_api.xCreateTokenizer.unwrap()(
        fts_api_p,
        "qcm_tokenizer\0".as_ptr() as *const i8,
        std::ptr::null_mut(),
        &mut tokenizer,
        None,
    );

    if rc != api::SQLITE_OK {
        log::error!("sqlite ec: {}", rc);
        return rc;
    }

    rc
}

pub fn load_fts_plugin() -> bool {
    let mut rc = api::SQLITE_OK;
    unsafe {
        rc = api::sqlite3_auto_extension(Some(sqlite3_qcm_init));
    }
    rc == api::SQLITE_OK
}
