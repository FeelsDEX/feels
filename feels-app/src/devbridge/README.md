# DevBridge

DevBridge is a lightweight WebSocket-based development tool that enables CLI tooks to interact with the Feels app during development. It provides real-time log streaming, event monitoring, and command execution capabilities.

## Features

- **Real-time log streaming**: Mirror browser console logs to CLI
- **Event tracking**: Monitor route changes, errors, and custom events
- **Command execution**: Send commands from CLI to browser
- **LLM-friendly**: Designed for tool-based interaction
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

- `ping` - Test connection (returns `{pong: true}`)
- `navigate` - Navigate to a route: `{"path": "/path"}`
- `refresh` - Refresh current route
- `getPath` - Get current pathname
- `toggleFlag` - Toggle feature flag: `{"name": "flagName"}`
- `getFlags` - Get all feature flags
- `appInfo` - Get app version and environment
- `clearStorage` - Clear localStorage and sessionStorage
- `storageInfo` - Get storage usage info
- `windowInfo` - Get window dimensions
- `perfMetrics` - Get performance metrics

## LLM Integration

The DevBridge is designed to be used by LLMs through their tool-calling capabilities:

```typescript
// Example LLM tool definition
{
  name: "devbridge",
  description: "Interact with Feels app via DevBridge",
  parameters: {
    command: "run",
    name: "navigate",
    args: { path: "/search" }
  }
}
```

## Adding Custom Commands

Add commands in `app/devbridge/commands.ts`:

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