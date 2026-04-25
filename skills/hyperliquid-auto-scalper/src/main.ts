import { runStrategy } from "./strategy";

export async function main(input: any, ctx: any) {
  const text = (input?.text || "").toLowerCase().trim();

  // Mode 1: One-time run
  if (text.includes("run") || text.includes("scalper") || text.includes("steady")) {
    return await runStrategy(input, ctx);
  }

  // Mode 2: Start continuous loop
  if (text.includes("start loop") || text.includes("24/7") || text.includes("continuous")) {
    return startContinuousMode(ctx);
  }

  return { 
    message: "HL Steady Scalper is ready!\n\n" +
             "• One-time: **Run scalper**\n" +
             "• Continuous (every 5 min): **Start loop** or **Run 24/7**" 
  };
}

// Continuous mode
async function startContinuousMode(ctx: any) {
  console.log("[HL Steady Scalper] Starting continuous mode (every 5 minutes)...");

  const intervalMs = 5 * 60 * 1000; // 5 phút

  const interval = setInterval(async () => {
    try {
      await runStrategy({ text: "run" }, ctx);
    } catch (e) {
      console.error("Loop error:", e);
    }
  }, intervalMs);

  // Giữ reference để có thể stop sau
  (ctx as any).scalperInterval = interval;

  return { 
    message: "✅ Continuous mode started! Strategy will run every 5 minutes.\n" +
             "Say 'stop loop' to stop." 
  };
}