// Script to check vanity miner status via browser console
// Run this in the browser console at http://localhost:3000

console.log('=== Vanity Miner Status Check ===');
console.log('SharedArrayBuffer available:', typeof SharedArrayBuffer !== 'undefined');
console.log('Window vanity miner status:', window.__vanityMinerStatus);
console.log('Window vanity miner control:', window.__vanityMinerControl);

// Check if miner is available
if (window.__vanityMinerStatus) {
  console.log('\nMiner Status:');
  console.log('- Ready:', window.__vanityMinerStatus.isReady);
  console.log('- Running:', window.__vanityMinerStatus.isRunning);
  console.log('- Attempts:', window.__vanityMinerStatus.attempts);
  console.log('- Elapsed (ms):', window.__vanityMinerStatus.elapsedMs);
  
  if (window.__vanityMinerStatus.attempts > 0 && window.__vanityMinerStatus.elapsedMs > 0) {
    const rate = Math.round(window.__vanityMinerStatus.attempts / (window.__vanityMinerStatus.elapsedMs / 1000));
    console.log('- Rate:', rate.toLocaleString(), 'attempts/sec');
  }
  
  if (window.__vanityMinerStatus.keypair) {
    console.log('- Found keypair:', window.__vanityMinerStatus.keypair.publicKey);
  }
}

// Force start mining if not running
if (window.__vanityMinerControl && window.__vanityMinerStatus && !window.__vanityMinerStatus.isRunning) {
  console.log('\nStarting mining...');
  window.__vanityMinerControl.resetAndMine();
}

console.log('\nTo manually control:');
console.log('- Start new: window.__vanityMinerControl.resetAndMine()');
console.log('- Stop: window.__vanityMinerControl.stopMining()');
console.log('- Check status: window.__vanityMinerStatus');