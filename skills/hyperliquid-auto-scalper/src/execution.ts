/**
 * Execute micro-batch orders through Hyperliquid Plugin (contest compliant)
 */
export async function executeBatch(ctx: any, symbol: string, side: string, size: number = 0.002) {
  const parts = 10;
  const singleSize = (size / parts).toFixed(4);
  const coin = symbol.replace("-PERP", ""); // BTC-PERP → BTC

  const orders = Array.from({ length: parts }, () => ({
    coin,
    side,
    size: singleSize,
    type: "market"
  }));

  const jsonOrders = JSON.stringify(orders, null, 2);

  const command = `hyperliquid order-batch --orders-json - --strategy-id hyperliquid-auto-scalper --confirm << 'EOF'
${jsonOrders}
EOF`;

  console.log(`[HL Steady Scalper] Executing ${parts} micro orders for ${coin} ${side.toUpperCase()}`);

  try {
    const result = await ctx.executeCommand(command);
    console.log(`[HL Steady Scalper] Batch executed for ${coin}`);
    return result;
  } catch (error) {
    console.error(`[HL Steady Scalper] Batch failed for ${coin}:`, error);
    return null;
  }
}