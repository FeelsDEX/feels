/// Example of using the hub-constrained router
use feels_sdk::{HubRouter, PoolInfo};
use solana_program::pubkey::Pubkey;

fn main() {
    // FeelsSOL mint (hub token)
    let feelssol_mint = Pubkey::new_unique();
    
    // Other tokens
    let usdc_mint = Pubkey::new_unique();
    let sol_mint = Pubkey::new_unique();
    let jitosol_mint = Pubkey::new_unique();
    
    // Create router
    let mut router = HubRouter::new(feelssol_mint);
    
    // Add pools (all must include FeelsSOL)
    
    // USDC-FeelsSOL pool
    let usdc_feelssol_pool = PoolInfo {
        address: Pubkey::new_unique(),
        token_a: usdc_mint,
        token_b: feelssol_mint,
        fee_rate: 30, // 0.3%
    };
    router.add_pool(usdc_feelssol_pool).unwrap();
    
    // SOL-FeelsSOL pool
    let sol_feelssol_pool = PoolInfo {
        address: Pubkey::new_unique(),
        token_a: sol_mint,
        token_b: feelssol_mint,
        fee_rate: 25, // 0.25%
    };
    router.add_pool(sol_feelssol_pool).unwrap();
    
    // JitoSOL-FeelsSOL pool (entry/exit)
    let jitosol_feelssol_pool = PoolInfo {
        address: Pubkey::new_unique(),
        token_a: jitosol_mint,
        token_b: feelssol_mint,
        fee_rate: 10, // 0.1% for entry/exit
    };
    router.add_pool(jitosol_feelssol_pool).unwrap();
    
    // Example 1: Direct route (USDC -> FeelsSOL)
    println!("=== Direct Route Example ===");
    let route1 = router.find_route(&usdc_mint, &feelssol_mint).unwrap();
    println!("Route: {}", router.get_route_summary(&route1));
    println!("Hops: {}", route1.hops);
    println!("Uses hub: {}", route1.uses_hub);
    
    // Example 2: Two-hop route (USDC -> SOL via FeelsSOL)
    println!("\n=== Two-Hop Route Example ===");
    let route2 = router.find_route(&usdc_mint, &sol_mint).unwrap();
    println!("Route: {}", router.get_route_summary(&route2));
    println!("Hops: {}", route2.hops);
    println!("Pools:");
    for (i, pool) in route2.pools.iter().enumerate() {
        println!("  Hop {}: {} <-> {}", 
            i + 1,
            pool.token_a.to_string()[..8].to_string(),
            pool.token_b.to_string()[..8].to_string()
        );
    }
    
    // Example 3: Validate external route
    println!("\n=== Route Validation Example ===");
    let external_route = feels_sdk::Route {
        pools: vec![usdc_feelssol_pool.clone(), sol_feelssol_pool.clone()],
        hops: 2,
        token_in: usdc_mint,
        token_out: sol_mint,
        uses_hub: true,
    };
    
    match router.validate_route(&external_route) {
        Ok(_) => println!("Route is valid!"),
        Err(e) => println!("Route validation failed: {}", e),
    }
    
    // Example 4: Invalid pool (no FeelsSOL)
    println!("\n=== Invalid Pool Example ===");
    let invalid_pool = PoolInfo {
        address: Pubkey::new_unique(),
        token_a: usdc_mint,
        token_b: sol_mint,
        fee_rate: 30,
    };
    
    match router.add_pool(invalid_pool) {
        Ok(_) => println!("Pool added (unexpected!)"),
        Err(e) => println!("Pool rejected: {}", e),
    }
    
    // Example 5: Get all pools for a token
    println!("\n=== Pools for Token Example ===");
    let feelssol_pools = router.get_pools_for_token(&feelssol_mint);
    println!("Pools containing FeelsSOL: {}", feelssol_pools.len());
    
    let usdc_pools = router.get_pools_for_token(&usdc_mint);
    println!("Pools containing USDC: {}", usdc_pools.len());
}