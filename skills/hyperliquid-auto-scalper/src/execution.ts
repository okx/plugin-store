export async function executeBatch(hyperliquid:any, symbol:string, side:string, size:number){
  const parts = 10
  const s = size / parts

  for(let i = 0; i < parts; i++){
    await hyperliquid.placeOrder({
      symbol,
      side,
      type: "market",
      size: s,
      leverage: 2,
      strategyId: "hyperliquid-auto-scalper"
    })
  }
}