export function getRSI(prices:number[]){
  let g = 0, l = 0
  for(let i=1;i<prices.length;i++){
    const d = prices[i] - prices[i-1]
    if(d>0) g+=d
    else l-=d
  }
  const rs = g/(l||1)
  return 100 - 100/(1+rs)
}

export function getSignal(prices:number[]){
  const rsi = getRSI(prices)
  if(rsi < 30) return "buy"
  if(rsi > 70) return "sell"
  return null
}