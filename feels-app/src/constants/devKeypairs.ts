// Development testing keypairs - DO NOT USE IN PRODUCTION
// These are publicly known and intentionally insecure

export interface DevKeypair {
  publicKey: string;
  address: string;
  secretKey: number[];
}

// Primary development keypair ending in "FEEL"
export const PRIMARY_DEV_KEYPAIR: DevKeypair = {
  publicKey: 'tRfecbDu1OqMfcjEaR49esSFbLFEEL',
  address: 'tRfecbDu1OqMfcjEaR49esSFbLFEEL',
  secretKey: [17, 89, 82, 56, 91, 149, 203, 202, 120, 246, 79, 216, 138, 243, 118, 57, 234, 43, 75, 188, 190, 198, 158, 108, 199, 233, 151, 25, 163, 255, 210, 205]
};

// Alternative development keypair ending in "FEEL"
export const ALT_DEV_KEYPAIR: DevKeypair = {
  publicKey: 'CEZJn30U0GL5jgd89oKNtHCv665FEEL',
  address: 'CEZJn30U0GL5jgd89oKNtHCv665FEEL',
  secretKey: [152, 197, 84, 78, 11, 102, 198, 65, 240, 58, 47, 142, 105, 216, 46, 223, 90, 167, 95, 177, 63, 70, 31, 230, 132, 160, 126, 171, 253, 46, 223, 161]
};

/**
 * Get the primary development keypair for test data mode
 * WARNING: This is publicly known and insecure - only for development!
 */
export function getDevKeypair(): DevKeypair {
  return PRIMARY_DEV_KEYPAIR;
}