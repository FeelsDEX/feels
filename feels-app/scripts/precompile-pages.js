#!/usr/bin/env node

/**
 * Pre-compile all Next.js pages after the dev server starts
 * This improves the development experience by building pages in advance
 */

const http = require('http');
const https = require('https');

// Configuration
const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';
const PARALLEL_REQUESTS = 3; // Number of concurrent requests

// All static pages to precompile
const STATIC_PAGES = [
  '/',              // Homepage (priority - loaded first)
  '/search',
  '/control',
  '/info',
  '/launch',
  '/docs',
  '/blog',
];

// Example dynamic pages for common use cases
const DYNAMIC_PAGES = [
  '/token/So11111111111111111111111111111111111111112',  // Wrapped SOL
  '/account/11111111111111111111111111111111',           // System program
  '/docs/getting-started',
  '/blog/welcome',
];

// Helper function to make HTTP request
function fetchPage(url) {
  return new Promise((resolve, reject) => {
    const client = url.startsWith('https') ? https : http;
    
    client.get(url, (res) => {
      // Consume response data to ensure the request completes
      res.on('data', () => {});
      res.on('end', () => {
        resolve({ url, status: res.statusCode });
      });
    }).on('error', (err) => {
      resolve({ url, error: err.message });
    });
  });
}

// Fetch pages with rate limiting
async function fetchPagesInBatches(pages, batchSize) {
  const results = [];
  
  for (let i = 0; i < pages.length; i += batchSize) {
    const batch = pages.slice(i, i + batchSize);
    const batchPromises = batch.map(page => fetchPage(`${BASE_URL}${page}`));
    const batchResults = await Promise.all(batchPromises);
    results.push(...batchResults);
    
    // Small delay between batches to avoid overwhelming the server
    if (i + batchSize < pages.length) {
      await new Promise(resolve => setTimeout(resolve, 100));
    }
  }
  
  return results;
}

// Wait for server to be ready
async function waitForServer(maxAttempts = 60) {
  console.log('⏳ Waiting for Next.js server to be ready...');
  
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const result = await fetchPage(BASE_URL);
      if (result.status === 200) {
        console.log('✅ Server is ready!');
        return true;
      }
    } catch (err) {
      // Server not ready yet
    }
    
    await new Promise(resolve => setTimeout(resolve, 1000));
  }
  
  console.log('⚠️  Server took too long to start');
  return false;
}

// Main execution
async function main() {
  // Wait for server to be ready
  const serverReady = await waitForServer();
  if (!serverReady) {
    process.exit(1);
  }

  // Wait a bit more to ensure server is stable
  await new Promise(resolve => setTimeout(resolve, 2000));

  console.log('\n🚀 Starting page pre-compilation...');
  
  // First, load homepage with priority
  console.log('  → Loading homepage (priority)...');
  const homepageResult = await fetchPage(BASE_URL);
  if (homepageResult.error) {
    console.log(`    ❌ Failed: ${homepageResult.error}`);
  } else {
    console.log(`    ✓ Homepage loaded (${homepageResult.status})`);
  }
  
  // Wait a bit to ensure homepage is fully processed
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  // Load static pages
  console.log('\n  → Pre-compiling static pages...');
  const staticResults = await fetchPagesInBatches(STATIC_PAGES.slice(1), PARALLEL_REQUESTS);
  
  staticResults.forEach(result => {
    if (result.error) {
      console.log(`    ❌ ${result.url}: ${result.error}`);
    } else {
      console.log(`    ✓ ${result.url.replace(BASE_URL, '')} (${result.status})`);
    }
  });
  
  // Load example dynamic pages
  console.log('\n  → Pre-compiling example dynamic routes...');
  const dynamicResults = await fetchPagesInBatches(DYNAMIC_PAGES, PARALLEL_REQUESTS);
  
  dynamicResults.forEach(result => {
    if (result.error) {
      console.log(`    ❌ ${result.url}: ${result.error}`);
    } else {
      console.log(`    ✓ ${result.url.replace(BASE_URL, '')} (${result.status})`);
    }
  });
  
  console.log('\n✅ Page pre-compilation complete!\n');
}

// Run if executed directly
if (require.main === module) {
  main().catch(err => {
    console.error('Error during pre-compilation:', err);
    process.exit(1);
  });
}

module.exports = { main, fetchPage, waitForServer };