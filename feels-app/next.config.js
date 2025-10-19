/** @type {import('next').NextConfig} */
const webpack = require('webpack');
const ChunkLoadErrorFixPlugin = require('./src/utils/webpack/chunk-fix-plugin');

const nextConfig = {
  // Disable strict mode for better Solana wallet compatibility
  reactStrictMode: false,
  webpack: (config, { isServer, dev, webpack }) => {
    // Handle server-side externals and problematic modules
    if (isServer) {
      // Externalize ws and other node modules on server to prevent bundling issues
      config.externals = [...(config.externals || []), 'ws', 'bufferutil', 'utf-8-validate', 'mermaid'];
      
      // Mock ws and mermaid for server-side rendering where they're not needed
      config.resolve.alias = {
        ...config.resolve.alias,
        'ws': require('path').resolve(__dirname, './src/utils/webpack/ws-mock.js'),
        'mermaid': require('path').resolve(__dirname, './src/utils/webpack/mermaid-mock.js'),
      };
    }
    
    // Fix for vendor chunk issues with Solana/Anchor dependencies
    if (!isServer) {
      // Ensure problematic dependencies are properly resolved
      config.resolve.alias = {
        ...config.resolve.alias,
        'eventemitter3': require.resolve('eventemitter3'),
        '@solana/web3.js': require.resolve('@solana/web3.js'),
        '@coral-xyz/anchor': require.resolve('@coral-xyz/anchor'),
        '@project-serum/anchor': require.resolve('@project-serum/anchor'),
      };
    }
    if (!isServer) {
      // Provide fallbacks for node modules in the browser
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
        net: false,
        tls: false,
        crypto: require.resolve('crypto-browserify'),
        stream: require.resolve('stream-browserify'),
        path: require.resolve('path-browserify'),
        zlib: require.resolve('browserify-zlib'),
        os: require.resolve('os-browserify/browser'),
        http: require.resolve('stream-http'),
        https: require.resolve('https-browserify'),
        assert: require.resolve('assert'),
        buffer: require.resolve('buffer'),
        process: require.resolve('process/browser'),
        util: require.resolve('util'),
        url: require.resolve('url'),
        ws: false,  // Disable ws module for client-side
      };
      
      // Add alias for process and ws
      config.resolve.alias = {
        ...config.resolve.alias,
        process: 'process/browser',
        ws: require('path').resolve(__dirname, './src/utils/ws-mock.js'),
      };
    }
    
    // Add rules for mjs and cjs files
    config.module.rules.push({
      test: /\.m?js/,
      resolve: {
        fullySpecified: false,
      },
    });

    // Add WASM support
    config.experiments = {
      ...config.experiments,
      asyncWebAssembly: true,
      layers: true,
    };

    // Handle WASM files
    config.module.rules.push({
      test: /\.wasm$/,
      type: 'webassembly/async',
    });
    
    // Copy WASM files to static directory for worker access
    if (!isServer) {
      config.module.rules.push({
        test: /vanity_miner_wasm_bg\.wasm$/,
        type: 'asset/resource',
        generator: {
          filename: 'static/wasm/[name][ext]',
        },
      });
    }


    // Workaround for Next.js webpack runtime issues
    if (!isServer) {
      // Ensure webpack runtime doesn't try to dynamically load chunks that don't exist
      config.output.chunkLoadingGlobal = 'webpackChunkFeelsApp';
      
      // Add publicPath to ensure chunks are loaded from the correct location
      if (!config.output.publicPath) {
        config.output.publicPath = '/_next/';
      }
      
      // Configure chunk filename to be more predictable
      config.output.chunkFilename = dev 
        ? 'static/chunks/[name].js'
        : 'static/chunks/[name].[contenthash].js';
    }

    // Use Next.js default optimization to avoid CSS/JS mixing issues
    if (!isServer) {
      // Only set essential optimization options, let Next.js handle chunk splitting
      config.optimization = {
        ...config.optimization,
        // Let Next.js handle splitChunks properly to avoid CSS/JS conflicts
      };
    }
    
    
    // Add webpack plugins - but only in production to avoid chunk issues
    if (!dev) {
      config.plugins.push(
        new webpack.ProvidePlugin({
          Buffer: ['buffer', 'Buffer'],
          process: 'process/browser',
        })
      );
      
      // Define process.env for browser - only include NEXT_PUBLIC_ variables
      const env = {};
      Object.keys(process.env).forEach(key => {
        if (key.startsWith('NEXT_PUBLIC_')) {
          env[`process.env.${key}`] = JSON.stringify(process.env[key]);
        }
      });
      if (Object.keys(env).length > 0) {
        config.plugins.push(
          new webpack.DefinePlugin(env)
        );
      }
    } else {
      // In dev, provide Buffer globally and disable Lit dev mode
      config.plugins.push(
        new webpack.ProvidePlugin({
          Buffer: ['buffer', 'Buffer'],
        }),
        new webpack.DefinePlugin({
          'globalThis.litDev': JSON.stringify(false),
          'window.litDev': JSON.stringify(false),
        })
      );
    }
    
    // Ignore pino-pretty which is only used in development
    config.plugins.push(
      new webpack.IgnorePlugin({
        resourceRegExp: /^pino-pretty$/,
        contextRegExp: /pino/,
      })
    );
    
    // Add custom chunk fix plugin for client builds
    if (!isServer) {
      config.plugins.push(new ChunkLoadErrorFixPlugin());
    }
    
    // Ignore React Native modules which are not needed in web
    config.plugins.push(
      new webpack.IgnorePlugin({
        resourceRegExp: /^react-native$/,
      }),
      new webpack.IgnorePlugin({
        resourceRegExp: /^@react-native-async-storage\/async-storage$/,
      })
    );
    
    return config;
  },
  // Set output file tracing root to silence workspace detection warning
  outputFileTracingRoot: __dirname,
  
  // Optimize for production builds with Solana
  experimental: {
    // esmExternals removed as it's deprecated in Next.js 15
  },
  transpilePackages: [
    '@solana/web3.js',
    '@solana/wallet-adapter-base',
    '@solana/wallet-adapter-react',
    '@solana/wallet-adapter-wallets',
    '@coral-xyz/anchor',
  ],
  // Proxy API requests to avoid CORS issues in development
  async rewrites() {
    return [
      {
        source: '/api/indexer/:path*',
        destination: 'http://localhost:8080/:path*',
      },
    ];
  },

  // Headers for SharedArrayBuffer support
  async headers() {
    return [
      {
        // Apply COEP/COOP headers to all pages for multi-threading support
        source: '/:path*',
        headers: [
          {
            key: 'Cross-Origin-Embedder-Policy',
            value: 'credentialless',
          },
          {
            key: 'Cross-Origin-Opener-Policy',
            value: 'same-origin',
          },
        ],
      },
      {
        // Apply headers to WASM files
        source: '/wasm/:path*',
        headers: [
          {
            key: 'Cross-Origin-Resource-Policy',
            value: 'cross-origin',
          },
          {
            key: 'Cache-Control',
            value: process.env.NODE_ENV === 'development' 
              ? 'no-store, must-revalidate' 
              : 'public, max-age=31536000, immutable',
          },
        ],
      },
    ];
  },

  images: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'raw.githubusercontent.com',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'arweave.net',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'www.arweave.net',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: '**.ipfs.nftstorage.link',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'cdn.jsdelivr.net',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'storage.googleapis.com',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'cryptologos.cc',
        pathname: '/**',
      },
      {
        protocol: 'https',
        hostname: 'assets.coingecko.com',
        pathname: '/**',
      },
    ],
  },
};

module.exports = nextConfig;