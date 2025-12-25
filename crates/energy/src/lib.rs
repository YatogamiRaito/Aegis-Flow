//! Aegis Energy - Carbon-Aware Energy API Integration
//!
//! This crate provides integration with energy grid APIs (WattTime, Electricity Maps)
//! for carbon-aware traffic routing in Aegis-Flow.

mod cache;
mod client;
mod types;

pub use cache::CarbonIntensityCache;
pub use client::{ElectricityMapsClient, EnergyApiClient, WattTimeClient};
pub use types::{CarbonIntensity, EnergyApiError, EnergyApiProvider, Region};
