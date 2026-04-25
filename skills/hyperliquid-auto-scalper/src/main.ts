import { runStrategy } from "./strategy"

export async function main(input:any, ctx:any){
  const text = (input?.text || "").toLowerCase()

  if(text.includes("run")){
    return runStrategy(input, ctx)
  }

  return { message: "Say 'run scalper'" }
}