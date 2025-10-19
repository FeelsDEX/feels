// Browser console script to test chart axis switching
// Paste this directly in the browser console to test axis functionality

// Extend Window interface with debugAxis
declare global {
  interface Window {
    debugAxis?: {
      getCurrentAxisType: () => unknown;
      setLogAxis: () => unknown;
      getYAxisTicks: () => unknown;
    };
  }
}

// First check current state
console.log('=== Current Axis State ===');
const result1 = window.debugAxis?.getCurrentAxisType();
console.log(result1);

// Try to set logarithmic axis
console.log('\n=== Attempting to set logarithmic axis ===');
const result2 = window.debugAxis?.setLogAxis();
console.log(result2);

// Check Y-axis ticks
console.log('\n=== Y-Axis Ticks Analysis ===');
const result3 = window.debugAxis?.getYAxisTicks();
console.log(result3);

// Try clicking the dropdown manually
console.log('\n=== Manual dropdown test ===');
const button = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find(
  (b) => b.textContent?.includes('Linear') || b.textContent?.includes('Logarithmic')
);
console.log('Current button text:', button?.textContent);

// If we have debug functions, use them
if (window.debugAxis) {
  console.log('\nDebug functions are available. Results above.');
} else {
  console.log('\nDebug functions not available. Page may need refresh.');
}

export {};

