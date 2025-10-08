/** @type {import('next').NextConfig} */
const webpack = require('webpack');
const ChunkLoadErrorFixPlugin = require('./src/utils/webpack-chunk-fix-plugin');

const nextConfig = {
  // Disable strict mode for better Solana wallet compatibility
  reactStrictMode: false,
  webpack: (config, { isServer, dev, webpack }) => {
    // Handle server-side externals and problematic modules
    if (isServer) {
      // Externalize ws and other node modules on server to prevent bundling issues
      config.externals = [...(config.externals || []), 'ws', 'bufferutil', 'utf-8-validate'];
      
      // Mock ws for server-side rendering where it's not needed
      config.resolve.alias = {
        ...config.resolve.alias,
        'ws': require('path').resolve(__dirname, './src/utils/ws-mock.js'),
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

    // Configure optimization for better chunk splitting
    if (!isServer) {
      config.optimization = {
        ...config.optimization,
        moduleIds: 'deterministic',
        splitChunks: {
          chunks: 'all',
          maxAsyncRequests: 30,
          maxInitialRequests: 30,
          cacheGroups: {
            default: {
              minChunks: 2,
              priority: -20,
              reuseExistingChunk: true,
            },
            // Handle CSS separately to prevent JS/CSS mixing
            styles: {
              test: /\.(css|scss|sass)$/,
              enforce: true,
              priority: 20,
            },
            // Keep all vendor chunks in a stable vendor bundle
            // Solana packages - group all @solana packages together
            'vendor-solana': {
              test: /[\\/]node_modules[\\/](@solana|@coral-xyz|@project-serum)[\\/]/,
              name: 'vendor-solana',
              priority: 30,
              reuseExistingChunk: true,
            },
            // Wallet adapter packages
            'vendor-wallet': {
              test: /[\\/]node_modules[\\/].*wallet-adapter.*[\\/]/,
              name: 'vendor-wallet',
              priority: 25,
              reuseExistingChunk: true,
            },
            // React/Next packages
            'vendor-react': {
              test: /[\\/]node_modules[\\/](react|next)[\\/]/,
              name: 'vendor-react',
              priority: 20,
              reuseExistingChunk: true,
            },
            // All other vendor modules
            'vendor-common': {
              test: /[\\/]node_modules[\\/]/,
              name: 'vendor-common',
              priority: 10,
              reuseExistingChunk: true,
            },
          },
        },
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
      // In dev, provide Buffer globally
      config.plugins.push(
        new webpack.ProvidePlugin({
          Buffer: ['buffer', 'Buffer'],
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
  // Optimize for production builds with Solana
  experimental: {
    esmExternals: 'loose',
  },
  // Temporarily disable SWC minification to fix CSS issues
  swcMinify: false,
  transpilePackages: [
    '@solana/web3.js',
    '@solana/wallet-adapter-base',
    '@solana/wallet-adapter-react',
    '@solana/wallet-adapter-wallets',
    '@coral-xyz/anchor',
  ],
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