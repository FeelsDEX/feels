// SDK wrapper to handle import compatibility issues

// Program ID
export const FEELS_PROGRAM_ID = 'BLjLS7TzUBncLxXMjFYxezioeg4RVdc5vRpVRYDq8GyQ';

// Import the IDL - this will be bundled by Next.js
// Using the full generated IDL
import GENERATED_IDL from '../idl/feels.json';

// Debug: Log IDL structure
if (typeof window !== 'undefined') {
  console.log('IDL loaded in browser:', {
    hasInstructions: !!(GENERATED_IDL as any).instructions,
    instructionCount: (GENERATED_IDL as any).instructions?.length,
    hasTypes: !!(GENERATED_IDL as any).types,
    typeCount: (GENERATED_IDL as any).types?.length,
  });
}

// Export the IDL directly - Anchor will handle the address internally
export const FEELS_IDL = GENERATED_IDL;

// Export instruction names for reference
export const INSTRUCTION_NAMES = [
  'initialize_protocol',
  'update_floor',
  'update_protocol',
  'set_protocol_owned_override',
  'initialize_pool_registry',
  'register_pool',
  'update_pool_phase',
  'initialize_pomm_position',
  'manage_pomm_position',
  'transition_market_phase',
  // ... more instructions available in the full IDL
] as const;

export type FeelsInstructionName = typeof INSTRUCTION_NAMES[number];