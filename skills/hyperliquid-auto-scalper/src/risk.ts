export async function checkFunding(market:any, symbol:string){
  const f = await market.getFundingRate(symbol)
  return Math.abs(f) < 0.01
}