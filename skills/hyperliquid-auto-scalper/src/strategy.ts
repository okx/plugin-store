import { getSignal } from "./indicators"
import { executeBatch } from "./execution"
import { getRotationList } from "./rotation"
import { checkFunding } from "./risk"

export async function runStrategy(input:any, ctx:any){
  const {hyperliquid, market} = ctx
  const symbols = getRotationList()

  for(const symbol of symbols){
    const prices = await market.getKlines(symbol)
    if(!prices || prices.length < 20) continue

    if(!(await checkFunding(market, symbol))) continue

    const signal = getSignal(prices)
    if(!signal) continue

    const size = 0.002

    await executeBatch(hyperliquid, symbol, "buy", size)
    await executeBatch(hyperliquid, symbol, "sell", size)
  }

  return { message: "Strategy executed" }
}