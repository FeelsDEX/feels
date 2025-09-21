import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { FEELS_IDL } from './sdk';

// Create Feels program instance with workaround for Anchor 0.31.1 account parsing issues
export function createFeelsProgram(provider: AnchorProvider): Program {
  try {
    // First, try with the full IDL as-is
    try {
      const program = new Program(FEELS_IDL as Idl, provider);
      console.log('Program created successfully with full IDL');
      return program;
    } catch (fullIdlError) {
      console.log('Full IDL failed:', fullIdlError);
    }
    
    // If that fails, modify the IDL to work around the issue
    // For Anchor 0.31.1, we need to ensure type definitions are preserved
    const idl = FEELS_IDL as any;
    
    // Create a type map for quick lookup
    const typeMap = new Map<string, any>();
    if (idl.types) {
      for (const type of idl.types) {
        typeMap.set(type.name, type);
      }
    }
    
    // Ensure all instruction args have their types properly defined
    const modifiedIDL = {
      ...idl,
      // Keep empty accounts to avoid the size error
      accounts: [],
      // Ensure types array is present
      types: idl.types || [],
      // Fix instruction args to ensure they reference types correctly
      instructions: (idl.instructions || []).map((ix: any) => ({
        ...ix,
        args: (ix.args || []).map((arg: any) => {
          // If the arg references a defined type, ensure it's in the correct format
          if (arg.type && arg.type.defined) {
            const typeName = typeof arg.type.defined === 'string' 
              ? arg.type.defined 
              : arg.type.defined.name || arg.type.defined;
            
            // Ensure the type exists in our types array
            if (!typeMap.has(typeName)) {
              console.warn(`Type ${typeName} not found in types array`);
            }
            
            return {
              ...arg,
              type: {
                defined: typeName
              }
            };
          }
          return arg;
        })
      }))
    };
    
    // Create program with modified IDL
    const program = new Program(modifiedIDL as Idl, provider);
    
    // Manually add account fetch methods if needed
    // This is a workaround for the missing account clients
    (program as any).account = (program as any).account || {};
    
    // Add manual account fetchers for commonly used accounts
    const accountTypes = ['Buffer', 'Market', 'Position', 'ProtocolConfig', 'PoolRegistry'];
    for (const accountType of accountTypes) {
      const accountDef = (FEELS_IDL as any).accounts?.find((a: any) => a.name === accountType);
      if (accountDef) {
        (program as any).account[accountType.toLowerCase()] = {
          fetch: async (address: PublicKey) => {
            const accountInfo = await provider.connection.getAccountInfo(address);
            if (!accountInfo) return null;
            // Note: This would need proper deserialization based on the IDL
            // For now, just return the raw data
            return accountInfo.data;
          }
        };
      }
    }
    
    // Log available methods for debugging
    const methods = Object.keys(program.methods || {});
    console.log(`Program created with ${methods.length} methods`);
    console.log('Available instructions:', methods.slice(0, 10).join(', '), methods.length > 10 ? '...' : '');
    
    return program;
  } catch (error) {
    console.error('Failed to create program:', error);
    throw new Error(`Failed to create Feels program: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}