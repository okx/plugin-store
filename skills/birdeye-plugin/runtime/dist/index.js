import { birdeyeGet } from './client.js';
function arg(name, fallback = '') {
    const i = process.argv.indexOf(`--${name}`);
    if (i === -1 || i + 1 >= process.argv.length)
        return fallback;
    return process.argv[i + 1];
}
async function main() {
    const cmd = process.argv[2];
    const chain = arg('chain', 'solana');
    if (cmd === 'price') {
        const address = arg('address');
        if (!address)
            throw new Error('--address is required');
        console.log(JSON.stringify(await birdeyeGet('/defi/price', { address }, chain), null, 2));
        return;
    }
    if (cmd === 'trending') {
        const limit = arg('limit', '20');
        console.log(JSON.stringify(await birdeyeGet('/defi/token_trending', { sort_by: 'rank', sort_type: 'asc', limit }, chain), null, 2));
        return;
    }
    if (cmd === 'overview') {
        const address = arg('address');
        if (!address)
            throw new Error('--address is required');
        console.log(JSON.stringify(await birdeyeGet('/defi/token_overview', { address }, chain), null, 2));
        return;
    }
    if (cmd === 'security') {
        const address = arg('address');
        if (!address)
            throw new Error('--address is required');
        console.log(JSON.stringify(await birdeyeGet('/defi/token_security', { address }, chain), null, 2));
        return;
    }
    throw new Error('Usage: [price|trending|overview|security] --chain solana [--address <token>] [--limit 20]');
}
main().catch((e) => {
    console.error(e.message || String(e));
    process.exit(1);
});
