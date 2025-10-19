// Unified token data source
import feelsGuyImage from '@/assets/images/feels_guy.png';
import wojakImage from '@/assets/images/wojak_original.jpg';
import pinkWojakImage from '@/assets/images/pink_wojak_correct.png';
import chadImage from '@/assets/images/chad.png';
import npcWojakImage from '@/assets/images/npc_wojak.png';

export interface Token {
  id: string;
  address: string;
  name: string;
  symbol: string;
  imageUrl: string | any; // Can be URL string or Next.js static import
  decimals: number;
  price: number;
  priceChange24h: number;
  marketCap: string;
  volume24h: string;
  high24h: number;
  low24h: number;
  floorPrice: number;
  gtwapPrice: number;
  floorChange24h: number;
  floorGtwapRatio: number;
  launched: string;
  description: string;
  isFeelsToken: boolean;
  creator: string;
  isGraduated: boolean;
}

// All tokens in the Feels ecosystem
export const FEELS_TOKENS: Token[] = [
  // Common base tokens for swapping
  {
    id: '0-sol',
    address: 'So11111111111111111111111111111111111111112',
    name: 'Solana',
    symbol: 'SOL',
    imageUrl: 'https://cdn.jsdelivr.net/gh/trustwallet/assets@master/blockchains/solana/assets/So11111111111111111111111111111111111111112/logo.png',
    decimals: 9,
    price: 58.32,
    priceChange24h: 3.2,
    marketCap: '$25.4B',
    volume24h: '$2.3B',
    high24h: 61.45,
    low24h: 56.12,
    floorPrice: 57.89,
    gtwapPrice: 58.15,
    floorChange24h: 2.8,
    floorGtwapRatio: 99.5,
    launched: 'Native',
    description: 'Native Solana token',
    isFeelsToken: false,
    creator: '11111111111111111111111111111111',
    isGraduated: true
  },
  {
    id: '0-usdc',
    address: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
    name: 'USD Coin',
    symbol: 'USDC',
    imageUrl: 'https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png',
    decimals: 6,
    price: 0.9998,
    priceChange24h: -0.02,
    marketCap: '$43.8B',
    volume24h: '$5.2B',
    high24h: 1.0005,
    low24h: 0.9992,
    floorPrice: 0.9996,
    gtwapPrice: 0.9998,
    floorChange24h: -0.01,
    floorGtwapRatio: 99.98,
    launched: 'Native',
    description: 'USD Coin stablecoin',
    isFeelsToken: false,
    creator: '11111111111111111111111111111111',
    isGraduated: true
  },
  {
    id: '1',
    address: 'WojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb1feel',
    name: 'Wojak',
    symbol: 'WOJAK',
    imageUrl: wojakImage,
    decimals: 9,
    price: 0.0423,
    priceChange24h: 8.9,
    marketCap: '$4.2M',
    volume24h: '$1.2M',
    high24h: 0.0445,
    low24h: 0.0389,
    floorPrice: 0.0418,
    gtwapPrice: 0.0421,
    floorChange24h: 7.2,
    floorGtwapRatio: 99.3,
    launched: '3 days ago',
    description: 'The original feels guy',
    isFeelsToken: true,
    creator: '7XawhbbxtsRcQA8FstyZpudN8pSDS9DC95uJDxPBaqMf',
    isGraduated: true
  },
  {
    id: '2',
    address: 'PepewJ9nJKy3sLKCqczaTrd2TRnhjxNLPqZB8nu2feel',
    name: 'Pepe',
    symbol: 'PEPE',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0089,
    priceChange24h: -3.2,
    marketCap: '$890K',
    volume24h: '$120K',
    high24h: 0.0092,
    low24h: 0.0086,
    floorPrice: 0.0087,
    gtwapPrice: 0.0089,
    floorChange24h: -4.1,
    floorGtwapRatio: 97.8,
    launched: '5 days ago',
    description: 'Smug frog friend of Wojak',
    isFeelsToken: true,
    creator: '7XawhbbxtsRcQA8FstyZpudN8pSDS9DC95uJDxPBaqMf',
    isGraduated: true
  },
  {
    id: '3',
    address: 'DoomrP9uyrQpS3yn2Q5GeRFrYWnBDfvPKjLX84A3feel',
    name: 'Doomer',
    symbol: 'DOOMER',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0089,
    priceChange24h: -3.2,
    marketCap: '$890K',
    volume24h: '$120K',
    high24h: 0.0092,
    low24h: 0.0086,
    floorPrice: 0.0087,
    gtwapPrice: 0.0089,
    floorChange24h: -4.1,
    floorGtwapRatio: 97.8,
    launched: '5 days ago',
    description: 'Eternal pessimist of the Wojak universe',
    isFeelsToken: true,
    creator: 'BsV4An3XGGe7S7DqmTz8kMS9gJ3JAddVHRBM54GfJpBQ',
    isGraduated: false
  },
  {
    id: '4',
    address: 'BloomJ34hNPn8NAzX5HJpNvFcXJGGWTZWkKUbjRpfeel',
    name: 'Bloomer',
    symbol: 'BLOOMER',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0234,
    priceChange24h: 15.6,
    marketCap: '$2.3M',
    volume24h: '$890K',
    high24h: 0.0251,
    low24h: 0.0202,
    floorPrice: 0.0231,
    gtwapPrice: 0.0233,
    floorChange24h: 14.2,
    floorGtwapRatio: 99.1,
    launched: '7 days ago',
    description: 'Optimistic transformation of the Doomer',
    isFeelsToken: true,
    creator: 'BsV4An3XGGe7S7DqmTz8kMS9gJ3JAddVHRBM54GfJpBQ',
    isGraduated: false
  },
  {
    id: '5',
    address: 'CoomrGPT4NL8z3xZpYjQcBJknmggY3htVKe3SUBzfeel',
    name: 'Coomer',
    symbol: 'COOMER',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0057,
    priceChange24h: -8.3,
    marketCap: '$1.9M',
    volume24h: '$320K',
    high24h: 0.0062,
    low24h: 0.0055,
    floorPrice: 0.0056,
    gtwapPrice: 0.0058,
    floorChange24h: -9.1,
    floorGtwapRatio: 96.6,
    launched: '10 days ago',
    description: 'Down bad member of the Wojak family',
    isFeelsToken: true,
    creator: 'Gz7VkD4MacbEB6yC5XD3HcumEiYx2EtDYYrfikGsvopG',
    isGraduated: false
  },
  {
    id: '6',
    address: 'GrugJ3fYgKwpt5HqNYoiFcF3WgC5Nz7VXeMBBBq6feel',
    name: 'Grug',
    symbol: 'GRUG',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0912,
    priceChange24h: 67.3,
    marketCap: '$9.1M',
    volume24h: '$3.4M',
    high24h: 0.0934,
    low24h: 0.0544,
    floorPrice: 0.0898,
    gtwapPrice: 0.0906,
    floorChange24h: 65.1,
    floorGtwapRatio: 99.1,
    launched: '1 day ago',
    description: 'Simple cave brain Wojak variant',
    isFeelsToken: true,
    creator: 'FUkonnF8eCT8x3wfhXdNPKNs6MaRVqxaCyRdAQUwVWXw',
    isGraduated: false
  },
  {
    id: '7',
    address: 'NPCfQ2XbTDN4bWoFZCTQDrdgnDVXKyVGaBPc8Qy7feel',
    name: 'NPC',
    symbol: 'NPC',
    imageUrl: npcWojakImage,
    decimals: 9,
    price: 0.0123,
    priceChange24h: 2.1,
    marketCap: '$12.3M',
    volume24h: '$4.5M',
    high24h: 0.0125,
    low24h: 0.0119,
    floorPrice: 0.0121,
    gtwapPrice: 0.0122,
    floorChange24h: 1.8,
    floorGtwapRatio: 99.2,
    launched: '2 weeks ago',
    description: 'Non-player character in the feels economy',
    isFeelsToken: true,
    creator: '5Q544fKrFoe6tsEbD7S8Emxmy5WPg4zAqx5c9cW9STKS',
    isGraduated: true
  },
  {
    id: '8',
    address: 'ZoomrMb58rwhpJXagnSy2ypqJm8H5RJJvPPuxmW8feel',
    name: 'Zoomer',
    symbol: 'ZOOMER',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0345,
    priceChange24h: 12.4,
    marketCap: '$3.4M',
    volume24h: '$780K',
    high24h: 0.0359,
    low24h: 0.0307,
    floorPrice: 0.0341,
    gtwapPrice: 0.0344,
    floorChange24h: 11.8,
    floorGtwapRatio: 99.1,
    launched: '4 days ago',
    description: 'Young, energetic member of the Wojak ecosystem',
    isFeelsToken: true,
    creator: 'E5rk3nmgJUfKpKBGM7cP6RNKD4onPZU5gT7xhPhZYigN',
    isGraduated: false
  },
  {
    id: '9',
    address: 'BoboH61oqCBBcdjW23wYJQCKX1eYm5xYRSCwQCEefeel',
    name: 'Bobo',
    symbol: 'BOBO',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0078,
    priceChange24h: -12.3,
    marketCap: '$780K',
    volume24h: '$56K',
    high24h: 0.0089,
    low24h: 0.0076,
    floorPrice: 0.0077,
    gtwapPrice: 0.0079,
    floorChange24h: -13.1,
    floorGtwapRatio: 97.5,
    launched: '6 days ago',
    description: 'Bear market predictor, always shorting',
    isFeelsToken: true,
    creator: 'CUqDJqBBKiXMKRiZN4fAf7NbMR8p8UaG8M6U7X8hQxvb',
    isGraduated: false
  },
  {
    id: '10',
    address: 'BizMBqsKBNqhfhQ2gWdg4KuBPKBXxKBkHKECQo10feel',
    name: 'Bizonacci',
    symbol: 'BIZONACCI',
    imageUrl: feelsGuyImage,
    decimals: 9,
    price: 0.0567,
    priceChange24h: 23.8,
    marketCap: '$5.6M',
    volume24h: '$2.1M',
    high24h: 0.0578,
    low24h: 0.0458,
    floorPrice: 0.0561,
    gtwapPrice: 0.0565,
    floorChange24h: 22.4,
    floorGtwapRatio: 99.3,
    launched: '2 days ago',
    description: 'The prophet of crypto memes',
    isFeelsToken: true,
    creator: 'Hu9wbnyDdtafDEdWJpqn4MXMNuC6uAFJYkLdgfDYVsHx',
    isGraduated: false
  },
  {
    id: 'chad',
    address: 'ChadGPT4NL8z3xZpYjQcBJknmggY3htVKe3SUBz1feel',
    name: 'Chad',
    symbol: 'CHAD',
    imageUrl: chadImage,
    decimals: 9,
    price: 0.0823,
    priceChange24h: 45.2,
    marketCap: '$8.2M',
    volume24h: '$3.1M',
    high24h: 0.0834,
    low24h: 0.0567,
    floorPrice: 0.0798,
    gtwapPrice: 0.0815,
    floorChange24h: 41.8,
    floorGtwapRatio: 97.9,
    launched: '2 days ago',
    description: 'The ultimate alpha male of crypto',
    isFeelsToken: true,
    creator: 'Gz7VkD4MacbEB6yC5XD3HcumEiYx2EtDYYrfikGsvopG',
    isGraduated: false
  },
  {
    id: 'pink-wojak',
    address: 'PinkWojakNL8z3xZpYjQcBJknmggY3htVKe3SUBzfeel',
    name: 'Pink Wojak',
    symbol: 'PINK',
    imageUrl: pinkWojakImage,
    decimals: 9,
    price: 0.0034,
    priceChange24h: -23.7,
    marketCap: '$340K',
    volume24h: '$89K',
    high24h: 0.0045,
    low24h: 0.0032,
    floorPrice: 0.0033,
    gtwapPrice: 0.0037,
    floorChange24h: -26.2,
    floorGtwapRatio: 89.2,
    launched: '1 week ago',
    description: 'When your portfolio hits different',
    isFeelsToken: true,
    creator: 'E5rk3nmgJUfKpKBGM7cP6RNKD4onPZU5gT7xhPhZYigN',
    isGraduated: false
  }
];

// Get featured tokens for homepage (top 4 by market cap)
export function getHomepageTokens() {
  // Return specific meme tokens: Wojak, Chad, Pink Wojak, NPC
  return [
    FEELS_TOKENS.find(t => t.symbol === 'WOJAK')!,
    FEELS_TOKENS.find(t => t.symbol === 'CHAD')!,
    FEELS_TOKENS.find(t => t.symbol === 'PINK')!,
    FEELS_TOKENS.find(t => t.symbol === 'NPC')!
  ];
}

// Get token by address
export function getTokenByAddress(address: string) {
  return FEELS_TOKENS.find(token => token.address === address);
}

// Get token by symbol
export function getTokenBySymbol(symbol: string) {
  return FEELS_TOKENS.find(token => token.symbol === symbol);
}