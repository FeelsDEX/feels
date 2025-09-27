#!/usr/bin/env node
// Direct WebSocket connection to DevBridge for testing logarithmic axis

const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:54040');

let commandId = 1;
const pendingCommands = new Map();

ws.on('open', () => {
  console.log('Connected to DevBridge');
  
  // First, let's check the current state
  sendCommand('eval', {
    code: `
      const chartContainer = document.querySelector('#kline-chart');
      const chart = chartContainer?.__chart__;
      const button = Array.from(document.querySelectorAll('button')).find(b => 
        b.textContent?.includes('Linear') || b.textContent?.includes('Logarithmic')
      );
      
      // Get current Y-axis tick values
      const yAxisTicks = Array.from(document.querySelectorAll('.k-line-chart-y-axis-text, [class*="y-axis"] text, [class*="axis"] text'))
        .map(el => el.textContent)
        .filter(t => t && /^[$0-9.,]+$/.test(t));
      
      console.log('[TEST] Current button text:', button?.textContent);
      console.log('[TEST] Current Y-axis ticks:', yAxisTicks);
      console.log('[TEST] Chart instance available:', !!chart);
      
      // Try to get current axis configuration
      if (chart) {
        const styles = chart.getStyles?.();
        const paneOptions = chart.getPaneOptions?.('candle_pane');
        console.log('[TEST] Current styles yAxis type:', styles?.yAxis?.type);
        console.log('[TEST] Current paneOptions:', paneOptions);
      }
      
      return {
        buttonText: button?.textContent,
        yAxisTicks,
        chartAvailable: !!chart,
        currentAxisType: chart?.getStyles?.()?.yAxis?.type || 'unknown'
      };
    `
  });
  
  // Wait a bit, then click the dropdown and select logarithmic
  setTimeout(() => {
    console.log('\n--- Attempting to switch to logarithmic axis ---');
    sendCommand('eval', {
      code: `
        const button = Array.from(document.querySelectorAll('button')).find(b => 
          b.textContent?.includes('Linear') || b.textContent?.includes('Logarithmic')
        );
        
        if (button) {
          console.log('[TEST] Clicking axis dropdown button');
          button.click();
          
          setTimeout(() => {
            const logOption = Array.from(document.querySelectorAll('[role="menuitem"]')).find(item => 
              item.textContent?.includes('Logarithmic')
            );
            
            if (logOption) {
              console.log('[TEST] Found Logarithmic option, clicking it');
              logOption.click();
              
              // Wait for the change to apply
              setTimeout(() => {
                const chartContainer = document.querySelector('#kline-chart');
                const chart = chartContainer?.__chart__;
                
                // Get new Y-axis tick values
                const newYAxisTicks = Array.from(document.querySelectorAll('.k-line-chart-y-axis-text, [class*="y-axis"] text, [class*="axis"] text'))
                  .map(el => el.textContent)
                  .filter(t => t && /^[$0-9.,]+$/.test(t));
                
                console.log('[TEST] After switch - Button text:', button?.textContent);
                console.log('[TEST] After switch - Y-axis ticks:', newYAxisTicks);
                
                // Check the actual axis configuration
                if (chart) {
                  const styles = chart.getStyles?.();
                  const paneOptions = chart.getPaneOptions?.('candle_pane');
                  console.log('[TEST] After switch - styles yAxis type:', styles?.yAxis?.type);
                  console.log('[TEST] After switch - paneOptions:', paneOptions);
                  
                  // Try to directly check the internal axis state
                  const chartInternal = chart._chartPane || chart;
                  console.log('[TEST] Chart internal structure keys:', Object.keys(chartInternal));
                  
                  // Look for axis-related properties
                  if (chartInternal._panes) {
                    const candlePane = chartInternal._panes.find(p => p.id === 'candle_pane' || p._id === 'candle_pane');
                    console.log('[TEST] Candle pane found:', !!candlePane);
                    if (candlePane) {
                      console.log('[TEST] Candle pane axis info:', candlePane._axis || candlePane.axis);
                    }
                  }
                }
                
                // Parse tick values to check if they're logarithmic
                const parseValue = (str) => {
                  if (!str) return null;
                  const cleaned = str.replace(/[$,]/g, '');
                  return parseFloat(cleaned);
                };
                
                const values = newYAxisTicks.map(parseValue).filter(v => v !== null && !isNaN(v));
                if (values.length >= 3) {
                  const ratios = [];
                  for (let i = 1; i < values.length; i++) {
                    ratios.push(values[i] / values[i-1]);
                  }
                  console.log('[TEST] Tick value ratios:', ratios);
                  console.log('[TEST] Are ratios consistent (logarithmic)?', 
                    Math.max(...ratios) / Math.min(...ratios) < 1.5);
                }
              }, 1000);
            } else {
              console.log('[TEST] Could not find Logarithmic option in dropdown');
            }
          }, 300);
        } else {
          console.log('[TEST] Could not find axis dropdown button');
        }
        
        return 'Started axis switch process';
      `
    });
  }, 2000);
  
  // After switching, try to manually force logarithmic axis
  setTimeout(() => {
    console.log('\n--- Attempting to force logarithmic axis through chart API ---');
    sendCommand('eval', {
      code: `
        const chartContainer = document.querySelector('#kline-chart');
        const chart = chartContainer?.__chart__;
        
        if (chart) {
          console.log('[TEST] Attempting to force logarithmic axis...');
          
          // Try method 1: setPaneOptions
          try {
            chart.setPaneOptions({
              id: 'candle_pane',
              axisOptions: {
                type: 'log',
                name: 'log'
              }
            });
            console.log('[TEST] setPaneOptions with "log" succeeded');
          } catch (e) {
            console.log('[TEST] setPaneOptions with "log" failed:', e.message);
            
            // Try with 'logarithm'
            try {
              chart.setPaneOptions({
                id: 'candle_pane',
                axisOptions: {
                  type: 'logarithm',
                  name: 'logarithm'
                }
              });
              console.log('[TEST] setPaneOptions with "logarithm" succeeded');
            } catch (e2) {
              console.log('[TEST] setPaneOptions with "logarithm" failed:', e2.message);
            }
          }
          
          // Try method 2: setStyles
          chart.setStyles({
            yAxis: {
              type: 'log'
            }
          });
          console.log('[TEST] setStyles with type:"log" applied');
          
          // Force a re-render
          const data = chart.getDataList();
          if (data && data.length > 0) {
            chart.applyNewData(data, true);
            console.log('[TEST] Forced data refresh');
          }
          
          // Check the result
          setTimeout(() => {
            const newStyles = chart.getStyles?.();
            const newPaneOptions = chart.getPaneOptions?.('candle_pane');
            console.log('[TEST] Final styles yAxis type:', newStyles?.yAxis?.type);
            console.log('[TEST] Final paneOptions:', newPaneOptions);
            
            // Get final Y-axis ticks
            const finalTicks = Array.from(document.querySelectorAll('.k-line-chart-y-axis-text, [class*="y-axis"] text, [class*="axis"] text'))
              .map(el => el.textContent)
              .filter(t => t && /^[$0-9.,]+$/.test(t));
            console.log('[TEST] Final Y-axis ticks:', finalTicks);
          }, 500);
        }
        
        return 'Force logarithmic attempt complete';
      `
    });
  }, 5000);
});

ws.on('message', (data) => {
  const msg = JSON.parse(data.toString());
  
  if (msg.t === 'log') {
    console.log(`[Browser Console] ${msg.level}:`, ...msg.args);
  } else if (msg.t === 'result') {
    const callback = pendingCommands.get(msg.id);
    if (callback) {
      pendingCommands.delete(msg.id);
      if (msg.ok) {
        console.log('Command result:', msg.data);
        callback(null, msg.data);
      } else {
        console.log('Command error:', msg.error);
        callback(new Error(msg.error));
      }
    }
  } else if (msg.t === 'error') {
    console.error('[DevBridge Error]:', msg.message);
  }
});

ws.on('error', (error) => {
  console.error('WebSocket error:', error);
});

ws.on('close', () => {
  console.log('Disconnected from DevBridge');
  process.exit(0);
});

function sendCommand(name, args) {
  const id = `cmd_${commandId++}`;
  
  return new Promise((resolve, reject) => {
    pendingCommands.set(id, (err, result) => {
      if (err) reject(err);
      else resolve(result);
    });
    
    ws.send(JSON.stringify({
      t: 'command',
      id,
      name,
      args
    }));
    
    // Timeout after 10 seconds
    setTimeout(() => {
      if (pendingCommands.has(id)) {
        pendingCommands.delete(id);
        reject(new Error('Command timeout'));
      }
    }, 10000);
  });
}

// Keep the process alive
process.on('SIGINT', () => {
  console.log('\nClosing connection...');
  ws.close();
});
