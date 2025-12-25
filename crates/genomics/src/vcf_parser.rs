//! VCF Parser
//!
//! Parses VCF files into Arrow RecordBatches.

use crate::variant::{VariantBatchBuilder, VariantRecord};
use crate::{GenomicsError, Result};
use std::io::BufRead;
use tracing::{debug, info};

/// VCF file parser
#[derive(Debug, Default)]
pub struct VcfParser;

impl VcfParser {
    /// Create a new VCF parser
    pub fn new() -> Self {
        Self
    }

    /// Parse VCF from a reader
    pub fn parse<R: BufRead>(&self, reader: R) -> Result<VariantBatchBuilder> {
        let mut builder = VariantBatchBuilder::new();
        let mut line_count = 0;
        let mut variant_count = 0;

        for line in reader.lines() {
            let line = line?;
            line_count += 1;

            // Skip header lines
            if line.starts_with('#') {
                continue;
            }

            // Parse variant record
            if let Some(record) = self.parse_line(&line)? {
                builder.push(record);
                variant_count += 1;
            }
        }

        info!(
            "Parsed {} lines, {} variants from VCF",
            line_count, variant_count
        );

        Ok(builder)
    }

    /// Parse a single VCF line
    fn parse_line(&self, line: &str) -> Result<Option<VariantRecord>> {
        let fields: Vec<&str> = line.split('\t').collect();

        if fields.len() < 8 {
            return Err(GenomicsError::InvalidFormat(format!(
                "VCF line has {} fields, expected >= 8",
                fields.len()
            )));
        }

        let chrom = fields[0];
        let pos: i64 = fields[1]
            .parse()
            .map_err(|_| GenomicsError::ParseError("Invalid position".to_string()))?;
        let id = if fields[2] == "." {
            None
        } else {
            Some(fields[2].to_string())
        };
        let reference = fields[3];
        let alternate = fields[4];
        let qual: Option<f64> = if fields[5] == "." {
            None
        } else {
            fields[5].parse().ok()
        };
        let filter = if fields[6] == "." {
            None
        } else {
            Some(fields[6].to_string())
        };
        let info = if fields[7] == "." {
            None
        } else {
            Some(fields[7].to_string())
        };

        let mut record = VariantRecord::new(chrom, pos, reference, alternate);
        if let Some(id) = id {
            record = record.with_id(&id);
        }
        if let Some(qual) = qual {
            record = record.with_qual(qual);
        }
        if let Some(filter) = filter {
            record = record.with_filter(&filter);
        }
        record.info = info;

        debug!(
            "Parsed variant: {}:{} {}/{}",
            chrom, pos, reference, alternate
        );

        Ok(Some(record))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_simple_vcf() {
        let vcf_data = r#"##fileformat=VCFv4.2
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	100	rs123	A	T	99.0	PASS	DP=50
chr1	200	.	G	C	50.5	.	DP=30
chr2	300	rs456	C	G	.	LowQual	.
"#;

        let reader = Cursor::new(vcf_data);
        let parser = VcfParser::new();
        let builder = parser.parse(reader).unwrap();

        assert_eq!(builder.len(), 3);

        let batch = builder.build().unwrap();
        assert_eq!(batch.num_rows(), 3);
    }

    #[test]
    fn test_parse_variant_with_id() {
        let vcf_data =
            "#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO\nchr1	100	rs12345	A	G	99.0	PASS	DP=100";
        let reader = Cursor::new(vcf_data);
        let parser = VcfParser::new();
        let builder = parser.parse(reader).unwrap();

        assert_eq!(builder.len(), 1);
    }

    #[test]
    fn test_skip_header_lines() {
        let vcf_data = r#"##fileformat=VCFv4.2
##INFO=<ID=DP,Number=1,Type=Integer>
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO
chr1	100	.	A	T	99.0	PASS	DP=50
"#;

        let reader = Cursor::new(vcf_data);
        let parser = VcfParser::new();
        let builder = parser.parse(reader).unwrap();

        assert_eq!(builder.len(), 1);
    }
}
