//! Aegis Energy - Carbon-Aware Energy API Integration
//!
//! This crate provides integration with energy grid APIs (WattTime, Electricity Maps)
//! for carbon-aware traffic routing in Aegis-Flow.

mod client;
mod cache;
mod types;

pub use client::{EnergyApiClient, WattTimeClient, ElectricityMapsClient};
pub use cache::CarbonIntensityCache;
pub use types::{CarbonIntensity, Region, EnergyApiError, EnergyApiProvider};
