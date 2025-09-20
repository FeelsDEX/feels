// Centralized test data for the application
// All tokens are characters from the Feels Guy/Wojak universe

import feelsGuyImage from '@/assets/images/feels_guy.png';
import solanaLogoImage from '@/assets/images/solana_logo.svg';

export interface TokenData {
  id: string;
  address: string;
  name: string;
  symbol: string;
  imageUrl: string;
  decimals: number;
  price: number;
  priceChange24h: number;
  marketCap: string;
  volume24h: string;
  launched: string;
  description?: string;
  isFeelsToken: boolean;
  creator?: string;
}

export interface PoolData {
  id: string;
  name: string;
  fee: string;
  tvl: string;
  volume24h?: string;
}

// Wojak universe tokens
export const WOJAK_TOKENS: TokenData[] = [
  {
    id: '1',
    address: 'feelsWojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb',
    name: 'Wojak',
    symbol: 'WOJAK',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0423,
    priceChange24h: 8.9,
    marketCap: '$4.2M',
    volume24h: '$1.2M',
    launched: '3 days ago',
    description: 'The original feels guy',
    isFeelsToken: true,
    creator: '7XawhbbxtsRcQA8FstyZpudN8pSDS9DC95uJDxPBaqMf'
  },
  {
    id: '2',
    address: 'feelsDoomrP9uyrQpS3yn2Q5GeRFrYWnBDfvPKjLX84A',
    name: 'Doomer',
    symbol: 'DOOMER',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0089,
    priceChange24h: -3.2,
    marketCap: '$890K',
    volume24h: '$120K',
    launched: '5 days ago',
    description: 'Everything is meaningless',
    isFeelsToken: true,
    creator: '9Zpn2Mx5cpGJn8oSnqVHCpCJigrYCGWxjQ9faiEGmUfN'
  },
  {
    id: '3',
    address: 'feelsBoomrKxH7C8mghZMc6VvgLTJfKebM2oqcf8rZRd',
    name: 'Boomer',
    symbol: 'BOOMER',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0156,
    priceChange24h: 45.8,
    marketCap: '$1.6M',
    volume24h: '$780K',
    launched: '1 week ago',
    description: 'Back in my day...',
    isFeelsToken: true,
    creator: 'BZRsZqgABjitpaNAJ2dgeVJBvyQ8CRYzUCA3qNnHfQMj'
  },
  {
    id: '4',
    address: 'feelsZoomrABNqRdyVtDD9zK3voj4UBEn8FbNsWXEQgB',
    name: 'Zoomer',
    symbol: 'ZOOMER',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0078,
    priceChange24h: -12.4,
    marketCap: '$780K',
    volume24h: '$95K',
    launched: '2 weeks ago',
    description: 'No cap fr fr',
    isFeelsToken: true,
    creator: '4Hkz3XvHm2jjbexvpZYcYQqAx5rq3Wqo3hKyhxm4x9sw'
  },
  {
    id: '5',
    address: 'feelsGrugJ3fYgKwpt5HqNYoiFcF3WgC5Nz7VXeMBBBq',
    name: 'Grug',
    symbol: 'GRUG',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0912,
    priceChange24h: 67.3,
    marketCap: '$9.1M',
    volume24h: '$3.4M',
    launched: '1 day ago',
    description: 'Grug simple',
    isFeelsToken: true
  },
  {
    id: '6',
    address: 'feelsChadQbQg8cUKW3pEfkYQwzJhfvj5u8fUvJgpQfG',
    name: 'Chad',
    symbol: 'CHAD',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0345,
    priceChange24h: 23.1,
    marketCap: '$3.4M',
    volume24h: '$890K',
    launched: '4 days ago',
    description: 'Yes.',
    isFeelsToken: true
  },
  {
    id: '7',
    address: 'feelsSoyjakzK2LmXZH9uJbE8tSeXQXXp7FWbJbB5CB5',
    name: 'Soyjak',
    symbol: 'SOYJAK',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0567,
    priceChange24h: 5.7,
    marketCap: '$5.7M',
    volume24h: '$2.1M',
    launched: '6 days ago',
    description: 'I heckin love science!',
    isFeelsToken: true
  },
  {
    id: '8',
    address: 'feelsCoomrGPT4NL8z3xZpYjQcBJknmggY3htVKe3SUBz',
    name: 'Coomer',
    symbol: 'COOMER',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0198,
    priceChange24h: -8.3,
    marketCap: '$1.9M',
    volume24h: '$320K',
    launched: '10 days ago',
    description: 'Must... not...',
    isFeelsToken: true
  },
  {
    id: '9',
    address: 'feelsBloomWGMKdCK9SqtAjeuK6DSQSz3cDitYCYGBtfE',
    name: 'Bloomer',
    symbol: 'BLOOMER',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0756,
    priceChange24h: 34.2,
    marketCap: '$7.5M',
    volume24h: '$2.8M',
    launched: '3 days ago',
    description: 'We\'re all gonna make it',
    isFeelsToken: true
  },
  {
    id: '10',
    address: 'feelsSchizoFfeeLSgNzQs5ZuqVWcQRgcDrBB3W3bMoRM',
    name: 'Schizo',
    symbol: 'SCHIZO',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0042,
    priceChange24h: 420.69,
    marketCap: '$420K',
    volume24h: '$69K',
    launched: '1 hour ago',
    description: 'They glow in the dark',
    isFeelsToken: true
  },
  {
    id: '11',
    address: 'feelsNPCMintK5TQaWjM3BRJpnkYxBsHcqKzWWUFYg27M',
    name: 'NPC',
    symbol: 'NPC',
    imageUrl: feelsGuyImage.src,
    decimals: 9,
    price: 0.0111,
    priceChange24h: 0.0,
    marketCap: '$1.1M',
    volume24h: '$111K',
    launched: '1 week ago',
    description: 'I support the current thing',
    isFeelsToken: true
  }
];

// Standard tokens (not Feels tokens)
export const STANDARD_TOKENS = [
  {
    id: 'sol',
    address: 'So11111111111111111111111111111111111111112',
    symbol: 'SOL',
    name: 'Solana',
    decimals: 9,
    logoURI: solanaLogoImage.src,
    isFeelsToken: false
  },
  {
    id: 'jitosol',
    address: 'J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn',
    symbol: 'JitoSOL',
    name: 'Jito Staked SOL',
    decimals: 9,
    logoURI: 'https://storage.googleapis.com/token-metadata/JitoSOL-256.png',
    isFeelsToken: false
  },
  {
    id: 'feelssol',
    address: 'FeeLSoLFCcUe32kiGgtPBZ4pGhcpLnkUbVFZB26oZaD',
    symbol: 'FeelsSOL',
    name: 'Feels Protocol SOL',
    decimals: 9,
    logoURI: feelsGuyImage.src,
    isFeelsToken: false
  },
  {
    id: 'usdc',
    address: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
    symbol: 'USDC',
    name: 'USD Coin',
    decimals: 6,
    logoURI: 'https://assets.coingecko.com/coins/images/6319/large/usdc.png',
    isFeelsToken: false
  }
];

// Available pools for liquidity page
export const AVAILABLE_POOLS: PoolData[] = [
  { id: 'SOL/USDC', name: 'SOL/USDC', fee: '0.3%', tvl: '$2.4M', volume24h: '$450K' },
  { id: 'SOL/JitoSOL', name: 'SOL/JitoSOL', fee: '0.05%', tvl: '$1.8M', volume24h: '$320K' },
  { id: 'FeelsSOL/SOL', name: 'FeelsSOL/SOL', fee: '0.3%', tvl: '$950K', volume24h: '$180K' },
  { id: 'USDC/USDT', name: 'USDC/USDT', fee: '0.01%', tvl: '$3.2M', volume24h: '$890K' },
  { id: 'JitoSOL/mSOL', name: 'JitoSOL/mSOL', fee: '0.05%', tvl: '$1.1M', volume24h: '$210K' },
  { id: 'WOJAK/FeelsSOL', name: 'WOJAK/FeelsSOL', fee: '1%', tvl: '$420K', volume24h: '$69K' },
  { id: 'COOMER/FeelsSOL', name: 'COOMER/FeelsSOL', fee: '1%', tvl: '$380K', volume24h: '$85K' },
  { id: 'CHAD/FeelsSOL', name: 'CHAD/FeelsSOL', fee: '1%', tvl: '$560K', volume24h: '$120K' }
];

// Combined token list for swap interface
export const ALL_TOKENS = [
  ...STANDARD_TOKENS.map(token => ({
    ...token,
    address: token.address,
    imageUrl: token.logoURI
  })),
  ...WOJAK_TOKENS.map(token => ({
    address: token.address,
    symbol: token.symbol,
    name: token.name,
    decimals: token.decimals,
    logoURI: token.imageUrl,
    isFeelsToken: token.isFeelsToken,
    creator: token.creator
  }))
];

// Helper function to get token by symbol
export const getTokenBySymbol = (symbol: string) => {
  return ALL_TOKENS.find(token => token.symbol === symbol);
};

// Helper function to get pool by ID
export const getPoolById = (poolId: string) => {
  return AVAILABLE_POOLS.find(pool => pool.id === poolId);
};