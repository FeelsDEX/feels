# DevBridge

DevBridge is a lightweight WebSocket-based development tool that enables CLI tools to interact with the Feels app during development. It provides real-time log streaming, event monitoring, and command execution capabilities.

## Features

- **Real-time log streaming**: Mirror browser console logs to CLI
- **Event tracking**: Monitor route changes, errors, and custom events
- **Command execution**: Send commands from CLI to browser
- **Zero production footprint**: Completely disabled in production builds

## Setup

1. Set environment variables in your `.env.local`:
```bash
DEVBRIDGE_ENABLED=true
NEXT_PUBLIC_DEVBRIDGE_ENABLED=true
```

2. Start the development server (includes DevBridge):
```bash
npm run dev
```

This will start:
- Next.js dev server on http://localhost:3000
- DevBridge WebSocket server on ws://127.0.0.1:54040

## CLI Usage

### Tail logs and events (default)
```bash
npm run devbridge tail
# or simply:
npm run devbridge
```

### Run a single command
```bash
npm run devbridge run <command> [args-json]

# Examples:
npm run devbridge run ping
npm run devbridge run navigate '{"path":"/search"}'
npm run devbridge run toggleFlag '{"name":"darkMode"}'
```

### Interactive mode
```bash
npm run devbridge interactive
> ping
> navigate {"path":"/token/WOJAK"}
> exit
```

## Available Commands

### Core Navigation & Testing
- `ping` - Test connection (returns `{pong: true}`)
- `navigate` - Navigate to a route: `{"path": "/path"}`
- `refresh` - Refresh current route
- `getPath` - Get current pathname
- `appInfo` - Get app version and environment

### Feature Flags & Storage
- `toggleFlag` - Toggle feature flag: `{"name": "flagName"}`
- `getFlags` - Get all feature flags
- `clearStorage` - Clear localStorage and sessionStorage
- `storageInfo` - Get storage usage info

### System Information
- `windowInfo` - Get window dimensions
- `perfMetrics` - Get performance metrics
- `setLogLevel` - Set console log level: `{"level": "all" | "warn" | "error" | "none"}`

### Testing & Events
- `testEvent` - Trigger a test event: `{"message": "optional message"}`
- `simulateWallet` - Simulate wallet connection: `{"connected": true/false}`

### Chart Debugging Commands
- `setChartAxisType` - Set chart y-axis type: `{"type": "linear" | "logarithmic" | "percentage"}`
- `getChartState` - Get chart state information
- `debugChart` - Get detailed chart instance info and available methods
- `recalcChartZoom` - Force chart zoom recalculation
- `testOverlayToggle` - Test floor/GTWAP overlay toggle functionality
- `debugUsdToggle` - Debug USD toggle and chart data
- `debugLogAxis` - Debug logarithmic axis behavior
- `getChartDebugInfo` - Get comprehensive chart debug information


## Adding Custom Commands

Add commands in `tools/devbridge/client/commands.ts`:

```typescript
registerCommand('myCommand', async ({ param }: { param: string }) => {
  // Command logic here
  return { success: true, param };
});
```

## Architecture

```
┌─────────────┐     WebSocket      ┌──────────────┐
│     CLI     │ ◄──────────────────► │  WS Server   │
│  (devbridge)│                     │ (port 54040) │
└─────────────┘                     └──────────────┘
                                           ▲
                                           │
                                     WebSocket
                                           │
                                           ▼
                                    ┌──────────────┐
                                    │   Browser    │
                                    │ (React Hook) │
                                    └──────────────┘
```

## Security

- Only accepts connections from localhost (127.0.0.1)
- Commands are allowlisted (no arbitrary code execution)
- Completely disabled in production builds
- No authentication (dev-only tool)

## Production Safety

The DevBridge is automatically disabled in production:

1. Server won't start without `DEVBRIDGE_ENABLED=true`
2. Client code is tree-shaken when `NEXT_PUBLIC_DEVBRIDGE_ENABLED` is not set
3. Dynamic imports ensure zero bundle impact in production

## Troubleshooting

**Connection refused**: Ensure both environment variables are set and the server is running

**Commands not working**: Check that the app has loaded and DevBridge is connected (you should see `[devbridge:connected]` event)

**Missing logs**: Verify `NEXT_PUBLIC_DEVBRIDGE_ENABLED=true` is set and refresh the browser