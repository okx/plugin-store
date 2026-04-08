/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// DssCdpManager — CDP tracking contract
pub const CDP_MANAGER: &str = "0x5ef30b9986345249bc32d8928B7ee64DE9435E39";

/// Vat — core accounting engine
pub const VAT: &str = "0x35D1b3F3D7966A1DFe207aa4514C12a259A0492B";

/// Jug — stability fee accumulator
pub const JUG: &str = "0x19c0976f590D67707E62397C87829d896Dc0f1F";

/// ETH-A GemJoin adapter
pub const ETH_A_JOIN: &str = "0x2F0b23f53734252Bda2277357e97e1517d6B042A";

/// DaiJoin adapter
pub const DAI_JOIN: &str = "0x9759A6Ac90977b93B58547b4A71c78317f391A28";

/// DAI ERC-20 token
pub const DAI_TOKEN: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";

/// MCD Spot (oracle prices)
#[allow(dead_code)]
pub const MCD_SPOT: &str = "0x65C79fcB50Ca1594B025960e539eD7A9a6D434A7";

// --- Known ilk names and their bytes32 encodings (right-padded to 32 bytes) ---

/// ETH-A ilk: "ETH-A" as bytes32
pub const ILK_ETH_A: &str = "4554482d41000000000000000000000000000000000000000000000000000000";

/// WBTC-A ilk: "WBTC-A" as bytes32
pub const ILK_WBTC_A: &str = "574254432d410000000000000000000000000000000000000000000000000000";

/// USDC-A ilk: "USDC-A" as bytes32
pub const ILK_USDC_A: &str = "555344432d410000000000000000000000000000000000000000000000000000";

/// WSTETH-A ilk: "WSTETH-A" as bytes32
pub const ILK_WSTETH_A: &str = "5753544554482d4100000000000000000000000000000000000000000000000000";

/// All known ilks with human-readable names
pub const KNOWN_ILKS: &[(&str, &str)] = &[
    ("ETH-A", ILK_ETH_A),
    ("WBTC-A", ILK_WBTC_A),
    ("USDC-A", ILK_USDC_A),
    ("WSTETH-A", ILK_WSTETH_A),
];

// --- Function selectors ---

// DssCdpManager
pub const SEL_CDP_OPEN: &str = "6090dec5";         // open(bytes32,address)
pub const SEL_CDP_URNS: &str = "2726b073";         // urns(uint256) -> address urn
pub const SEL_CDP_ILKS: &str = "2c2cb9fd";         // ilks(uint256) -> bytes32 ilk
#[allow(dead_code)]
pub const SEL_CDP_OWNS: &str = "8161b120";         // owns(uint256) -> address owner
pub const SEL_CDP_FIRST: &str = "fc73d771";        // first(address) -> uint256
pub const SEL_CDP_COUNT: &str = "05d85eda";        // count(address) -> uint256
pub const SEL_CDP_LIST: &str = "80c9419e";         // list(uint256) -> (uint256 prev, uint256 next)
#[allow(dead_code)]
pub const SEL_CDP_CDPI: &str = "b3d178f2";         // cdpi() -> uint256

// Vat
pub const SEL_VAT_URNS: &str = "2424be5c";         // urns(bytes32,address) -> (uint256 ink, uint256 art)
pub const SEL_VAT_ILKS: &str = "d9638d36";         // ilks(bytes32) -> (Art,rate,spot,line,dust)
pub const SEL_VAT_FROB: &str = "76088703";         // frob(bytes32,address,address,address,int256,int256)

// Jug
pub const SEL_JUG_ILKS: &str = "d9638d36";         // ilks(bytes32) -> (uint256 duty, uint256 rho) [same selector, different contract]

// ERC-20
#[allow(dead_code)]
pub const SEL_BALANCE_OF: &str = "70a08231";       // balanceOf(address)
pub const SEL_APPROVE: &str = "095ea7b3";          // approve(address,uint256)

// DaiJoin
pub const SEL_DAIJOIN_JOIN: &str = "3b4da69f";     // join(address,uint256)
pub const SEL_DAIJOIN_EXIT: &str = "ef693bed";     // exit(address,uint256)

// EthJoin
pub const SEL_ETHJOIN_JOIN: &str = "28ffe6c8";     // join(address) payable
pub const SEL_ETHJOIN_EXIT: &str = "ef693bed";     // exit(address,uint256)
