export async function checkFunding(market: any, symbol: string): Promise<boolean> {
  try {
    const f = await market.getFundingRate(symbol);
    return Math.abs(f) < 0.01; // Only trade when funding rates are very low.
  } catch (error) {
    console.error(`Funding check failed for ${symbol}`);
    return false;
  }
}