use crate::Cli;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("wpw").join("config.toml")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".config").join("wpw").join("config.toml")
    }
}

fn load_config() -> toml::Value {
    let path = config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or(toml::Value::Table(toml::map::Map::new()))
    } else {
        toml::Value::Table(toml::map::Map::new())
    }
}

fn save_config(config: &toml::Value) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml::to_string_pretty(config)?)?;
    Ok(())
}

pub fn run_set(_cli: &Cli, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = load_config();
    
    // Support dotted keys like "kdf.m_cost"
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = &mut config;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Set the value - try to parse as number first
            if let Ok(n) = value.parse::<i64>() {
                current[part] = toml::Value::Integer(n);
            } else if let Ok(f) = value.parse::<f64>() {
                current[part] = toml::Value::Float(f);
            } else if value == "true" || value == "false" {
                current[part] = toml::Value::Boolean(value == "true");
            } else {
                current[part] = toml::Value::String(value.to_string());
            }
        } else {
            if current.get(part).is_none() {
                current[part] = toml::Value::Table(toml::map::Map::new());
            }
            current = current.get_mut(part).unwrap();
        }
    }
    
    save_config(&config)?;
    println!("Set {} = {}", key, value);
    Ok(())
}

pub fn run_get(_cli: &Cli, key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = &config;
    for part in &parts {
        match current.get(part) {
            Some(v) => current = v,
            None => {
                println!("Key '{}' not found", key);
                return Ok(());
            }
        }
    }
    
    match current {
        toml::Value::String(s) => println!("{}", s),
        toml::Value::Integer(n) => println!("{}", n),
        toml::Value::Float(f) => println!("{}", f),
        toml::Value::Boolean(b) => println!("{}", b),
        _ => println!("{}", current),
    }
    
    Ok(())
}
