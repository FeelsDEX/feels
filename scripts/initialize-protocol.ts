import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
const FEELS_IDL = require("../target/idl/feels.json");

// Protocol seeds
const PROTOCOL_CONFIG_SEED = Buffer.from("protocol_config");
const PROTOCOL_ORACLE_SEED = Buffer.from("protocol_oracle");
const SAFETY_CONTROLLER_SEED = Buffer.from("safety_controller");

async function initializeProtocol() {
  // Connect to local validator
  const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  
  // Use the local keypair
  const wallet = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(
      require("fs").readFileSync(
        require("os").homedir() + "/.config/solana/id.json",
        "utf-8"
      )
    ))
  );
  
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(wallet),
    { commitment: "confirmed" }
  );
  
  // Create program interface
  const programId = new PublicKey("Cbv2aa2zMJdwAwzLnRZuWQ8efpr6Xb9zxpJhEzLe3v6N");
  const program = new Program(FEELS_IDL as any, programId, provider);
  
  console.log("Initializing protocol...");
  console.log("Authority:", wallet.publicKey.toString());
  
  // Derive PDAs
  const [protocolConfigPDA] = PublicKey.findProgramAddressSync(
    [PROTOCOL_CONFIG_SEED],
    program.programId
  );
  
  const [protocolOraclePDA] = PublicKey.findProgramAddressSync(
    [PROTOCOL_ORACLE_SEED],
    program.programId
  );
  
  const [safetyControllerPDA] = PublicKey.findProgramAddressSync(
    [SAFETY_CONTROLLER_SEED],
    program.programId
  );
  
  // Check if already initialized
  try {
    const existing = await connection.getAccountInfo(protocolConfigPDA);
    if (existing) {
      console.log("Protocol already initialized!");
      return;
    }
  } catch (e) {
    // Account doesn't exist, proceed with initialization
  }
  
  // Initialize protocol
  try {
    const tx = await (program.methods as any).initializeProtocol({
      mintFee: new anchor.BN(1_000_000), // 0.001 FeelsSOL
      treasury: wallet.publicKey,
      defaultProtocolFeeRate: 8, // 8 basis points
      defaultCreatorFeeRate: 2, // 2 basis points
      maxProtocolFeeRate: 50, // 50 basis points max
      tokenExpirationSeconds: new anchor.BN(3600), // 1 hour
      dexTwapUpdater: wallet.publicKey,
      depegThresholdBps: 50, // 0.5%
      depegRequiredObs: 3,
      clearRequiredObs: 5,
      dexTwapWindowSecs: new anchor.BN(900), // 15 minutes
      dexTwapStaleAgeSecs: new anchor.BN(1800), // 30 minutes
      mintPerSlotCapFeelssol: new anchor.BN(0), // No cap
      redeemPerSlotCapFeelssol: new anchor.BN(0), // No cap
    })
    .accounts({
      authority: wallet.publicKey,
      protocolConfig: protocolConfigPDA,
      protocolOracle: protocolOraclePDA,
      safetyController: safetyControllerPDA,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
    
    console.log("Protocol initialized successfully!");
    console.log("Transaction:", tx);
    console.log("Protocol Config:", protocolConfigPDA.toString());
    console.log("Protocol Oracle:", protocolOraclePDA.toString());
    console.log("Safety Controller:", safetyControllerPDA.toString());
  } catch (error) {
    console.error("Failed to initialize protocol:", error);
  }
}

initializeProtocol().catch(console.error);