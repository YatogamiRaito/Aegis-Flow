//! VCF Variant Records
//!
//! Arrow-backed variant records for VCF data.

use crate::Result;
use crate::schema::GenomicSchema;
use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray};
use std::sync::Arc;

/// A single VCF variant record
#[derive(Debug, Clone)]
pub struct VariantRecord {
    /// Chromosome
    pub chrom: String,
    /// Position (1-based)
    pub pos: i64,
    /// Variant ID
    pub id: Option<String>,
    /// Reference allele
    pub reference: String,
    /// Alternate allele
    pub alternate: String,
    /// Quality score
    pub qual: Option<f64>,
    /// Filter status
    pub filter: Option<String>,
    /// Info field
    pub info: Option<String>,
}

impl VariantRecord {
    /// Create a new variant record
    pub fn new(chrom: &str, pos: i64, reference: &str, alternate: &str) -> Self {
        Self {
            chrom: chrom.to_string(),
            pos,
            id: None,
            reference: reference.to_string(),
            alternate: alternate.to_string(),
            qual: None,
            filter: None,
            info: None,
        }
    }

    /// Set variant ID
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    /// Set quality score
    pub fn with_qual(mut self, qual: f64) -> Self {
        self.qual = Some(qual);
        self
    }

    /// Set filter status
    pub fn with_filter(mut self, filter: &str) -> Self {
        self.filter = Some(filter.to_string());
        self
    }

    /// Set info field
    pub fn with_info(mut self, info: &str) -> Self {
        self.info = Some(info.to_string());
        self
    }
}

/// Builder for creating Arrow RecordBatch from variants
#[derive(Debug, Default)]
pub struct VariantBatchBuilder {
    chroms: Vec<String>,
    positions: Vec<i64>,
    ids: Vec<Option<String>>,
    refs: Vec<String>,
    alts: Vec<String>,
    quals: Vec<Option<f64>>,
    filters: Vec<Option<String>>,
    infos: Vec<Option<String>>,
}

impl VariantBatchBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new builder with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            chroms: Vec::with_capacity(capacity),
            positions: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
            refs: Vec::with_capacity(capacity),
            alts: Vec::with_capacity(capacity),
            quals: Vec::with_capacity(capacity),
            filters: Vec::with_capacity(capacity),
            infos: Vec::with_capacity(capacity),
        }
    }

    /// Add a variant record
    pub fn push(&mut self, record: VariantRecord) {
        self.chroms.push(record.chrom);
        self.positions.push(record.pos);
        self.ids.push(record.id);
        self.refs.push(record.reference);
        self.alts.push(record.alternate);
        self.quals.push(record.qual);
        self.filters.push(record.filter);
        self.infos.push(record.info);
    }

    /// Get the number of records
    pub fn len(&self) -> usize {
        self.chroms.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.chroms.is_empty()
    }

    /// Build an Arrow RecordBatch
    pub fn build(&self) -> Result<RecordBatch> {
        let schema = GenomicSchema::variant();

        let columns: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(self.chroms.clone())),
            Arc::new(Int64Array::from(self.positions.clone())),
            Arc::new(StringArray::from(
                self.ids.iter().map(|s| s.as_deref()).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(self.refs.clone())),
            Arc::new(StringArray::from(self.alts.clone())),
            Arc::new(Float64Array::from(self.quals.clone())),
            Arc::new(StringArray::from(
                self.filters
                    .iter()
                    .map(|s| s.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                self.infos.iter().map(|s| s.as_deref()).collect::<Vec<_>>(),
            )),
        ];

        Ok(RecordBatch::try_new(schema.arrow_schema(), columns)?)
    }

    /// Clear the builder
    pub fn clear(&mut self) {
        self.chroms.clear();
        self.positions.clear();
        self.ids.clear();
        self.refs.clear();
        self.alts.clear();
        self.quals.clear();
        self.filters.clear();
        self.infos.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_record_creation() {
        let record = VariantRecord::new("chr1", 12345, "A", "G")
            .with_id("rs123")
            .with_qual(99.0);

        assert_eq!(record.chrom, "chr1");
        assert_eq!(record.pos, 12345);
        assert_eq!(record.reference, "A");
        assert_eq!(record.alternate, "G");
        assert_eq!(record.qual, Some(99.0));
    }

    #[test]
    fn test_variant_batch_builder() {
        let mut builder = VariantBatchBuilder::new();

        builder.push(VariantRecord::new("chr1", 100, "A", "T"));
        builder.push(VariantRecord::new("chr1", 200, "G", "C").with_qual(50.0));
        builder.push(VariantRecord::new("chr2", 300, "T", "A"));

        assert_eq!(builder.len(), 3);

        let batch = builder.build().unwrap();
        assert_eq!(batch.num_rows(), 3);
        assert_eq!(batch.num_columns(), 8);
    }

    #[test]
    fn test_empty_batch() {
        let builder = VariantBatchBuilder::new();
        assert!(builder.is_empty());

        let batch = builder.build().unwrap();
        assert_eq!(batch.num_rows(), 0);
    }

    #[test]
    fn test_with_filter() {
        let record = VariantRecord::new("chr1", 100, "A", "T").with_filter("PASS");
        assert_eq!(record.filter, Some("PASS".to_string()));
    }

    #[test]
    fn test_builder_clear() {
        let mut builder = VariantBatchBuilder::new();
        builder.push(VariantRecord::new("chr1", 100, "A", "T"));
        assert_eq!(builder.len(), 1);

        builder.clear();
        assert!(builder.is_empty());
    }

    #[test]
    fn test_builder_with_capacity() {
        let builder = VariantBatchBuilder::with_capacity(10);
        assert!(builder.is_empty());
    }

    #[test]
    fn test_variant_record_clone() {
        let record = VariantRecord::new("chr1", 100, "A", "T")
            .with_qual(99.0)
            .with_filter("PASS");
        let cloned = record.clone();
        assert_eq!(record.chrom, cloned.chrom);
        assert_eq!(record.qual, cloned.qual);
    }

    #[test]
    fn test_builder_multiple_chromosomes() {
        let mut builder = VariantBatchBuilder::with_capacity(25);
        for i in 1..=22 {
            builder.push(VariantRecord::new(&format!("chr{}", i), i * 100, "A", "T"));
        }
        assert_eq!(builder.len(), 22);
    }

    #[test]
    fn test_variant_record_with_info() {
        let record = VariantRecord::new("chr1", 100, "A", "T")
            .with_info("DP=50;AF=0.25");
        assert_eq!(record.info, Some("DP=50;AF=0.25".to_string()));
    }

    #[test]
    fn test_variant_record_all_fields() {
        let record = VariantRecord::new("chrX", 500, "GGG", "TTT")
            .with_id("rs12345")
            .with_qual(100.5)
            .with_filter("PASS")
            .with_info("DP=100");
        
        assert_eq!(record.chrom, "chrX");
        assert_eq!(record.pos, 500);
        assert_eq!(record.ref_allele, "GGG");
        assert_eq!(record.alt_allele, "TTT");
        assert!(record.id.is_some());
        assert!(record.qual.is_some());
        assert!(record.filter.is_some());
        assert!(record.info.is_some());
    }

    #[test]
    fn test_builder_push_multiple() {
        let mut builder = VariantBatchBuilder::new();
        
        for i in 1..=10 {
            builder.push(VariantRecord::new("chr1", i * 1000, "A", "G"));
        }
        
        assert_eq!(builder.len(), 10);
        assert!(!builder.is_empty());
    }

    #[test]
    fn test_variant_record_minimal() {
        let record = VariantRecord::new("chr1", 1, "A", "T");
        assert!(record.id.is_none());
        assert!(record.qual.is_none());
        assert!(record.filter.is_none());
        assert!(record.info.is_none());
    }

    #[test]
    fn test_builder_capacity_growth() {
        let mut builder = VariantBatchBuilder::with_capacity(2);
        
        // Push more than capacity
        for i in 0..5 {
            builder.push(VariantRecord::new("chr1", i, "A", "T"));
        }
        
        assert_eq!(builder.len(), 5);
    }

    #[test]
    fn test_variant_record_debug_format() {
        let record = VariantRecord::new("chr1", 100, "A", "T");
        let debug_str = format!("{:?}", record);
        assert!(debug_str.contains("chr1"));
        assert!(debug_str.contains("100"));
    }
}
