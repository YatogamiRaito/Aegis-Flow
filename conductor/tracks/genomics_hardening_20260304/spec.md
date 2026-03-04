# Track Specification: Genomic Data Processing Hardening

## Overview
Elevating the existing structural genomic parsers into a full-scale, networked, zero-copy analytics engine by implementing the missing Apache Arrow Flight Server, full BAM reading logic, and integrating it into the proxy architecture.

## Functional Requirements

### FR-1: Full BAM and FASTA Sequence Parsing
- Expand the current `bam_parser` to stream alignment records (not just headers) via `noodles` to Arrow batches.
- Implement the missing FASTA/FASTQ sequence parser for reference sequences.

### FR-2: Arrow Flight Server Implementation
- Introduce `arrow-flight` crate dependency.
- Create a gRPC-based Flight Service in `crates/genomics` to serve `FlightData` streams.
- Expose `DoGet` for retrieving variant DataFrames and `DoExchange` for uploading data.

### FR-3: Proxy Router Integration
- Expose an endpoint or gRPC reflection in `aegis-proxy` to route requests dynamically to the internal Flight server. 
- Enable security controls (ML-KEM/mTLS) for the Genomic data endpoint.

### FR-4: Kusursuzluk Fazı (Perfection Elements)
- Out-of-Core Processing: Polars lazy evaluation scaling out to on-disk memory mapping for files > RAM capacity.
- VCF INFO/FORMAT fields dynamically mapped to struct arrays instead of pure strings.
- SIMD optimization via specific Rust target features or polars configurations.

## Non-Functional Requirements

### NFR-1: Streaming Performance Benchmark
- Processing a 1GB BAM file MUST take < 5 seconds to convert into Arrow format and stream out.

### NFR-2: 10x Python Baseline Proof
- Setup a benchmark proving execution is >10x faster than pure python pandas.

## Acceptance Criteria
1. `arrow-flight` is successfully started on boot.
2. Proxy can route an incoming gRPC call to the genomic engine.
3. Fully functional streaming of 1GB BAM variants under the 5-second limit.
4. Python benchmark clearly succeeds.
