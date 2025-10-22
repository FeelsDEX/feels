# DeepWiki MCP Integration Guide

## Overview

The Feels Protocol documentation is available through DeepWiki's Model Context Protocol (MCP) server at `https://deepwiki.com/feelsdex/feels`. This integration provides AI-assisted access to the complete protocol documentation directly from MCP-compatible development environments.

## What is MCP?

Model Context Protocol (MCP) is an open standard developed by Anthropic in November 2024 that standardizes how AI systems integrate and share data with external tools, systems, and data sources. MCP provides a universal interface for reading files, executing functions, and handling contextual prompts.

The protocol has been adopted by major AI providers including OpenAI and Google DeepMind, and is supported by development tools like Cursor, Claude Desktop, and Windsurf.

## Accessing Feels Protocol Documentation via MCP

### Prerequisites

1. An MCP-compatible client (Cursor, Claude Desktop, or Windsurf)
2. Network access to DeepWiki's servers

### Configuration

To configure your MCP client to access the Feels Protocol documentation:

1. **Add the MCP Server**: Configure your MCP client to connect to the DeepWiki server hosting Feels Protocol documentation:

   ```
   https://deepwiki.com/feelsdex/feels
   ```

2. **Client-Specific Setup**:
   
   - **Cursor**: The MCP server can be referenced using the `@https://deepwiki.com/feelsdex/feels` syntax in conversations
   - **Claude Desktop**: Add the server URL to your MCP configuration file
   - **Windsurf**: Follow the IDE's MCP server configuration process

3. **Verify Connection**: Once configured, test the connection by querying basic protocol information

### Example Queries

When connected to the DeepWiki MCP server, you can ask questions like:

- "How does the hub-and-spoke architecture work in Feels Protocol?"
- "What are the steps to launch a new token?"
- "Explain the concentrated liquidity mechanism"
- "How does the dynamic fee model calculate fees?"
- "What is the FeelsSOL solvency model?"

## Documentation Scope

The DeepWiki MCP server provides access to comprehensive Feels Protocol documentation organized into several categories:

### Getting Started

**Introduction** (`001-introduction.md`)
- Protocol overview and key features
- Hub-and-spoke architecture summary
- Concentrated liquidity basics
- Physics-based trading model introduction

**Quickstart Guide** (`002-quickstart.md`)
- Wallet connection instructions
- First swap walkthrough
- Liquidity provision guide
- Fee structure overview

**Hub and Spoke Architecture** (`003-hub-and-spoke-architecture.md`)
- FeelsSOL hub token mechanics
- Trade routing patterns (1-hop and 2-hop)
- Liquidity concentration benefits
- Capital efficiency for traders and LPs

### Core Specifications

**FeelsSOL Solvency** (`200-feelssol-solvency.md`)
- Two-layer solvency model (pool-level and protocol-level)
- JitoSOL backing mechanics
- Risk analysis and mitigation strategies
- Oracle architecture for reserve pricing
- Safety controller mechanisms
- Solvency invariants and mathematical proofs
- Worst-case exit scenarios

**Dynamic Fees** (`201-dynamic-fees.md`)
- Base fee configuration
- Impact fees based on price movement
- Fee distribution across LPs, protocol, and creators
- Floor protection mechanisms

**JIT Liquidity** (`202-jit-liquidity.md`)
- Just-in-time liquidity provision
- JIT position management
- Integration with swap flows

**Concentrated Liquidity AMM** (`203-pool-clmm.md`)
- Price, ticks, and liquidity mechanics
- Position management (NFT-tokenized)
- Tick arrays and data structures
- Core instructions: initialize_pool, open_position, close_position, swap
- Fee accounting mechanisms
- External library dependencies (orca-whirlpools-core, ethnum)

**Pool Oracle** (`204-pool-oracle.md`)
- Per-pool GTWAP (Geometric Time-Weighted Average Price)
- Oracle update mechanisms
- Integration with fees, JIT, and floor systems

**Floor Liquidity** (`205-floor-liquidity.md`)
- Floor price calculation
- Monotonic floor ratchet mechanism
- Floor buffer management
- Protocol-owned liquidity

**Pool Allocation** (`206-pool-allocation.md`)
- Liquidity distribution strategies
- Position sizing and range selection

**Bonding Curve Feels** (`207-bonding-curve-feels.md`)
- Token launch bonding curves
- Stair-step liquidity pattern
- Price discovery mechanisms

**After Swap Pipeline** (`208-after-swap-pipeline.md`)
- Post-swap processing
- Fee distribution execution
- State updates and events

**Params and Governance** (`209-params-and-governance.md`)
- Hierarchical parameter management
- Core parameters (risk tolerance, responsiveness, floor safety margin)
- Governance mechanisms
- Parameter update procedures

**Safety Controller** (`210-safety-controller.md`)
- Health status tracking
- Global pause and degraded mode controls
- Rate limiting mechanisms
- Oracle update validation

**Events and Units** (`211-events-and-units.md`)
- Event emission patterns
- Unit conventions and precision
- Q64.64 fixed-point arithmetic

**Pool Registry** (`212-pool-registry.md`)
- Pool registration and discovery
- Market metadata management

### Protocol Sequences

**Launch Sequence** (`300-launch-sequence.md`)
- Complete token launch process with detailed steps:
  1. Convert JitoSOL to FeelsSOL (`enter_feelssol`)
  2. Mint Protocol Token (`mint_token`)
  3. Initialize Market (`initialize_market`)
  4. Deploy Initial Liquidity (`deploy_initial_liquidity`)
  5. Token Expiration handling (`destroy_expired_token`)
  6. Pool Graduation (future)
  7. FeelsSOL Redemption (`exit_feelssol`)
- Account structures and PDAs
- Complete sequence diagram
- Example flow with 3000 FeelsSOL

**Market State and Lifecycle** (`301-market-state-and-lifecycle.md`)
- Market states and transitions
- Lifecycle management
- State validation rules

### Blog Content

The MCP server also provides access to blog posts explaining protocol concepts:

**Introducing Feels** (`100-introducing-feels.md`)
- Protocol announcement and vision
- Key innovations and differentiators

**System Introduction** (`101-system-intro.md`)
- High-level system architecture
- Component interactions

**Unified Markets** (`102-unified-markets.md`)
- Hub-and-spoke market structure
- Unified liquidity benefits

**Spot LP** (`103-spot-lp.md`)
- Liquidity provider strategies
- Position management best practices

## Documentation Categories

The documentation is organized into four primary categories:

1. **Getting Started** - Introductory materials and quickstart guides
2. **Specifications** - Detailed technical specifications (200-series)
3. **Protocol Sequences** - Step-by-step process documentation (300-series)
4. **Blog** - High-level explanations and announcements

## Key Topics Covered

### Architecture
- Hub-and-spoke routing with FeelsSOL as the central hub
- Concentrated liquidity (Uniswap V3-style)
- Zero-copy account design for efficiency
- Token-2022 support

### Trading Mechanics
- Maximum 2-hop routing for any swap
- Tick-based pricing with configurable spacing
- Dynamic fee model (base + impact fees)
- NFT-based position tracking

### Solvency & Safety
- Two-layer solvency model (pool and protocol levels)
- JitoSOL backing with 1:1 FeelsSOL minting
- Conservative oracle design with safety buffers
- Safety controller with circuit breakers
- Monotonic floor ratchet protection

### Token Launches
- Pre-launch escrow system
- Fixed-supply tokens (revoked mint authority)
- Stair-step initial liquidity deployment
- Token expiration and destruction mechanism
- Optional initial buy at market price

### Developer Resources
- Account structure diagrams
- PDA seed patterns
- Instruction parameters
- Mathematical formulas and invariants
- Sequence diagrams

## Using MCP for Development

### Common Use Cases

1. **Understanding Instructions**: Query specific instruction details, including parameters, accounts, and validation logic
2. **Architecture Review**: Explore system architecture and component interactions
3. **Integration Planning**: Learn about SDK usage and integration patterns
4. **Debugging**: Look up expected behavior and state transitions
5. **Code Examples**: Find implementation examples and sequence flows

### Best Practices

1. **Specific Queries**: Ask targeted questions about specific components rather than general overviews
2. **Context Building**: Start with high-level architecture before diving into implementation details
3. **Cross-Reference**: Use the MCP server to cross-reference related documentation sections
4. **Code Verification**: Verify understanding of instruction flows and account relationships

## Limitations

- The MCP server provides read-only access to documentation
- Documentation reflects the current state of the codebase and may not include unreleased features
- Some implementation details may require direct code inspection
- Real-time data (market prices, pool states) is not available through the documentation server

## Additional Resources

- **Main Repository**: The Feels Protocol codebase at `/Users/hxrts/projects/timewave/feels-solana/`
- **Program Code**: `programs/feels/` contains the on-chain program implementation
- **SDK**: `feels-sdk/` provides Rust SDK with examples
- **Frontend Documentation**: `feels-app/content/docs/` contains the source markdown files
- **README**: Project root README.md for build and test instructions

## Integration Examples

### Cursor Example

In a Cursor conversation, reference the MCP server:

```
@https://deepwiki.com/feelsdex/feels How does the swap instruction handle multi-hop routing?
```

### Development Workflow

1. Use MCP to understand the instruction you need to implement
2. Review the account structure and PDA seeds
3. Check validation rules and constraints
4. Examine sequence diagrams for state transitions
5. Implement with reference to SDK examples

## Technical Details

### Document Format
- Markdown format with frontmatter metadata
- Code examples in Rust, Mermaid diagrams for flows
- Mathematical notation using LaTeX syntax
- Tables for structured information

### Organization
- Numbered prefixes for ordering (001, 002, 200-series, 300-series)
- Categories for logical grouping
- Draft and searchable flags for content management

### Coverage
- **18 documentation files** in `feels-app/content/docs/`
- **4 blog posts** in `feels-app/content/blog/`
- **Multiple README files** across component directories
- **Inline code documentation** throughout the Rust codebase

## Support and Updates

The DeepWiki MCP server is automatically synchronized with the Feels Protocol documentation. As documentation is updated in the repository, the MCP server reflects those changes.

For issues or questions about the MCP integration:
- Check your MCP client's connection status
- Verify network access to `deepwiki.com`
- Consult your IDE's MCP configuration documentation
- Review the Model Context Protocol specification

## Conclusion

The DeepWiki MCP integration provides efficient, AI-assisted access to comprehensive Feels Protocol documentation directly within your development environment. By leveraging MCP, developers can quickly query protocol specifications, understand implementation details, and access examples without leaving their IDE.

The documentation covers everything from basic concepts to detailed technical specifications, making it a valuable resource throughout the development lifecycle.

