// Build script to ensure rebuild when environment variables change
// This ensures that changes to TOTAL_POWER_BUDGET trigger a rebuild

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if the environment variable changes
    println!("cargo:rerun-if-env-changed=TOTAL_POWER_BUDGET");

    // Also rerun if the .cargo/config.toml file changes
    println!("cargo:rerun-if-changed=.cargo/config.toml");

    // Read the current value from .cargo/config.toml
    let config_path = ".cargo/config.toml";
    let config_value = read_power_budget_from_config(config_path);

    // Get the environment variable value
    let env_value = env::var("TOTAL_POWER_BUDGET").ok();

    // Print current configuration for debugging
    match (config_value, env_value) {
        (Some(config), Some(env_val)) => {
            println!(
                "cargo:warning=Config file TOTAL_POWER_BUDGET={config}, Environment TOTAL_POWER_BUDGET={env_val}"
            );
            if config != env_val {
                println!("cargo:warning=Environment variable overrides config file value");
            }
        }
        (Some(config), None) => {
            println!("cargo:warning=Using TOTAL_POWER_BUDGET={config} from config file");
        }
        (None, Some(env_val)) => {
            println!("cargo:warning=Using TOTAL_POWER_BUDGET={env_val} from environment");
        }
        (None, None) => {
            println!("cargo:warning=Using default TOTAL_POWER_BUDGET=100");
        }
    }

    // Force rebuild if config file timestamp is newer than last build
    if Path::new(config_path).exists() {
        println!("cargo:rerun-if-changed={config_path}");
    }
}

fn read_power_budget_from_config(config_path: &str) -> Option<String> {
    if let Ok(content) = fs::read_to_string(config_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("TOTAL_POWER_BUDGET") && line.contains('=') {
                let value_part = line.split('=').nth(1)?;
                let value = value_part.trim().trim_matches('"').trim_matches('\'');
                return Some(value.to_string());
            }
        }
    }
    None
}
