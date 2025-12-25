# Track Plan: Genomic Data Processing

## Phase 1: Arrow Foundation
- [x] Task: Create aegis-genomics crate
- [x] Task: Add arrow-rs and polars dependencies
- [x] Task: Define GenomicRecord Arrow schema
- [x] Task: Implement RecordBatch builder
- [x] Task: Conductor Verification 'Arrow Foundation'

## Phase 2: Format Parsers
- [x] Task: Add noodles dependency for BAM/VCF
- [x] Task: Implement BAM header parser
- [x] Task: Implement VCF record parser
- [x] Task: Convert to Arrow RecordBatch
- [x] Task: Conductor Verification 'Format Parsers'

## Phase 3: Polars Analytics
- [ ] Task: DataFrame from Arrow RecordBatch
- [ ] Task: Variant filtering operations
- [ ] Task: Aggregation (counts, stats)
- [ ] Task: Lazy evaluation for large files
- [ ] Task: Conductor Verification 'Polars Analytics'

## Phase 4: Release v0.8.0
- [ ] Task: Documentation update
- [ ] Task: Arrow Flight endpoint (optional)
- [ ] Task: Release v0.8.0
- [ ] Task: Conductor Verification 'Release v0.8.0'
