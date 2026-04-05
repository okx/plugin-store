import { execSync } from "child_process";

const args = process.argv.slice(2);

if (args[0] === "--query" && args[1] === "eth-price") {
  console.log("Querying ETH price via onchainos...");
  try {
    const result = execSync("onchainos token price-info --address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --chain ethereum", { encoding: "utf-8" });
    console.log(result);
  } catch (e: any) {
    console.error("Error:", e.message);
  }
} else if (args[0] === "--help") {
  console.log("test-ts-cli v1.0.0");
  console.log("Usage: test-ts-cli --query eth-price");
  console.log("Queries ETH price via onchainos token price-info --address 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --chain ethereum");
} else {
  console.log("test-ts-cli v1.0.0 - E2E test TypeScript CLI");
  console.log("Run with --help for usage");
}
