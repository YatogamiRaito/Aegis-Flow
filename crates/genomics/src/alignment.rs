//! BAM Alignment Records
//!
//! Arrow-backed alignment records for BAM data.

use crate::Result;
use crate::schema::GenomicSchema;
use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray, UInt8Array, UInt16Array};
use std::sync::Arc;

/// A single BAM alignment record
#[derive(Debug, Clone)]
pub struct AlignmentRecord {
    /// Query name
    pub qname: String,
    /// Bitwise flag
    pub flag: u16,
    /// Reference name
    pub rname: Option<String>,
    /// Position (1-based)
    pub pos: i64,
    /// Mapping quality
    pub mapq: u8,
    /// CIGAR string
    pub cigar: Option<String>,
    /// Mate reference name
    pub rnext: Option<String>,
    /// Mate position
    pub pnext: i64,
    /// Template length
    pub tlen: i64,
    /// Sequence
    pub seq: String,
    /// Quality string
    pub qual: String,
}

impl AlignmentRecord {
    /// Create a new alignment record
    pub fn new(qname: &str, flag: u16, pos: i64, seq: &str) -> Self {
        Self {
            qname: qname.to_string(),
            flag,
            rname: None,
            pos,
            mapq: 0,
            cigar: None,
            rnext: None,
            pnext: 0,
            tlen: 0,
            seq: seq.to_string(),
            qual: String::new(),
        }
    }

    /// Set reference name
    pub fn with_rname(mut self, rname: &str) -> Self {
        self.rname = Some(rname.to_string());
        self
    }

    /// Set mapping quality
    pub fn with_mapq(mut self, mapq: u8) -> Self {
        self.mapq = mapq;
        self
    }

    /// Set CIGAR string
    pub fn with_cigar(mut self, cigar: &str) -> Self {
        self.cigar = Some(cigar.to_string());
        self
    }

    /// Check if read is mapped
    pub fn is_mapped(&self) -> bool {
        (self.flag & 0x4) == 0
    }

    /// Check if read is reverse strand
    pub fn is_reverse(&self) -> bool {
        (self.flag & 0x10) != 0
    }
}

/// Builder for creating Arrow RecordBatch from alignments
#[derive(Debug, Default)]
pub struct AlignmentBatchBuilder {
    qnames: Vec<String>,
    flags: Vec<u16>,
    rnames: Vec<Option<String>>,
    positions: Vec<i64>,
    mapqs: Vec<u8>,
    cigars: Vec<Option<String>>,
    rnexts: Vec<Option<String>>,
    pnexts: Vec<i64>,
    tlens: Vec<i64>,
    seqs: Vec<String>,
    quals: Vec<String>,
}

impl AlignmentBatchBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an alignment record
    pub fn push(&mut self, record: AlignmentRecord) {
        self.qnames.push(record.qname);
        self.flags.push(record.flag);
        self.rnames.push(record.rname);
        self.positions.push(record.pos);
        self.mapqs.push(record.mapq);
        self.cigars.push(record.cigar);
        self.rnexts.push(record.rnext);
        self.pnexts.push(record.pnext);
        self.tlens.push(record.tlen);
        self.seqs.push(record.seq);
        self.quals.push(record.qual);
    }

    /// Get the number of records
    pub fn len(&self) -> usize {
        self.qnames.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.qnames.is_empty()
    }

    /// Build an Arrow RecordBatch
    pub fn build(&self) -> Result<RecordBatch> {
        let schema = GenomicSchema::alignment();

        let columns: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(self.qnames.clone())),
            Arc::new(UInt16Array::from(self.flags.clone())),
            Arc::new(StringArray::from(
                self.rnames.iter().map(|s| s.as_deref()).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(self.positions.clone())),
            Arc::new(UInt8Array::from(self.mapqs.clone())),
            Arc::new(StringArray::from(
                self.cigars.iter().map(|s| s.as_deref()).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                self.rnexts.iter().map(|s| s.as_deref()).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(self.pnexts.clone())),
            Arc::new(Int64Array::from(self.tlens.clone())),
            Arc::new(StringArray::from(self.seqs.clone())),
            Arc::new(StringArray::from(self.quals.clone())),
        ];

        Ok(RecordBatch::try_new(schema.arrow_schema(), columns)?)
    }

    /// Clear the builder
    pub fn clear(&mut self) {
        self.qnames.clear();
        self.flags.clear();
        self.rnames.clear();
        self.positions.clear();
        self.mapqs.clear();
        self.cigars.clear();
        self.rnexts.clear();
        self.pnexts.clear();
        self.tlens.clear();
        self.seqs.clear();
        self.quals.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_record_creation() {
        let record = AlignmentRecord::new("read1", 0, 100, "ACGT")
            .with_rname("chr1")
            .with_mapq(60)
            .with_cigar("4M");

        assert_eq!(record.qname, "read1");
        assert_eq!(record.pos, 100);
        assert_eq!(record.mapq, 60);
        assert!(record.is_mapped());
    }

    #[test]
    fn test_unmapped_read() {
        let record = AlignmentRecord::new("read2", 4, 0, "NNNN");
        assert!(!record.is_mapped());
    }

    #[test]
    fn test_reverse_strand() {
        let record = AlignmentRecord::new("read3", 16, 100, "ACGT");
        assert!(record.is_reverse());
    }

    #[test]
    fn test_alignment_batch_builder() {
        let mut builder = AlignmentBatchBuilder::new();

        builder.push(AlignmentRecord::new("read1", 0, 100, "ACGT").with_rname("chr1"));
        builder.push(AlignmentRecord::new("read2", 0, 200, "TGCA").with_mapq(50));

        assert_eq!(builder.len(), 2);

        let batch = builder.build().unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 11);
    }
}
