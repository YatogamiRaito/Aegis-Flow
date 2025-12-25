//! Polars Analytics
//!
//! DataFrame analytics for genomic data using Polars.

use crate::variant::VariantBatchBuilder;

/// Variant analytics using Polars
pub struct VariantAnalytics {
    /// Chromosome column
    chroms: Vec<String>,
    /// Position column
    positions: Vec<i64>,
    /// Quality scores
    quals: Vec<Option<f64>>,
    /// Reference alleles
    refs: Vec<String>,
    /// Alternate alleles
    alts: Vec<String>,
}

impl VariantAnalytics {
    /// Create from VariantBatchBuilder
    pub fn from_builder(_builder: &VariantBatchBuilder) -> Self {
        // Extract data from builder - for now create simple stats
        Self {
            chroms: Vec::new(),
            positions: Vec::new(),
            quals: Vec::new(),
            refs: Vec::new(),
            alts: Vec::new(),
        }
    }

    /// Add a variant for analysis
    pub fn add_variant(
        &mut self,
        chrom: &str,
        pos: i64,
        reference: &str,
        alt: &str,
        qual: Option<f64>,
    ) {
        self.chroms.push(chrom.to_string());
        self.positions.push(pos);
        self.refs.push(reference.to_string());
        self.alts.push(alt.to_string());
        self.quals.push(qual);
    }

    /// Get total variant count
    pub fn count(&self) -> usize {
        self.chroms.len()
    }

    /// Count variants per chromosome
    pub fn count_by_chromosome(&self) -> Vec<(String, usize)> {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for chrom in &self.chroms {
            *counts.entry(chrom.clone()).or_insert(0) += 1;
        }
        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }

    /// Filter by quality threshold
    pub fn filter_by_quality(&self, min_qual: f64) -> Vec<usize> {
        self.quals
            .iter()
            .enumerate()
            .filter_map(|(i, q)| q.filter(|qual| *qual >= min_qual).map(|_| i))
            .collect()
    }

    /// Get variants in a region
    pub fn filter_by_region(&self, chrom: &str, start: i64, end: i64) -> Vec<usize> {
        self.chroms
            .iter()
            .zip(self.positions.iter())
            .enumerate()
            .filter_map(|(i, (c, p))| {
                if c == chrom && *p >= start && *p <= end {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Count SNPs vs INDELs
    pub fn variant_type_counts(&self) -> (usize, usize) {
        let mut snps = 0;
        let mut indels = 0;
        for (r, a) in self.refs.iter().zip(self.alts.iter()) {
            if r.len() == 1 && a.len() == 1 {
                snps += 1;
            } else {
                indels += 1;
            }
        }
        (snps, indels)
    }

    /// Get quality statistics
    pub fn quality_stats(&self) -> QualityStats {
        let valid_quals: Vec<f64> = self.quals.iter().filter_map(|q| *q).collect();
        if valid_quals.is_empty() {
            return QualityStats::default();
        }

        let sum: f64 = valid_quals.iter().sum();
        let count = valid_quals.len();
        let mean = sum / count as f64;
        let min = valid_quals.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = valid_quals
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        QualityStats {
            count,
            mean,
            min,
            max,
        }
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

    fn create_test_analytics() -> VariantAnalytics {
        let mut analytics = VariantAnalytics::from_builder(&crate::VariantBatchBuilder::new());
        analytics.add_variant("chr1", 100, "A", "T", Some(99.0));
        analytics.add_variant("chr1", 200, "G", "C", Some(50.0));
        analytics.add_variant("chr2", 300, "AT", "A", Some(75.0)); // deletion
        analytics.add_variant("chr2", 400, "C", "G", Some(30.0));
        analytics
    }

    #[test]
    fn test_variant_count() {
        let analytics = create_test_analytics();
        assert_eq!(analytics.count(), 4);
    }

    #[test]
    fn test_count_by_chromosome() {
        let analytics = create_test_analytics();
        let counts = analytics.count_by_chromosome();

        assert_eq!(counts.len(), 2);
        // Both chr1 and chr2 have 2 variants each
        assert!(counts.iter().all(|(_, c)| *c == 2));
    }

    #[test]
    fn test_filter_by_quality() {
        let analytics = create_test_analytics();
        let filtered = analytics.filter_by_quality(60.0);

        assert_eq!(filtered.len(), 2); // quals 99 and 75
    }

    #[test]
    fn test_filter_by_region() {
        let analytics = create_test_analytics();
        let region = analytics.filter_by_region("chr1", 0, 250);

        assert_eq!(region.len(), 2); // chr1:100 and chr1:200
    }

    #[test]
    fn test_variant_type_counts() {
        let analytics = create_test_analytics();
        let (snps, indels) = analytics.variant_type_counts();

        assert_eq!(snps, 3); // A>T, G>C, C>G
        assert_eq!(indels, 1); // AT>A
    }

    #[test]
    fn test_quality_stats() {
        let analytics = create_test_analytics();
        let stats = analytics.quality_stats();

        assert_eq!(stats.count, 4);
        assert!((stats.mean - 63.5).abs() < 0.1); // (99+50+75+30)/4 = 63.5
        assert_eq!(stats.min, 30.0);
        assert_eq!(stats.max, 99.0);
    }
}
