import { getSignal } from "./indicators";
import { executeBatch } from "./execution";
import { getRotationList } from "./rotation";
import { checkFunding } from "./risk";

export async function runStrategy(input: any, ctx: any) {
  console.log("[HL Steady Scalper] Starting strategy run...");

  const symbols = getRotationList();
  let executed = 0;

  for (const symbol of symbols) {
    try {
      // Lấy dữ liệu giá
      const prices = await ctx.market?.getKlines(symbol);
      if (!prices || prices.length < 20) {
        console.log(`[HL Steady Scalper] Skipping ${symbol} - insufficient klines`);
        continue;
      }

      // Kiểm tra funding rate
      if (!(await checkFunding(ctx.market, symbol))) {
        console.log(`[HL Steady Scalper] Skipping ${symbol} - high funding rate`);
        continue;
      }

      const signal = getSignal(prices);
      if (!signal) continue;

      const size = 0.002;
      await executeBatch(ctx, symbol, signal, size);
      executed++;
    } catch (error) {
      console.error(`[HL Steady Scalper] Error processing ${symbol}:`, error);
    }
  }

  if (executed === 0) {
    return { message: "No trading signals found at the moment. Try again later." };
  }

  return { 
    message: `Strategy executed successfully! ${executed} coin(s) traded with micro-batches.` 
  };
}