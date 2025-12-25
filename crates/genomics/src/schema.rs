//! Arrow Schema Definitions for Genomic Data
//!
//! Defines Arrow schemas for BAM alignments and VCF variants.

use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

/// Type of genomic schema
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaType {
    /// VCF variant records
    Variant,
    /// BAM alignment records
    Alignment,
    /// FASTA/FASTQ sequence records
    Sequence,
}

/// Genomic schema factory
#[derive(Debug, Clone)]
pub struct GenomicSchema {
    /// Arrow schema
    pub schema: Arc<Schema>,
    /// Schema type
    pub schema_type: SchemaType,
}

impl GenomicSchema {
    /// Create a VCF variant schema
    pub fn variant() -> Self {
        let fields = vec![
            Field::new("chrom", DataType::Utf8, false),
            Field::new("pos", DataType::Int64, false),
            Field::new("id", DataType::Utf8, true),
            Field::new("ref", DataType::Utf8, false),
            Field::new("alt", DataType::Utf8, false),
            Field::new("qual", DataType::Float64, true),
            Field::new("filter", DataType::Utf8, true),
            Field::new("info", DataType::Utf8, true),
        ];

        Self {
            schema: Arc::new(Schema::new(fields)),
            schema_type: SchemaType::Variant,
        }
    }

    /// Create a BAM alignment schema
    pub fn alignment() -> Self {
        let fields = vec![
            Field::new("qname", DataType::Utf8, false),
            Field::new("flag", DataType::UInt16, false),
            Field::new("rname", DataType::Utf8, true),
            Field::new("pos", DataType::Int64, false),
            Field::new("mapq", DataType::UInt8, false),
            Field::new("cigar", DataType::Utf8, true),
            Field::new("rnext", DataType::Utf8, true),
            Field::new("pnext", DataType::Int64, false),
            Field::new("tlen", DataType::Int64, false),
            Field::new("seq", DataType::Utf8, false),
            Field::new("qual_str", DataType::Utf8, false),
        ];

        Self {
            schema: Arc::new(Schema::new(fields)),
            schema_type: SchemaType::Alignment,
        }
    }

    /// Create a sequence (FASTA/FASTQ) schema
    pub fn sequence() -> Self {
        let fields = vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("description", DataType::Utf8, true),
            Field::new("sequence", DataType::Utf8, false),
            Field::new("quality", DataType::Utf8, true),
        ];

        Self {
            schema: Arc::new(Schema::new(fields)),
            schema_type: SchemaType::Sequence,
        }
    }

    /// Get the underlying Arrow schema
    pub fn arrow_schema(&self) -> Arc<Schema> {
        Arc::clone(&self.schema)
    }

    /// Get field names
    pub fn field_names(&self) -> Vec<&str> {
        self.schema
            .fields()
            .iter()
            .map(|f| f.name().as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_schema() {
        let schema = GenomicSchema::variant();
        assert_eq!(schema.schema_type, SchemaType::Variant);
        assert_eq!(schema.schema.fields().len(), 8);
        assert!(schema.field_names().contains(&"chrom"));
        assert!(schema.field_names().contains(&"pos"));
    }

    #[test]
    fn test_alignment_schema() {
        let schema = GenomicSchema::alignment();
        assert_eq!(schema.schema_type, SchemaType::Alignment);
        assert_eq!(schema.schema.fields().len(), 11);
        assert!(schema.field_names().contains(&"qname"));
        assert!(schema.field_names().contains(&"mapq"));
    }

    #[test]
    fn test_sequence_schema() {
        let schema = GenomicSchema::sequence();
        assert_eq!(schema.schema_type, SchemaType::Sequence);
        assert_eq!(schema.schema.fields().len(), 4);
    }
}
