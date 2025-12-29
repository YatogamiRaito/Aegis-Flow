//! BAM Header Parser
//!
//! Parses BAM file headers for reference information.

use crate::{GenomicsError, Result};
use std::collections::HashMap;
use tracing::{debug, info};

/// BAM file header information
#[derive(Debug, Clone, Default)]
pub struct BamHeader {
    /// Version
    pub version: Option<String>,
    /// Sorting order
    pub sort_order: Option<String>,
    /// Reference sequences
    pub references: Vec<ReferenceSequence>,
    /// Read groups
    pub read_groups: Vec<ReadGroup>,
    /// Programs
    pub programs: Vec<Program>,
}

/// Reference sequence from BAM header
#[derive(Debug, Clone)]
pub struct ReferenceSequence {
    /// Sequence name
    pub name: String,
    /// Sequence length
    pub length: u64,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

/// Read group from BAM header
#[derive(Debug, Clone)]
pub struct ReadGroup {
    /// Read group ID
    pub id: String,
    /// Sample name
    pub sample: Option<String>,
    /// Library
    pub library: Option<String>,
    /// Platform
    pub platform: Option<String>,
}

/// Program record from BAM header
#[derive(Debug, Clone)]
pub struct Program {
    /// Program ID
    pub id: String,
    /// Program name
    pub name: Option<String>,
    /// Command line
    pub command_line: Option<String>,
    /// Version
    pub version: Option<String>,
}

impl BamHeader {
    /// Create a new empty header
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse header from SAM text format
    pub fn from_sam_text(text: &str) -> Result<Self> {
        let mut header = BamHeader::new();

        for line in text.lines() {
            if line.starts_with("@HD") {
                header.parse_hd_line(line)?;
            } else if line.starts_with("@SQ") {
                header.parse_sq_line(line)?;
            } else if line.starts_with("@RG") {
                header.parse_rg_line(line)?;
            } else if line.starts_with("@PG") {
                header.parse_pg_line(line)?;
            }
        }

        info!(
            "Parsed BAM header: {} references, {} read groups",
            header.references.len(),
            header.read_groups.len()
        );

        Ok(header)
    }

    fn parse_hd_line(&mut self, line: &str) -> Result<()> {
        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "VN" => self.version = Some(value.to_string()),
                    "SO" => self.sort_order = Some(value.to_string()),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn parse_sq_line(&mut self, line: &str) -> Result<()> {
        let mut name = String::new();
        let mut length: u64 = 0;
        let mut attributes = HashMap::new();

        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "SN" => name = value.to_string(),
                    "LN" => {
                        length = value.parse().map_err(|_| {
                            GenomicsError::ParseError("Invalid sequence length".to_string())
                        })?
                    }
                    _ => {
                        attributes.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        if !name.is_empty() {
            self.references.push(ReferenceSequence {
                name,
                length,
                attributes,
            });
            debug!(
                "Parsed reference: {} ({})",
                self.references.last().unwrap().name,
                length
            );
        }

        Ok(())
    }

    fn parse_rg_line(&mut self, line: &str) -> Result<()> {
        let mut id = String::new();
        let mut sample = None;
        let mut library = None;
        let mut platform = None;

        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "ID" => id = value.to_string(),
                    "SM" => sample = Some(value.to_string()),
                    "LB" => library = Some(value.to_string()),
                    "PL" => platform = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if !id.is_empty() {
            self.read_groups.push(ReadGroup {
                id,
                sample,
                library,
                platform,
            });
        }

        Ok(())
    }

    fn parse_pg_line(&mut self, line: &str) -> Result<()> {
        let mut id = String::new();
        let mut name = None;
        let mut command_line = None;
        let mut version = None;

        for field in line.split('\t').skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                match key {
                    "ID" => id = value.to_string(),
                    "PN" => name = Some(value.to_string()),
                    "CL" => command_line = Some(value.to_string()),
                    "VN" => version = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if !id.is_empty() {
            self.programs.push(Program {
                id,
                name,
                command_line,
                version,
            });
        }

        Ok(())
    }

    /// Get total reference length
    pub fn total_length(&self) -> u64 {
        self.references.iter().map(|r| r.length).sum()
    }

    /// Get reference by name
    pub fn get_reference(&self, name: &str) -> Option<&ReferenceSequence> {
        self.references.iter().find(|r| r.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sam_header() {
        let header_text = r"@HD	VN:1.6	SO:coordinate
@SQ	SN:chr1	LN:248956422
@SQ	SN:chr2	LN:242193529
@RG	ID:sample1	SM:NA12878	PL:ILLUMINA
@PG	ID:bwa	PN:bwa	VN:0.7.17	CL:bwa mem ref.fa reads.fq";

        let header = BamHeader::from_sam_text(header_text).unwrap();

        assert_eq!(header.version, Some("1.6".to_string()));
        assert_eq!(header.sort_order, Some("coordinate".to_string()));
        assert_eq!(header.references.len(), 2);
        assert_eq!(header.references[0].name, "chr1");
        assert_eq!(header.references[0].length, 248956422);
        assert_eq!(header.read_groups.len(), 1);
        assert_eq!(header.programs.len(), 1);
    }

    #[test]
    fn test_total_length() {
        let header_text = "@SQ	SN:chr1	LN:1000\n@SQ	SN:chr2	LN:2000";
        let header = BamHeader::from_sam_text(header_text).unwrap();

        assert_eq!(header.total_length(), 3000);
    }

    #[test]
    fn test_get_reference() {
        let header_text = "@SQ	SN:chr1	LN:1000\n@SQ	SN:chr2	LN:2000";
        let header = BamHeader::from_sam_text(header_text).unwrap();

        let chr1 = header.get_reference("chr1").unwrap();
        assert_eq!(chr1.length, 1000);
    }

    #[test]
    fn test_invalid_sq_length() {
        let header_text = "@SQ	SN:chr1	LN:invalid";
        let result = BamHeader::from_sam_text(header_text);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_sn() {
        // Technically this parser might allow empty name if SN is missing,
        // but let's check what it does.
        // Implementation check: name defaults to empty string, and it ONLY pushes if !name.is_empty()
        let header_text = "@SQ	LN:1000";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        assert_eq!(header.references.len(), 0);
    }

    #[test]
    fn test_parse_header_unknown_attributes() {
        // Just test the internal logic by feeding text directly (since BamParser::parse_header creates magic+size logic)
        // We can simulate the text parsing part via from_sam_text which uses the same internal helpers

        let text = "@HD\tVN:1.6\tSO:coordinate\t\n\
                    @SQ\tSN:chr1\tLN:1000\tX1:unknown\n\
                    @RG\tID:rg1\tX2:unknown\n";

        let header = BamHeader::from_sam_text(text).unwrap();

        assert_eq!(header.version, Some("1.6".to_string()));
        assert_eq!(header.references.len(), 1);
        assert_eq!(
            header.references[0]
                .attributes
                .get("X1")
                .map(|s| s.as_str()),
            Some("unknown")
        );

        assert_eq!(header.read_groups.len(), 1);
    }

    #[test]
    fn test_parse_pg_line() {
        let text = "@PG\tID:bwa\tPN:bwa\tVN:0.7.17\tCL:bwa mem ref.fa read.fq";
        let header = BamHeader::from_sam_text(text).unwrap();
        assert_eq!(header.programs.len(), 1);
        assert_eq!(header.programs[0].id, "bwa");
        assert_eq!(header.programs[0].name, Some("bwa".to_string()));
        assert_eq!(header.programs[0].version, Some("0.7.17".to_string()));
    }

    #[test]
    fn test_parse_rg_with_all_fields() {
        let text = "@RG\tID:sample1\tSM:sample\tLB:lib1\tPL:ILLUMINA";
        let header = BamHeader::from_sam_text(text).unwrap();
        assert_eq!(header.read_groups.len(), 1);
        let rg = &header.read_groups[0];
        assert_eq!(rg.id, "sample1");
        assert_eq!(rg.sample, Some("sample".to_string()));
        assert_eq!(rg.library, Some("lib1".to_string()));
        assert_eq!(rg.platform, Some("ILLUMINA".to_string()));
    }

    #[test]
    fn test_bam_header_clone() {
        let text = "@HD\tVN:1.6\n@SQ\tSN:chr1\tLN:1000";
        let header = BamHeader::from_sam_text(text).unwrap();
        let cloned = header.clone();
        assert_eq!(header.version, cloned.version);
        assert_eq!(header.references.len(), cloned.references.len());
    }

    #[test]
    fn test_bam_header_debug() {
        let header = BamHeader::new();
        let debug_str = format!("{:?}", header);
        assert!(debug_str.contains("BamHeader"));
    }

    #[test]
    fn test_bam_header_new() {
        let header = BamHeader::new();
        assert_eq!(header.references.len(), 0);
        assert_eq!(header.read_groups.len(), 0);
        assert_eq!(header.programs.len(), 0);
        assert_eq!(header.total_length(), 0);
    }

    #[test]
    fn test_get_reference_not_found() {
        let header_text = "@SQ\tSN:chr1\tLN:1000";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        assert!(header.get_reference("chr99").is_none());
    }

    #[test]
    fn test_reference_sequence_debug() {
        let header_text = "@SQ\tSN:chr1\tLN:1000";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        let ref_seq = &header.references[0];
        let debug_str = format!("{:?}", ref_seq);
        assert!(debug_str.contains("ReferenceSequence"));
    }

    #[test]
    fn test_read_group_debug() {
        let header_text = "@RG\tID:rg1\tSM:sample";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        let rg = &header.read_groups[0];
        let debug_str = format!("{:?}", rg);
        assert!(debug_str.contains("ReadGroup"));
    }

    #[test]
    fn test_program_debug() {
        let header_text = "@PG\tID:bwa\tPN:bwa";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        let prog = &header.programs[0];
        let debug_str = format!("{:?}", prog);
        assert!(debug_str.contains("Program"));
    }

    #[test]
    fn test_reference_sequence_clone() {
        let header_text = "@SQ\tSN:chr1\tLN:1000";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        let ref1 = &header.references[0];
        let ref2 = ref1.clone();
        assert_eq!(ref1.name, ref2.name);
        assert_eq!(ref1.length, ref2.length);
    }

    #[test]
    fn test_read_group_clone() {
        let header_text = "@RG\tID:rg1\tSM:sample";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        let rg1 = &header.read_groups[0];
        let rg2 = rg1.clone();
        assert_eq!(rg1.id, rg2.id);
    }

    #[test]
    fn test_program_clone() {
        let header_text = "@PG\tID:prog1\tPN:program\tVN:1.0";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        let prog1 = &header.programs[0];
        let prog2 = prog1.clone();
        assert_eq!(prog1.id, prog2.id);
        assert_eq!(prog1.name, prog2.name);
    }

    #[test]
    fn test_parse_rg_without_id() {
        // RG without ID should not be added
        let header_text = "@RG\tSM:sample\tLB:lib";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        assert_eq!(header.read_groups.len(), 0);
    }

    #[test]
    fn test_parse_pg_without_id() {
        // PG without ID should not be added
        let header_text = "@PG\tPN:progname\tVN:1.0";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        assert_eq!(header.programs.len(), 0);
    }

    #[test]
    fn test_parse_hd_with_extra_fields() {
        // @HD with extra unknown fields should still parse
        let header_text = "@HD\tVN:1.6\tSO:coordinate\tFO:unknown\tXX:extra";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        assert_eq!(header.version, Some("1.6".to_string()));
        assert_eq!(header.sort_order, Some("coordinate".to_string()));
    }

    #[test]
    fn test_parse_sq_with_extra_attributes() {
        // @SQ with multiple extra attributes
        let header_text = "@SQ\tSN:chr1\tLN:1000\tAS:assembly\tM5:checksum\tSP:species";
        let header = BamHeader::from_sam_text(header_text).unwrap();
        assert_eq!(header.references.len(), 1);
        let ref_seq = &header.references[0];
        assert_eq!(ref_seq.attributes.get("AS"), Some(&"assembly".to_string()));
        assert_eq!(ref_seq.attributes.get("M5"), Some(&"checksum".to_string()));
    }

    #[test]
    fn test_bam_header_default() {
        let header: BamHeader = Default::default();
        assert_eq!(header.version, None);
        assert_eq!(header.sort_order, None);
        assert_eq!(header.references.len(), 0);
        assert_eq!(header.read_groups.len(), 0);
        assert_eq!(header.programs.len(), 0);
    }
    #[test]
    fn test_parse_header_unknown_tags() {
        let text = "@HD\tVN:1.6\tOO:unknown\n\
                    @SQ\tSN:k1\tLN:10\tXX:x\n\
                    @RG\tID:r1\tZZ:z\n\
                    @PG\tID:p1\tYY:y";

        let header = BamHeader::from_sam_text(text).unwrap();

        // HD
        assert_eq!(header.version.as_deref(), Some("1.6"));

        // SQ
        let sq = &header.references[0];
        assert_eq!(sq.attributes.get("XX").map(|s| s.as_str()), Some("x"));

        // RG - standard fields only captured, others ignored in struct but shouldn't error
        let rg = &header.read_groups[0];
        assert_eq!(rg.id, "r1");

        // PG - standard fields only captured
        let pg = &header.programs[0];
        assert_eq!(pg.id, "p1");
    }
}
