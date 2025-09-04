pub const PROTOCOL_PDA_SEED: &[u8] = b"protocol";
pub const TREASURY_PDA_SEED: &[u8] = b"treasury";
pub const FACTORY_PDA_SEED: &[u8] = b"factory";
pub const FEELSSOL_PDA_SEED: &[u8] = b"feelssol";
pub const VAULT_PDA_SEED: &[u8] = b"vault";
pub const KEEPER_PDA_SEED: &[u8] = b"keeper";

pub const PROTOCOL_PROGRAM_PATH: &str = "../../target/deploy/feels_protocol.so";
pub const FACTORY_PROGRAM_PATH: &str = "../../target/deploy/feels_token_factory.so";
pub const FEELSSOL_PROGRAM_PATH: &str = "../../target/deploy/feelssol_controller.so";
pub const KEEPER_PROGRAM_PATH: &str = "../../target/deploy/feels_keeper.so";

pub const JITOSOL_MINT: &str = "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn";
pub const JITO_STAKE_POOL: &str = "Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb";

pub const TEST_KEYPAIR_PATH: &str = "../../test_keypair.json";
pub const PROTOCOL_KEYPAIR_PATH: &str = "../../target/deploy/feels_protocol-keypair.json";
pub const FACTORY_KEYPAIR_PATH: &str = "../../target/deploy/feels_token_factory-keypair.json";
pub const FEELSSOL_CONTROLLER_KEYPAIR_PATH: &str =
    "../../target/deploy/feelssol_controller-keypair.json";
pub const KEEPER_KEYPAIR_PATH: &str = "../../target/deploy/feels_keeper-keypair.json";

// Example secret key that gives a Pubkey that starts with `Fee1s`.
pub const FEELS_PRIVATE_KEY: [u8; 32] = [
    208, 250, 243, 217, 178, 15, 248, 65, 233, 94, 242, 229, 196, 92, 156, 153, 172, 164, 14, 45,
    147, 20, 212, 158, 3, 235, 20, 9, 75, 178, 205, 35,
];
