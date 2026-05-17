// DTOs in this tree are OpenAI-compatible mirrors of provider request/response
// schemas; field meanings are documented in the provider docs (openai.com/docs,
// anthropic docs, etc.) rather than re-stated per-field here. Suppress
// `missing_docs` on each DTO submodule to avoid duplicating upstream docs.

#[allow(missing_docs)]
pub mod audio;
#[allow(missing_docs)]
pub mod batch;
#[allow(missing_docs)]
pub mod chat;
#[allow(missing_docs)]
pub mod common;
#[allow(missing_docs)]
pub mod embedding;
#[allow(missing_docs)]
pub mod files;
#[allow(missing_docs)]
pub mod image;
#[allow(missing_docs)]
pub mod models;
#[allow(missing_docs)]
pub mod moderation;
#[allow(missing_docs)]
pub mod ocr;
#[allow(missing_docs)]
pub mod raw;
#[allow(missing_docs)]
pub mod rerank;
#[allow(missing_docs)]
pub mod responses;
#[allow(missing_docs)]
pub mod search;

pub use audio::*;
pub use batch::*;
pub use chat::*;
pub use common::*;
pub use embedding::*;
pub use files::*;
pub use image::*;
pub use models::*;
pub use moderation::*;
pub use ocr::*;
pub use raw::*;
pub use rerank::*;
pub use responses::*;
pub use search::*;
