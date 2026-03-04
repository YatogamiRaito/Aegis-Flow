# Track Plan: Genomic Data Processing Hardening

## Phase 1: Full Parsing Implementations
- [ ] Task: Extend `bam_parser.rs` to parse Alignment bodies using noodles.
- [ ] Task: Create `fasta_parser.rs` for sequence records.
- [ ] Task: Conductor Verification 'Full Parsing Implementations'

## Phase 2: Arrow Flight Server Integration
- [ ] Task: Add `arrow-flight` and `tonic` dependencies to `crates/genomics`.
- [ ] Task: Implement `FlightService` trait (ticket system, `DoGet`, `DoExchange`).
- [ ] Task: Conductor Verification 'Arrow Flight Server Integration'

## Phase 3: Proxy Router & gRPC Exposure
- [ ] Task: Initialize Genomic Flight Server within Aegis Proxy boot sequence.
- [ ] Task: Bind server closely with Proxy's internal networking layout.
- [ ] Task: Conductor Verification 'Proxy Router & gRPC Exposure'

## Phase 4: Benchmarks & 10x Validation
- [ ] Task: Implement streaming benchmark for 1GB BAM parsing.
- [ ] Task: Implement Pandas/Python comparison baseline test.
- [ ] Task: Conductor Verification 'Benchmarks & 10x Validation'

## Phase 4.5: Kusursuzluk Fazı
- [ ] Task: VCF Array-typed INFO/FORMAT field dynamic extraction.
- [ ] Task: Memory mapped out-of-core evaluation tests for Polars (multi-GB scenario).
- [ ] Task: SIMD compilation tuning flags and validations.
- [ ] Task: Conductor Verification 'Kusursuzluk Fazı'

## Phase 5: Finalization
- [ ] Task: Documentation update
- [ ] Task: Release v0.36.0
- [ ] Task: Conductor Verification 'Finalization'
