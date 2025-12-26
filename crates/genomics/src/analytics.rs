//! Polars Analytics
//!
//! DataFrame analytics for genomic data using Polars.

use crate::variant::VariantBatchBuilder;

use arrow::ipc::writer::FileWriter;
use polars::io::SerReader;
use polars::prelude::*;
use std::io::Cursor;

/// Variant analytics using Polars
#[derive(Default)]
pub struct VariantAnalytics {
    df: DataFrame,
}

impl VariantAnalytics {
    /// Create from VariantBatchBuilder
    pub fn from_builder(builder: &VariantBatchBuilder) -> crate::Result<Self> {
        let batch = builder.build()?;

        // Convert Arrow RecordBatch to Polars DataFrame via IPC to handle version mismatches
        let mut buf = Vec::new();
        {
            let mut writer = FileWriter::try_new(&mut buf, &batch.schema())?;
            writer.write(&batch)?;
            writer.finish()?;
        }

        let cursor = Cursor::new(buf);
        let df = IpcReader::new(cursor).finish()?;

        Ok(Self { df })
    }

    /// Get total variant count
    pub fn count(&self) -> usize {
        self.df.height()
    }

    /// Count variants per chromosome
    pub fn count_by_chromosome(&self) -> crate::Result<Vec<(String, usize)>> {
        let counts = self
            .df
            .clone()
            .lazy()
            .group_by([col("chrom")])
            .agg([len().alias("count")])
            .sort(
                ["count"],
                SortMultipleOptions::default().with_order_descending(true),
            )
            .collect()?;

        let chroms = counts.column("chrom")?.str()?;
        let counts_col = counts.column("count")?.u32()?;

        let mut result = Vec::with_capacity(chroms.len());
        for (chrom, count) in chroms.into_iter().zip(counts_col.into_iter()) {
            if let (Some(c), Some(n)) = (chrom, count) {
                result.push((c.to_string(), n as usize));
            }
        }
        Ok(result)
    }

    /// Filter by quality threshold
    pub fn filter_by_quality(&self, min_qual: f64) -> crate::Result<usize> {
        let mask = self.df.column("qual")?.f64()?.gt_eq(min_qual);

        Ok(self.df.filter(&mask)?.height())
    }

    /// Get variants in a region
    pub fn filter_by_region(&self, chrom: &str, start: i64, end: i64) -> crate::Result<usize> {
        let ctx = self.df.clone().lazy();

        let filtered = ctx
            .filter(
                col("chrom")
                    .eq(lit(chrom))
                    .and(col("pos").gt_eq(lit(start)))
                    .and(col("pos").lt_eq(lit(end))),
            )
            .collect()?;

        Ok(filtered.height())
    }

    /// Count SNPs vs INDELs
    pub fn variant_type_counts(&self) -> crate::Result<(usize, usize)> {
        // This is a simplified check - real VCF analysis would be more complex
        // We'll check length of ref vs alt

        let df = self.df.clone();
        let refs = df.column("ref")?.str()?;
        let alts = df.column("alt")?.str()?;

        let mut snps = 0;
        let mut indels = 0;

        for (r, a) in refs.into_iter().zip(alts.into_iter()) {
            if let (Some(r_val), Some(a_val)) = (r, a) {
                if r_val.len() == 1 && a_val.len() == 1 {
                    snps += 1;
                } else {
                    indels += 1;
                }
            }
        }

        Ok((snps, indels))
    }

    /// Get quality statistics
    pub fn quality_stats(&self) -> crate::Result<QualityStats> {
        let qual_col = self.df.column("qual")?.f64()?;

        let mean = qual_col.mean().unwrap_or(0.0);
        let min = qual_col.min().unwrap_or(0.0);
        let max = qual_col.max().unwrap_or(0.0);
        // non-null count is len() - null_count() on ChunkedArray
        let count = qual_col.len() - qual_col.null_count();

        Ok(QualityStats {
            count,
            mean,
            min,
            max,
        })
    }
}

/// Quality score statistics
#[derive(Debug, Clone, Default)]
pub struct QualityStats {
    /// Number of variants with quality scores
    pub count: usize,
    /// Mean quality
    pub mean: f64,
    /// Minimum quality
    pub min: f64,
    /// Maximum quality
    pub max: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant::{VariantBatchBuilder, VariantRecord};

    fn create_test_analytics() -> VariantAnalytics {
        let mut builder = VariantBatchBuilder::new();
        builder.push(VariantRecord::new("chr1", 100, "A", "T").with_qual(99.0));
        builder.push(VariantRecord::new("chr1", 200, "G", "C").with_qual(50.0));
        builder.push(VariantRecord::new("chr2", 300, "AT", "A").with_qual(75.0)); // deletion
        builder.push(VariantRecord::new("chr2", 400, "C", "G").with_qual(30.0));

        VariantAnalytics::from_builder(&builder).expect("Failed to create analytics")
    }

    #[test]
    fn test_variant_count() {
        let analytics = create_test_analytics();
        assert_eq!(analytics.count(), 4);
    }

    #[test]
    fn test_count_by_chromosome() {
        let analytics = create_test_analytics();
        let counts = analytics.count_by_chromosome().unwrap();

        assert_eq!(counts.len(), 2);
        // Both chr1 and chr2 have 2 variants each
        // Polars group_by output order isn't guaranteed without sort, but we sorted desc
        // so it should be stable enough or we check content
        let count_map: std::collections::HashMap<_, _> = counts.into_iter().collect();
        assert_eq!(count_map.get("chr1"), Some(&2));
        assert_eq!(count_map.get("chr2"), Some(&2));
    }

    #[test]
    fn test_filter_by_quality() {
        let analytics = create_test_analytics();
        let count = analytics.filter_by_quality(60.0).unwrap();

        assert_eq!(count, 2); // quals 99 and 75
    }

    #[test]
    fn test_filter_by_region() {
        let analytics = create_test_analytics();
        let count = analytics.filter_by_region("chr1", 0, 250).unwrap();

        assert_eq!(count, 2); // chr1:100 and chr1:200
    }

    #[test]
    fn test_variant_type_counts() {
        let analytics = create_test_analytics();
        let (snps, indels) = analytics.variant_type_counts().unwrap();

        assert_eq!(snps, 3); // A>T, G>C, C>G
        assert_eq!(indels, 1); // AT>A
    }

    #[test]
    fn test_quality_stats() {
        let analytics = create_test_analytics();
        let stats = analytics.quality_stats().unwrap();

        assert_eq!(stats.count, 4);
        assert!((stats.mean - 63.5).abs() < 0.1); // (99+50+75+30)/4 = 63.5
        assert_eq!(stats.min, 30.0);
        assert_eq!(stats.max, 99.0);
    }
    #[test]
    fn test_empty_analytics() {
        let builder = VariantBatchBuilder::new();
        let analytics = VariantAnalytics::from_builder(&builder).unwrap();

        assert_eq!(analytics.count(), 0);
        assert_eq!(analytics.count_by_chromosome().unwrap().len(), 0);
        assert_eq!(analytics.quality_stats().unwrap().count, 0);
    }

    #[test]
    fn test_null_qualities() {
        let mut builder = VariantBatchBuilder::new();
        // Variant without quality (None)
        let mut record = VariantRecord::new("chr1", 100, "A", "T");
        record.qual = None;
        builder.push(record);

        // Variant with quality
        builder.push(VariantRecord::new("chr1", 200, "G", "C").with_qual(50.0));

        let analytics = VariantAnalytics::from_builder(&builder).unwrap();
        let stats = analytics.quality_stats().unwrap();

        assert_eq!(stats.count, 1); // Only 1 valid quality
        assert_eq!(stats.min, 50.0);
        assert_eq!(stats.max, 50.0);
    }
}
