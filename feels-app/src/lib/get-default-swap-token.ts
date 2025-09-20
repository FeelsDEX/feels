import { appConfig } from '@/config/app.config';
import { ALL_TOKENS } from './testData';

export function getDefaultSwapToken(): string {
  // If a specific token address is configured, use it
  if (appConfig.defaultSwapTokenAddress) {
    return appConfig.defaultSwapTokenAddress;
  }
  
  // If a token symbol is configured, find the matching token
  if (appConfig.defaultSwapTokenSymbol) {
    const token = ALL_TOKENS.find(t => t.symbol === appConfig.defaultSwapTokenSymbol);
    if (token) {
      return token.address;
    }
  }
  
  // If no default is configured or token not found, pick a random Feels token
  // Filter to only include Feels tokens (not SOL)
  const eligibleTokens = ALL_TOKENS.filter(t => t.symbol !== 'SOL' && t.isFeelsToken);
  
  if (eligibleTokens.length === 0) {
    // Fallback to first Feels token if no eligible tokens
    const firstFeelsToken = ALL_TOKENS.find(t => t.isFeelsToken);
    return firstFeelsToken?.address || ALL_TOKENS[0]?.address || '';
  }
  
  // Pick a random Feels token
  const randomIndex = Math.floor(Math.random() * eligibleTokens.length);
  return eligibleTokens[randomIndex].address;
}