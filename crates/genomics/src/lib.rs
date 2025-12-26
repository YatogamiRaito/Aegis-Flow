//! Aegis-Genomics: High-Performance Genomic Data Processing
//!
//! Provides Apache Arrow and Polars-based genomic data analytics.
//!
//! # Features
//! - Zero-copy Arrow RecordBatch for genomic data
//! - BAM/VCF parsing with noodles
//! - Polars DataFrame for analytics
//!
//! # Example
//! ```rust,ignore
//! use aegis_genomics::{GenomicSchema, VariantRecord};
//!
//! let schema = GenomicSchema::variant();
//! let batch = VariantRecord::to_record_batch(&variants, &schema)?;
//! ```

pub mod alignment;
pub mod analytics;
pub mod bam_parser;
pub mod schema;
pub mod variant;
pub mod vcf_parser;

pub use alignment::{AlignmentBatchBuilder, AlignmentRecord};
pub use analytics::VariantAnalytics;
pub use bam_parser::BamHeader;
pub use schema::{GenomicSchema, SchemaType};
pub use variant::{VariantBatchBuilder, VariantRecord};
pub use vcf_parser::VcfParser;

/// Error types for genomics operations
#[derive(Debug, thiserror::Error)]
pub enum GenomicsError {
    #[error("Arrow error: {0}")]
    ArrowError(#[from] arrow::error::ArrowError),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Polars error: {0}")]
    PolarsError(#[from] polars::error::PolarsError),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

pub type Result<T> = std::result::Result<T, GenomicsError>;
