//! Carbon-Aware Routing Example
//!
//! Demonstrates carbon intensity-based routing decisions.
//!
//! Run with: cargo run --example carbon_routing

use aegis_energy::{CarbonIntensity, CarbonIntensityCache, Region};
use chrono::Utc;

fn main() {
    println!("🌱 Aegis-Flow Carbon-Aware Routing Demo\n");

    // Create regions with mock carbon intensity data
    let regions = vec![
        ("us-west-2", "US West (Oregon)", 45.0), // Very green (hydro)
        ("eu-west-1", "EU West (Ireland)", 120.0), // Moderate (wind mix)
        ("us-east-1", "US East (Virginia)", 350.0), // High carbon (coal mix)
        ("ap-south-1", "Asia Pacific (Mumbai)", 600.0), // Very high carbon
    ];

    println!("1. Current Carbon Intensity by Region:\n");
    println!(
        "   {:<12} {:<25} {:>10} {:>12}",
        "Region", "Name", "gCO2/kWh", "Rating"
    );
    println!("   {}", "-".repeat(60));

    for (id, name, intensity) in &regions {
        let rating = match *intensity as u32 {
            0..=50 => "🟢 Very Low",
            51..=150 => "🟡 Low",
            151..=300 => "🟠 Medium",
            301..=500 => "🔴 High",
            _ => "⚫ Very High",
        };
        println!(
            "   {:<12} {:<25} {:>10.1} {:>12}",
            id, name, intensity, rating
        );
    }

    // Select greenest region
    println!("\n2. Route Selection:");
    let greenest = regions
        .iter()
        .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
        .unwrap();
    println!(
        "   ✅ Selected: {} ({:.1} gCO2/kWh)",
        greenest.1, greenest.2
    );
    println!("   📍 Region ID: {}", greenest.0);

    // Calculate routing weights
    println!("\n3. Weighted Load Balancing:");
    let max_intensity = 600.0;
    for (id, _, intensity) in &regions {
        let weight = ((1.0 - intensity / max_intensity) * 100.0) as u32;
        let bar = "█".repeat((weight / 5) as usize);
        println!("   {:<12} {:>3}% {}", id, weight, bar);
    }

    // Carbon savings estimate
    println!("\n4. Carbon Savings Estimate:");
    let requests_per_hour = 10_000;
    let joules_per_request = 0.5;
    let high_carbon = 350.0; // If we used us-east-1
    let low_carbon = 45.0; // Using us-west-2

    let kwh = (requests_per_hour as f64 * joules_per_request) / 3_600_000.0;
    let saved_grams = kwh * (high_carbon - low_carbon);

    println!("   Requests/hour: {}", requests_per_hour);
    println!("   Energy/hour: {:.4} kWh", kwh);
    println!("   Carbon saved: {:.2} gCO2/hour", saved_grams);
    println!(
        "   Monthly savings: {:.2} kgCO2",
        saved_grams * 24.0 * 30.0 / 1000.0
    );

    println!("\n🎉 Carbon-aware routing demo complete!");
}
