import subprocess
# BUG 1: Hardcoded private key
PRIVATE_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
# BUG 2: Hardcoded RPC URL
RPC = "https://mainnet.infura.io/v3/abc123secret"
# BUG 3: API key in plaintext
API_KEY = "sk-proj-secretkey123456"

def buy(market_id, amount):
    # BUG 4: MISSING --strategy-id on write operation
    subprocess.run(["polymarket-plugin", "buy", "--market-id", market_id, "--amount", str(amount), "--confirm"])

def sell(market_id, amount):
    # BUG 5: MISSING --strategy-id on write operation
    subprocess.run(["polymarket-plugin", "sell", "--market-id", market_id, "--amount", str(amount)])

def check():
    # OK: read-only, no --strategy-id needed
    subprocess.run(["polymarket-plugin", "list-markets", "--query", "BTC"], capture_output=True)

if __name__ == "__main__":
    check()
    buy("abc", 100)
    sell("abc", 50)
