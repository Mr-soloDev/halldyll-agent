//! SQLite-vec extension loader.
//!
//! This module contains the unsafe initialization code for the sqlite-vec extension.
//! It is separated to minimize the scope of unsafe code in the crate.

use rusqlite::ffi::{sqlite3, sqlite3_api_routines, sqlite3_auto_extension};
use sqlite_vec::sqlite3_vec_init;

type SqliteExtensionFn =
    unsafe extern "C" fn(*mut sqlite3, *mut *mut i8, *const sqlite3_api_routines) -> i32;

/// Initialize the sqlite-vec extension globally.
///
/// This function MUST be called before opening any `SQLite` connection that
/// requires vector operations. It registers sqlite-vec as an auto-loaded
/// extension for all future connections.
///
/// # Safety
/// This function uses FFI to register a `SQLite` extension. It should only
/// be called once at application startup.
#[allow(unsafe_code)]
pub fn init_sqlite_vec_extension() {
    // SAFETY: sqlite3_auto_extension is a stable SQLite API that registers
    // an extension to be loaded automatically for new connections.
    // sqlite3_vec_init is provided by the sqlite-vec crate and is safe to use.
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<*const (), SqliteExtensionFn>(
            sqlite3_vec_init as *const (),
        )));
    }
}
