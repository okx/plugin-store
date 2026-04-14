"""
One-click token launch v1.0 -- pump.fun adapter (via PumpPortal API).

Flow:
  1. Generate mint keypair (protocol requirement)
  2. Get unsigned TX from PumpPortal /api/trade-local
     - User wallet is fee payer (publicKey)
     - Mint keypair secret is passed so PumpPortal includes its signature
  3. Sign via onchainos TEE wallet (adds fee payer signature)
  4. Wait for confirmation

All signing goes through onchainos Agentic Wallet (TEE).
No local private key signing of the full transaction.

PumpPortal docs: https://pumpportal.fun/creation/
"""
from __future__ import annotations

import asyncio
import json

import httpx

import config as C
from .base import LaunchpadAdapter, LaunchParams, LaunchResult, onchainos_bin

_SOLANA_EXPLORER = "https://solscan.io/tx"
_PUMPFUN_TRADE = "https://pump.fun"


class PumpFunAdapter(LaunchpadAdapter):

    @property
    def name(self) -> str:
        return "pumpfun"

    @property
    def display_name(self) -> str:
        return "pump.fun"

    @property
    def chain(self) -> str:
        return "solana"

    def _fee_estimate(self, params: LaunchParams) -> float:
        pf = params.extras.get("priority_fee", C.PUMPFUN_PRIORITY_FEE)
        return pf + 0.015  # priority fee + rent

    async def launch(self, params: LaunchParams) -> LaunchResult:
        """Launch a token on pump.fun via PumpPortal."""

        if C.DRY_RUN:
            return LaunchResult(
                success=True,
                token_address="DRY_RUN_NO_TOKEN_ADDRESS",
                tx_hash="DRY_RUN_NO_TX_HASH",
                error="DRY_RUN mode -- no on-chain TX sent",
            )

        from solders.keypair import Keypair as SoldersKeypair

        # -- 1. Generate mint keypair (pump.fun protocol requirement) ------
        mint_kp = SoldersKeypair()
        mint_pubkey = str(mint_kp.pubkey())
        mint_secret = str(mint_kp)  # base58 full keypair for PumpPortal

        print(f"  [pump.fun] Mint address: {mint_pubkey}")

        # -- 2. Get unsigned TX from PumpPortal ---------------------------
        pool = params.extras.get("pool", C.PUMPFUN_POOL)
        priority_fee = params.extras.get("priority_fee", C.PUMPFUN_PRIORITY_FEE)

        # PumpPortal signs with the mint keypair when we pass the secret.
        # The returned TX only needs the fee payer (user wallet) signature.
        create_payload = {
            "publicKey": params.wallet_address,   # user wallet = fee payer
            "action": "create",
            "tokenMetadata": {
                "name": params.name,
                "symbol": params.symbol,
                "uri": params.metadata_uri,
            },
            "mint": mint_secret,       # full keypair -- PumpPortal adds mint signature
            "denominatedInSol": "true",
            "amount": params.buy_amount,
            "slippage": params.slippage_bps / 100,
            "priorityFee": priority_fee,
            "pool": pool,
        }

        async with httpx.AsyncClient(timeout=C.PUMPFUN_TX_TIMEOUT) as client:
            resp = await client.post(
                f"{C.PUMPFUN_API_BASE}/api/trade-local",
                headers={"Content-Type": "application/json"},
                json=create_payload,
            )

        if resp.status_code != 200:
            return LaunchResult(
                success=False,
                error=f"PumpPortal API error {resp.status_code}: {resp.text}",
            )

        # PumpPortal returns the TX as bytes (or base58 string)
        tx_data = resp.content

        if not tx_data:
            return LaunchResult(
                success=False,
                error="PumpPortal returned empty transaction",
            )

        # -- 3. Sign and broadcast via onchainos TEE wallet ---------------
        print("  [pump.fun] Signing via TEE wallet...")
        import base58
        tx_b58 = base58.b58encode(tx_data).decode()

        tx_hash = await self._sign_and_broadcast(
            tx_b58, params.wallet_address, mint_pubkey,
            mev_protection=params.mev_protection,
        )

        if not tx_hash:
            return LaunchResult(
                success=False,
                error="Failed to sign/broadcast via onchainos wallet",
            )

        # -- 4. Wait for confirmation -------------------------------------
        print(f"  [pump.fun] TX submitted: {tx_hash}")
        print("  [pump.fun] Waiting for confirmation...")

        confirmed = await self._wait_confirmation(tx_hash, params.wallet_address)

        return LaunchResult(
            success=confirmed,
            token_address=mint_pubkey,
            tx_hash=tx_hash,
            explorer_url=f"{_SOLANA_EXPLORER}/{tx_hash}",
            trade_page_url=f"{_PUMPFUN_TRADE}/{mint_pubkey}",
            error="" if confirmed else "Transaction not confirmed within timeout",
        )

    async def _sign_and_broadcast(
        self, unsigned_tx_b58: str, wallet_address: str,
        to_address: str = "", mev_protection: bool = False,
    ) -> str:
        """Sign unsigned TX via onchainos TEE wallet and broadcast.

        Returns TX hash on success, empty string on failure.
        """
        try:
            cmd = [
                onchainos_bin(), "wallet", "contract-call",
                "--chain", "501",
                "--to", to_address or wallet_address,
                "--unsigned-tx", unsigned_tx_b58,
            ]
            if mev_protection:
                cmd.append("--mev-protection")
                cmd.extend(["--jito-unsigned-tx", unsigned_tx_b58])

            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()

            if proc.returncode != 0:
                err = stderr.decode().strip() if stderr else "unknown error"
                print(f"  [pump.fun] contract-call failed: {err}")
                return ""

            output = json.loads(stdout.decode())
            data = output.get("data", {})
            if isinstance(data, list) and data:
                data = data[0]
            return data.get("txHash", "") or output.get("txHash", "")

        except Exception as e:
            print(f"  [pump.fun] Sign/broadcast error: {e}")
            return ""

    async def _wait_confirmation(
        self, tx_hash: str, wallet_address: str, max_retries: int = 15,
    ) -> bool:
        """Poll for TX confirmation via onchainos wallet history."""
        delays = [0.5, 0.5, 1, 1] + [2] * (max_retries - 4)
        for i in range(max_retries):
            await asyncio.sleep(delays[i] if i < len(delays) else 2)
            try:
                cmd = [
                    onchainos_bin(), "wallet", "history",
                    "--chain", "501",
                    "--tx-hash", tx_hash,
                    "--address", wallet_address,
                ]
                proc = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, _ = await proc.communicate()
                output = json.loads(stdout.decode())
                data = output.get("data", {})
                if isinstance(data, list) and data:
                    data = data[0]
                status = data.get("status", "") or data.get("txStatus", "")

                if status in ("confirmed", "finalized", "success"):
                    print(f"  [pump.fun] {status.capitalize()}! ({i + 1} polls)")
                    return True
                if "fail" in str(status).lower() or "error" in str(status).lower():
                    print(f"  [pump.fun] TX failed: {status}")
                    return False
            except Exception:
                pass
            if (i + 1) % 3 == 0:
                print(f"  [pump.fun] Waiting for confirmation... ({i + 1}/{max_retries})")
        return False
