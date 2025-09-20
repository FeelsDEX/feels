use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Idl {
    instructions: Vec<Instruction>,
}

#[derive(Debug, Deserialize)]
struct Instruction {
    name: String,
    discriminator: Vec<u8>,
    accounts: Vec<Account>,
    args: Vec<Arg>,
}

#[derive(Debug, Deserialize)]
struct Account {
    name: String,
    writable: Option<bool>,
    signer: Option<bool>,
    optional: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Arg {
    name: String,
    #[serde(rename = "type")]
    arg_type: serde_json::Value,
}

#[derive(Debug)]
struct ComparisonResult {
    instruction_name: String,
    idl_discriminator: Vec<u8>,
    sdk_discriminator: Option<Vec<u8>>,
    missing_in_sdk: bool,
    discriminator_mismatch: bool,
    account_count_mismatch: Option<(usize, usize)>, // (idl_count, sdk_count)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IDL vs SDK Instruction Comparison ===\n");

    // Read IDL file
    let idl_path = "../target/idl/feels.json";
    let idl_content = fs::read_to_string(idl_path)?;
    let idl: Idl = serde_json::from_str(&idl_content)?;

    // Find all SDK instruction files
    let sdk_instructions = find_sdk_instructions("../sdk/src/instructions")?;
    
    println!("Found {} instructions in IDL", idl.instructions.len());
    println!("Found {} SDK instruction files\n", sdk_instructions.len());

    let mut results = Vec::new();
    let mut sdk_discriminators = HashMap::new();

    // Extract discriminators from SDK files
    for (file_path, content) in &sdk_instructions {
        let discriminators = extract_discriminators(&content);
        for (name, disc) in discriminators {
            sdk_discriminators.insert(name, disc);
        }
    }

    // Compare each IDL instruction
    for instruction in &idl.instructions {
        let sdk_disc = sdk_discriminators.get(&instruction.name).cloned();
        
        let result = ComparisonResult {
            instruction_name: instruction.name.clone(),
            idl_discriminator: instruction.discriminator.clone(),
            sdk_discriminator: sdk_disc.clone(),
            missing_in_sdk: sdk_disc.is_none(),
            discriminator_mismatch: sdk_disc.as_ref().map_or(false, |d| d != &instruction.discriminator),
            account_count_mismatch: None, // TODO: Implement account comparison
        };
        
        results.push(result);
    }

    // Print results
    println!("## Summary\n");
    
    let missing_count = results.iter().filter(|r| r.missing_in_sdk).count();
    let mismatch_count = results.iter().filter(|r| r.discriminator_mismatch).count();
    
    println!("- Total IDL instructions: {}", idl.instructions.len());
    println!("- Missing in SDK: {}", missing_count);
    println!("- Discriminator mismatches: {}", mismatch_count);
    println!("- Correctly implemented: {}\n", idl.instructions.len() - missing_count - mismatch_count);

    // Detailed results
    println!("## Missing Instructions\n");
    for result in results.iter().filter(|r| r.missing_in_sdk) {
        println!("- {} (discriminator: {:?})", result.instruction_name, result.idl_discriminator);
    }

    println!("\n## Discriminator Mismatches\n");
    for result in results.iter().filter(|r| r.discriminator_mismatch) {
        println!("- {}", result.instruction_name);
        println!("  - IDL: {:?}", result.idl_discriminator);
        println!("  - SDK: {:?}", result.sdk_discriminator.as_ref().unwrap());
    }

    println!("\n## Correctly Implemented\n");
    for result in results.iter().filter(|r| !r.missing_in_sdk && !r.discriminator_mismatch) {
        println!("- {} ✓", result.instruction_name);
    }

    // Find extra instructions in SDK not in IDL
    println!("\n## Extra Instructions in SDK (not in IDL)\n");
    let idl_names: HashSet<_> = idl.instructions.iter().map(|i| i.name.clone()).collect();
    for sdk_name in sdk_discriminators.keys() {
        if !idl_names.contains(sdk_name) {
            println!("- {} (discriminator: {:?})", sdk_name, sdk_discriminators[sdk_name]);
        }
    }

    // Generate fix suggestions
    println!("\n## Fix Suggestions\n");
    for result in results.iter().filter(|r| r.missing_in_sdk || r.discriminator_mismatch) {
        let disc_str = format_discriminator(&result.idl_discriminator);
        println!("// {}", result.instruction_name);
        println!("const {}_DISCRIMINATOR: [u8; 8] = {};", 
            to_constant_case(&result.instruction_name), 
            disc_str
        );
        println!();
    }

    Ok(())
}

fn find_sdk_instructions(dir: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut files = HashMap::new();
    
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)?;
            files.insert(path.to_string_lossy().to_string(), content);
        }
    }
    
    Ok(files)
}

fn extract_discriminators(content: &str) -> HashMap<String, Vec<u8>> {
    let mut discriminators = HashMap::new();
    
    // Pattern to match discriminator constants
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("_DISCRIMINATOR: [u8; 8] = [") {
            // Extract instruction name
            let parts: Vec<&str> = line.split("const ").collect();
            if parts.len() < 2 { continue; }
            
            let name_part = parts[1].split("_DISCRIMINATOR").next().unwrap_or("");
            let instruction_name = from_constant_case(name_part);
            
            // Extract discriminator values
            if let Some(start) = line.find('[') {
                if let Some(end) = line.rfind(']') {
                    let disc_str = &line[start+1..end];
                    let values: Result<Vec<u8>, _> = disc_str
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| {
                            // Handle both decimal and hex formats
                            if s.starts_with("0x") {
                                u8::from_str_radix(&s[2..], 16)
                            } else {
                                s.parse::<u8>()
                            }
                        })
                        .collect();
                    
                    if let Ok(disc) = values {
                        if disc.len() == 8 {
                            discriminators.insert(instruction_name, disc);
                        }
                    }
                }
            }
        }
    }
    
    discriminators
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
    
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && prev_lowercase {
            result.push('_');
        }
        result.push(ch.to_uppercase().next().unwrap());
        prev_lowercase = ch.is_lowercase();
    }
    
    result
}

fn from_constant_case(s: &str) -> String {
    s.to_lowercase()
        .split('_')
        .collect::<Vec<_>>()
        .join("_")
}