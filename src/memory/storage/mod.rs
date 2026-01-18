//! Persistent storage modules for memory data.

pub mod sqlite_vec_loader;
pub mod summary_store;
pub mod user_store;
pub mod vector_store;

pub use sqlite_vec_loader::init_sqlite_vec_extension;
pub use summary_store::{SqliteSummaryStore, SummaryRecord, SummaryStore};
pub use user_store::{SqliteUserStore, UserStore};
pub use vector_store::{
    SqliteVectorMemoryStore, StoreFuture, VectorMemoryStore, VectorSearchResult,
};
