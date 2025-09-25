/** @type {import('next').NextConfig} */
const webpack = require('webpack');

const nextConfig = {
  // Disable strict mode for better Solana wallet compatibility
  reactStrictMode: false,
  webpack: (config, { isServer, dev, webpack }) => {
    // Handle server-side externals and problematic modules
    if (isServer) {
      // Don't externalize ws on server since it's needed
      config.externals = [...(config.externals || [])];
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

    // Configure optimization for better chunk splitting
    if (!isServer) {
      config.optimization = {
        ...config.optimization,
        splitChunks: {
          chunks: 'all',
          cacheGroups: {
            default: false,
            vendors: false,
            // Solana and related packages in their own chunk
            solana: {
              test: /[\\/]node_modules[\\/](@solana|@coral-xyz|@project-serum)[\\/]/,
              name: 'solana',
              priority: 10,
              reuseExistingChunk: true,
            },
            // Common packages
            commons: {
              name: 'commons',
              minChunks: 2,
              priority: 5,
              reuseExistingChunk: true,
            },
            // Framework and large dependencies
            framework: {
              test: /[\\/]node_modules[\\/](react|react-dom|next)[\\/]/,
              name: 'framework',
              priority: 15,
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
  // Re-enabled SWC minification for better performance
  swcMinify: true,
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