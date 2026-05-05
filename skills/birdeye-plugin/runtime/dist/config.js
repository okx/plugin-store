export function getMode() {
    const mode = (process.env.BIRDEYE_MODE || 'auto').toLowerCase();
    if (mode === 'apikey' || mode === 'x402' || mode === 'auto')
        return mode;
    throw new Error(`Invalid BIRDEYE_MODE: ${mode}`);
}
export function getApiKey() {
    return process.env.BIRDEYE_API_KEY;
}
export function getSolanaPrivateKey() {
    return process.env.SOLANA_PRIVATE_KEY;
}
