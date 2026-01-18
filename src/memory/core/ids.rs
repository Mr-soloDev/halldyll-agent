// File: src/memory/core/ids.rs

//! Identifier types for the conversational memory engine.
//!
//! This module is intentionally **type-heavy** and **logic-light**.
//! It provides strongly-typed ID newtypes (compile-time safety) and
//! helpers for generation, parsing, and formatting.
//!
//! ## Multi-user / multi-model continuity
//! Multiple models (text, embeddings, image, audio) share the same memory
//! as long as your orchestrator writes/reads to the same storage and
//! propagates the same (`UserId`, `SessionId`) across all calls.
//!
//! ## Cargo features used by this module
//! - `uuid_v7`: enables `UUIDv7` generation via `uuid/v7`.
//! - `ulid_ids`: adds optional ULID-based identifiers.
//! - `nanoid_ids`: adds optional NanoID-based public codes.
//! - `sqlx`: derives `sqlx::Type` for transparent newtypes.
//! - `rusqlite`: implements `ToSql`/`FromSql` for transparent newtypes.

use core::fmt;
use core::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generate an ID intended to have good DB insert locality.
///
/// With feature `uuid_v7` enabled, this uses `Uuid::now_v7()`.
/// Otherwise it falls back to `Uuid::new_v4()`.
#[inline]
#[must_use]
fn uuid_time_ordered() -> Uuid {
    #[cfg(feature = "uuid_v7")]
    {
        Uuid::now_v7()
    }
    #[cfg(not(feature = "uuid_v7"))]
    {
        Uuid::new_v4()
    }
}

/// Generate a random UUID (v4).
#[inline]
#[must_use]
fn uuid_random() -> Uuid {
    Uuid::new_v4()
}

/// Declare a UUID newtype with a consistent API.
macro_rules! define_uuid_id {
    (
        $(#[$meta:meta])*
        $name:ident,
        generator = $gen:ident
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[repr(transparent)]
        #[serde(transparent)]
        #[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
        #[cfg_attr(feature = "sqlx", sqlx(transparent))]
        pub struct $name(pub Uuid);

        impl Default for $name {
            #[inline]
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            /// Create a new identifier.
            #[inline]
            #[must_use]
            pub fn new() -> Self {
                Self($gen())
            }

            /// Wrap an existing UUID.
            #[inline]
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Borrow the underlying UUID.
            #[inline]
            #[must_use]
            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            /// Extract the underlying UUID.
            #[inline]
            #[must_use]
            pub const fn into_uuid(self) -> Uuid {
                self.0
            }

            /// Convert to 16 bytes for compact DB storage (e.g., `SQLite` `BLOB(16)`).
            #[inline]
            #[must_use]
            pub const fn to_bytes(self) -> [u8; 16] {
                self.0.into_bytes()
            }

            /// Build from 16 bytes (e.g., `SQLite` `BLOB(16)`).
            #[inline]
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(Uuid::from_bytes(bytes))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            #[inline]
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            #[inline]
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl AsRef<Uuid> for $name {
            #[inline]
            fn as_ref(&self) -> &Uuid {
                &self.0
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

// ===== Core IDs =============================================================

define_uuid_id!(
    /// User account identifier.
    ///
    /// Default strategy: `UUIDv4` (random) to reduce timestamp leakage when exposed.
    UserId,
    generator = uuid_random
);

define_uuid_id!(
    /// Identifier of a logical agent.
    ///
    /// Useful when multiple agents (tools, specialists) share the same storage.
    AgentId,
    generator = uuid_time_ordered
);

define_uuid_id!(
    /// Optional grouping identifier (workspace / project / tenant).
    ProjectId,
    generator = uuid_time_ordered
);

define_uuid_id!(
    /// Identifier for a conversation session.
    ///
    /// Propagate the same `SessionId` across all model invocations (text, embeddings,
    /// image, audio) if you want them to share the same memory.
    SessionId,
    generator = uuid_time_ordered
);

/// Alias for naming consistency.
pub type ConversationId = SessionId;

define_uuid_id!(
    /// Identifier for a conversation turn (one user message and its processing).
    TurnId,
    generator = uuid_time_ordered
);

define_uuid_id!(
    /// Identifier for a stored memory item (fact, summary, embedding chunk, etc.).
    MemoryId,
    generator = uuid_time_ordered
);

/// Preferred name in memory engines.
pub type MemoryItemId = MemoryId;

define_uuid_id!(
    /// Identifier for a model invocation request.
    ///
    /// Use this as a correlation identifier across logs spanning multiple models.
    RequestId,
    generator = uuid_time_ordered
);

/// Alias for tracing/correlation.
pub type CorrelationId = RequestId;

define_uuid_id!(
    /// Identifier for a system event (tool call, DB write, summarization run, etc.).
    EventId,
    generator = uuid_time_ordered
);

// ===== Model IDs ============================================================

/// Errors returned when parsing/validating a [`ModelId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelIdError {
    /// Empty (or whitespace-only) identifier.
    Empty,
    /// Exceeds the maximum accepted length.
    TooLong {
        /// Maximum allowed length.
        max: usize,
        /// Actual length received.
        got: usize,
    },
    /// Contains a disallowed character.
    InvalidChar {
        /// The invalid character.
        ch: char,
        /// The index where it was found.
        index: usize,
    },
}

impl fmt::Display for ModelIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "model id must not be empty"),
            Self::TooLong { max, got } => write!(f, "model id too long: got {got}, max {max}"),
            Self::InvalidChar { ch, index } => {
                write!(
                    f,
                    "model id contains invalid character {ch:?} at index {index}"
                )
            }
        }
    }
}

impl std::error::Error for ModelIdError {}

/// Identifier for a model (routing key).
///
/// Examples:
/// - `ollama:ministral-3:8b-instruct-2512-q8_0`
/// - `openai:gpt-5`
/// - `hf:meta-llama/Llama-3.1-8B-Instruct`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(transparent))]
pub struct ModelId(String);

impl ModelId {
    /// Hard ceiling to prevent pathological payloads.
    pub const MAX_LEN: usize = 192;

    /// Build a validated `ModelId`.
    ///
    /// Rules:
    /// - Non-empty after trimming.
    /// - Max length limited.
    /// - Conservative ASCII set: `[A-Za-z0-9._:/+-@]`.
    ///
    /// # Errors
    /// Returns `ModelIdError` if the input is empty, too long, or contains invalid characters.
    pub fn new(raw: impl AsRef<str>) -> Result<Self, ModelIdError> {
        let s = raw.as_ref().trim();

        if s.is_empty() {
            return Err(ModelIdError::Empty);
        }
        if s.len() > Self::MAX_LEN {
            return Err(ModelIdError::TooLong {
                max: Self::MAX_LEN,
                got: s.len(),
            });
        }

        for (i, ch) in s.chars().enumerate() {
            let ok =
                ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '/' | '-' | '+' | '@');
            if !ok {
                return Err(ModelIdError::InvalidChar { ch, index: i });
            }
        }

        Ok(Self(s.to_owned()))
    }

    /// Borrow as `&str`.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into `String`.
    #[inline]
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModelId {
    type Err = ModelIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for ModelId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<ModelId> for String {
    fn from(value: ModelId) -> Self {
        value.into_string()
    }
}

impl TryFrom<String> for ModelId {
    type Error = ModelIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// ===== Optional: human-friendly IDs =========================================

/// A human-friendly, lexicographically sortable ID (ULID).
///
/// Enable with the `ulid_ids` feature.
#[cfg(feature = "ulid_ids")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct UlidId(pub ulid::Ulid);

#[cfg(feature = "ulid_ids")]
impl Default for UlidId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "ulid_ids")]
impl UlidId {
    /// Generate a new ULID.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }

    /// Borrow the underlying ULID.
    #[inline]
    #[must_use]
    pub const fn as_ulid(&self) -> &ulid::Ulid {
        &self.0
    }
}

#[cfg(feature = "ulid_ids")]
impl fmt::Display for UlidId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// A short, URL-safe public code (NanoID).
///
/// Enable with the `nanoid_ids` feature.
#[cfg(feature = "nanoid_ids")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct PublicCode(String);

#[cfg(feature = "nanoid_ids")]
impl PublicCode {
    /// Default NanoID length (21 chars) is roughly ~126 bits of randomness.
    pub const DEFAULT_LEN: usize = 21;

    /// Generate a new public code with the default length.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(nanoid::nanoid!(Self::DEFAULT_LEN))
    }

    /// Generate a new public code with a custom length.
    #[inline]
    #[must_use]
    pub fn with_len(len: usize) -> Self {
        Self(nanoid::nanoid!(len))
    }

    /// Borrow as `&str`.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into `String`.
    #[inline]
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

#[cfg(feature = "nanoid_ids")]
impl Default for PublicCode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nanoid_ids")]
impl fmt::Display for PublicCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ===== Rusqlite integration ================================================

mod rusqlite_impl {
    use super::{
        AgentId, EventId, MemoryId, ModelId, ModelIdError, ProjectId, RequestId, SessionId, TurnId,
        UserId,
    };
    use std::error::Error as _;
    use std::fmt;

    use rusqlite::types::{
        FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, Value, ValueRef,
    };

    #[derive(Debug)]
    struct InvalidUuidBlobLen {
        got: usize,
    }

    impl fmt::Display for InvalidUuidBlobLen {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "invalid UUID blob length: got {}, expected 16", self.got)
        }
    }

    impl std::error::Error for InvalidUuidBlobLen {}

    fn uuid_from_blob(b: &[u8]) -> FromSqlResult<uuid::Uuid> {
        let bytes: [u8; 16] = b
            .try_into()
            .map_err(|_| FromSqlError::Other(Box::new(InvalidUuidBlobLen { got: b.len() })))?;
        Ok(uuid::Uuid::from_bytes(bytes))
    }

    fn uuid_from_text(t: &[u8]) -> FromSqlResult<uuid::Uuid> {
        let s = std::str::from_utf8(t).map_err(|e| FromSqlError::Other(Box::new(e)))?;
        uuid::Uuid::parse_str(s).map_err(|e| FromSqlError::Other(Box::new(e)))
    }

    macro_rules! impl_rusqlite_uuid_newtype {
        ($t:ty) => {
            impl ToSql for $t {
                fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                    // Store UUIDs as TEXT for compatibility
                    Ok(ToSqlOutput::Owned(Value::Text(self.0.to_string())))
                }
            }

            impl FromSql for $t {
                fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                    match value {
                        ValueRef::Blob(b) => uuid_from_blob(b).map(Self),
                        ValueRef::Text(t) => uuid_from_text(t).map(Self),
                        _ => Err(FromSqlError::InvalidType),
                    }
                }
            }
        };
    }

    impl_rusqlite_uuid_newtype!(UserId);
    impl_rusqlite_uuid_newtype!(AgentId);
    impl_rusqlite_uuid_newtype!(ProjectId);
    impl_rusqlite_uuid_newtype!(SessionId);
    impl_rusqlite_uuid_newtype!(TurnId);
    impl_rusqlite_uuid_newtype!(MemoryId);
    impl_rusqlite_uuid_newtype!(RequestId);
    impl_rusqlite_uuid_newtype!(EventId);

    impl ToSql for ModelId {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            Ok(ToSqlOutput::Owned(Value::Text(self.as_str().to_owned())))
        }
    }

    impl FromSql for ModelId {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            match value {
                ValueRef::Text(t) => {
                    let s = std::str::from_utf8(t).map_err(|e| FromSqlError::Other(Box::new(e)))?;
                    Self::new(s).map_err(|e| FromSqlError::Other(Box::new(e)))
                }
                ValueRef::Null => Err(FromSqlError::Other(Box::new(ModelIdError::Empty))),
                _ => Err(FromSqlError::InvalidType),
            }
        }
    }

    // Avoid \"unused import\" when compiling with `rusqlite` but not using the helper traits.
    #[allow(dead_code)]
    fn _assert_error_traits() {
        let _ = ModelIdError::Empty.source();
    }
}
