use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
struct Idl {
    instructions: Vec<Instruction>,
}

#[derive(Debug, Deserialize)]
struct Instruction {
    name: String,
    discriminator: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IDL vs SDK Instruction Comparison ===\n");

    // Read IDL file
    let idl_path = "../target/idl/feels.json";
    let idl_content = fs::read_to_string(idl_path)?;
    let idl: Idl = serde_json::from_str(&idl_content)?;

    // Find and parse SDK instruction files
    let sdk_discriminators = find_all_sdk_discriminators("../sdk/src/instructions")?;
    
    println!("Found {} instructions in IDL", idl.instructions.len());
    println!("Found {} discriminators in SDK\n", sdk_discriminators.len());
    
    // Debug: Print all found SDK discriminators
    println!("## SDK Discriminators Found:");
    for (name, disc) in &sdk_discriminators {
        println!("- {}: {:?}", name, disc);
    }
    println!();

    let mut missing = Vec::new();
    let mut mismatches = Vec::new();
    let mut correct = Vec::new();

    // Compare each IDL instruction
    for instruction in &idl.instructions {
        match sdk_discriminators.get(&instruction.name) {
            None => missing.push(&instruction.name),
            Some(sdk_disc) => {
                if sdk_disc == &instruction.discriminator {
                    correct.push(&instruction.name);
                } else {
                    mismatches.push((
                        &instruction.name,
                        instruction.discriminator.clone(),
                        sdk_disc.clone()
                    ));
                }
            }
        }
    }

    // Print results
    println!("## Summary\n");
    println!("- Total IDL instructions: {}", idl.instructions.len());
    println!("- Missing in SDK: {}", missing.len());
    println!("- Discriminator mismatches: {}", mismatches.len());
    println!("- Correctly implemented: {}\n", correct.len());

    if !missing.is_empty() {
        println!("## Missing Instructions\n");
        for name in &missing {
            if let Some(inst) = idl.instructions.iter().find(|i| i.name == **name) {
                println!("- {} (discriminator: {:?})", name, inst.discriminator);
            }
        }
        println!();
    }

    if !mismatches.is_empty() {
        println!("## Discriminator Mismatches\n");
        for (name, idl_disc, sdk_disc) in &mismatches {
            println!("- {}", name);
            println!("  - IDL: {:?}", idl_disc);
            println!("  - SDK: {:?}", sdk_disc);
        }
        println!();
    }

    if !correct.is_empty() {
        println!("## Correctly Implemented\n");
        for name in &correct {
            println!("- {} ✓", name);
        }
        println!();
    }

    // Generate fixes
    if !missing.is_empty() || !mismatches.is_empty() {
        println!("## Fix Suggestions\n");
        
        // Group by file
        let mut fixes_by_file: HashMap<String, Vec<(String, Vec<u8>)>> = HashMap::new();
        
        for name in &missing {
            if let Some(inst) = idl.instructions.iter().find(|i| i.name == **name) {
                let file = guess_file_for_instruction(name);
                fixes_by_file.entry(file).or_default().push((
                    inst.name.clone(),
                    inst.discriminator.clone()
                ));
            }
        }
        
        for (name, idl_disc, _) in &mismatches {
            let file = guess_file_for_instruction(name);
            fixes_by_file.entry(file).or_default().push((
                name.to_string(),
                idl_disc.clone()
            ));
        }
        
        for (file, instructions) in &fixes_by_file {
            println!("// File: {}", file);
            for (name, disc) in instructions {
                let const_name = to_constant_case(name);
                let disc_str = format_discriminator(disc);
                println!("const {}_DISCRIMINATOR: [u8; 8] = {};", const_name, disc_str);
            }
            println!();
        }
    }

    Ok(())
}

fn find_all_sdk_discriminators(dir: &str) -> Result<HashMap<String, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut all_discriminators = HashMap::new();
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if path.file_name().unwrap().to_str().unwrap() == "mod.rs" {
                continue; // Skip mod.rs
            }
            
            let content = fs::read_to_string(&path)?;
            let discriminators = extract_discriminators(&content);
            
            println!("File: {} found {} discriminators", path.display(), discriminators.len());
            
            for (name, disc) in discriminators {
                all_discriminators.insert(name, disc);
            }
        }
    }
    
    Ok(all_discriminators)
}

fn extract_discriminators(content: &str) -> HashMap<String, Vec<u8>> {
    let mut discriminators = HashMap::new();
    
    for line in content.lines() {
        if line.contains("_DISCRIMINATOR: [u8; 8] = [") {
            // Extract the constant name
            let const_start = line.find("const ").map(|i| i + 6);
            let const_end = line.find("_DISCRIMINATOR");
            
            if let (Some(start), Some(end)) = (const_start, const_end) {
                let const_name = &line[start..end];
                let instruction_name = from_constant_case(const_name);
                
                // Extract discriminator values - find the last [ and first ] after it
                if let Some(arr_start) = line.rfind(" = [") {
                    if let Some(arr_end) = line[arr_start..].find(']') {
                        let arr_content = &line[arr_start + 4..arr_start + arr_end];
                        let values: Result<Vec<u8>, _> = arr_content
                            .split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .map(|s| parse_byte_value(s))
                            .collect();
                        
                        if let Ok(disc) = values {
                            if disc.len() == 8 {
                                println!("Found discriminator: {} -> {:?}", instruction_name, disc);
                                discriminators.insert(instruction_name, disc);
                            }
                        }
                    }
                }
            }
        }
    }
    
    discriminators
}

fn parse_byte_value(s: &str) -> Result<u8, std::num::ParseIntError> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u8::from_str_radix(&s[2..], 16)
    } else {
        s.parse::<u8>()
    }
}

fn format_discriminator(disc: &[u8]) -> String {
    format!("[{}]", 
        disc.iter()
            .map(|b| format!("{}", b))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn to_constant_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_lowercase = false;
    
    for ch in s.chars() {
        if ch == '_' {
            result.push('_');
            prev_lowercase = false;
        } else if ch.is_uppercase() && prev_lowercase {
            result.push('_');
            result.push(ch);
            prev_lowercase = false;
        } else {
            result.push(ch.to_uppercase().next().unwrap());
            prev_lowercase = ch.is_lowercase();
        }
    }
    
    result
}

fn from_constant_case(s: &str) -> String {
    s.to_lowercase().replace('_', "_")
}

fn guess_file_for_instruction(name: &str) -> String {
    match name {
        n if n.contains("protocol") => "protocol.rs".to_string(),
        n if n.contains("market") && !n.contains("initialize_market") => "market.rs".to_string(),
        n if n.contains("position") => "position.rs".to_string(),
        n if n.contains("pool") || n.contains("registry") => "registry.rs".to_string(),
        n if n.contains("pomm") => "pomm.rs".to_string(),
        n if n.contains("swap") => "swap.rs".to_string(),
        n if n.contains("feelssol") || n.contains("hub") || 
             n.contains("liquidity") || n.contains("fees") ||
             n.contains("initialize_market") || n.contains("mint_token") => "liquidity.rs".to_string(),
        _ => "unknown.rs".to_string(),
    }
}