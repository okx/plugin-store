import { execSync } from "child_process";

const args = process.argv.slice(2);

if (args[0] === "--query" && args[1] === "eth-price") {
  console.log("Querying ETH price via onchainos...");
  try {
    const result = execSync("onchainos token price ETH", { encoding: "utf-8" });
    console.log(result);
  } catch (e: any) {
    console.error("Error:", e.message);
  }
} else if (args[0] === "--help") {
  console.log("test-node-cli v1.0.0");
  console.log("Usage: test-node-cli --query eth-price");
  console.log("Queries ETH price via onchainos token price ETH");
} else {
  console.log("test-node-cli v1.0.0 - E2E test Node.js CLI");
  console.log("Run with --help for usage");
}
