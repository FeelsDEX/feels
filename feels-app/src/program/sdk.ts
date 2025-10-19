// SDK wrapper to handle import compatibility issues

// Program ID
export const FEELS_PROGRAM_ID = 'FQSZnecUCVc2HnKsdPgNic641etrPT7gYiSic9NDPuTx';

// Import the IDL - this will be bundled by Next.js
// Using the full generated IDL
import GENERATED_IDL from '../idl/feels.json';

// IDL loaded successfully

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