"""
一键发币 v1.0 — LetsBonk adapter.

Flow:
  1. Create token via LetsBonk API (or PumpPortal with pool="bonk")
  2. Optional bundled initial buy
  3. Sign via onchainos wallet
  4. Submit and wait for confirmation

LetsBonk has two integration paths:
  A. Native LetsBonk API (if available)
  B. PumpPortal with pool="bonk" (fallback — proven to work)

We implement Path B as the primary path since PumpPortal is well-documented
and supports LetsBonk pools via the `pool` parameter.

Ref: https://github.com/letsbonk-ai/bonk-mcp
"""
from __future__ import annotations

import asyncio
import json

import httpx

import config as C
from .base import LaunchpadAdapter, LaunchParams, LaunchResult, onchainos_bin

_SOLANA_EXPLORER = "https://solscan.io/tx"
_LETSBONK_TRADE = "https://letsbonk.fun/token"


class LetsBonkAdapter(LaunchpadAdapter):

    @property
    def name(self) -> str:
        return "letsbonk"

    @property
    def display_name(self) -> str:
        return "LetsBonk"

    @property
    def chain(self) -> str:
        return "solana"

    def _fee_estimate(self, params: LaunchParams) -> float:
        pf = params.extras.get("priority_fee", C.LETSBONK_PRIORITY_FEE)
        return pf + 0.01  # priority + rent/fees

    async def launch(self, params: LaunchParams) -> LaunchResult:
        """Launch a token on LetsBonk via PumpPortal (pool=bonk)."""

        if C.DRY_RUN:
            return LaunchResult(
                success=True,
                token_address="DRY_RUN_BONK_NO_TOKEN",
                tx_hash="DRY_RUN_BONK_NO_TX",
                error="DRY_RUN mode — no on-chain TX sent",
            )

        # ── 1. Generate mint keypair ──────────────────────────────────
        mint_keypair = await self._generate_mint_keypair()
        mint_pubkey = mint_keypair["pubkey"]
        mint_secret = mint_keypair["secret"]

        print(f"  [LetsBonk] Mint address: {mint_pubkey}")

        # ── 2. Build create TX via PumpPortal (pool=bonk) ─────────────
        priority_fee = params.extras.get("priority_fee", C.LETSBONK_PRIORITY_FEE)

        create_payload = {
            "publicKey": params.wallet_address,
            "action": "create",
            "tokenMetadata": {
                "name": params.name,
                "symbol": params.symbol,
                "uri": params.metadata_uri,
            },
            "mint": mint_secret,
            "denominatedInSol": "true",
            "amount": params.buy_amount,
            "slippage": params.slippage_bps / 100,
            "priorityFee": priority_fee,
            "pool": "bonk",  # This routes to LetsBonk instead of pump.fun
        }

        async with httpx.AsyncClient(timeout=30) as client:
            resp = await client.post(
                f"{C.PUMPFUN_API_BASE}/api/trade-local",
                json=create_payload,
            )

        if resp.status_code != 200:
            return LaunchResult(
                success=False,
                error=f"PumpPortal API error {resp.status_code}: {resp.text}",
            )

        tx_data = resp.content

        # ── 3. Sign and submit ────────────────────────────────────────
        print("  [LetsBonk] Signing and submitting...")
        tx_hash = await self._sign_and_submit(tx_data, params.wallet_address, mint_pubkey)

        if not tx_hash:
            return LaunchResult(
                success=False,
                error="Failed to sign/submit via onchainos wallet",
            )

        # ── 4. Wait for confirmation ──────────────────────────────────
        print(f"  [LetsBonk] TX submitted: {tx_hash}")
        confirmed = await self._wait_confirmation(tx_hash, params.wallet_address)

        return LaunchResult(
            success=confirmed,
            token_address=mint_pubkey,
            tx_hash=tx_hash,
            explorer_url=f"{_SOLANA_EXPLORER}/{tx_hash}",
            trade_page_url=f"{_LETSBONK_TRADE}/{mint_pubkey}",
            error="" if confirmed else "Transaction not confirmed within timeout",
        )

    async def _generate_mint_keypair(self) -> dict:
        """Generate a random Solana Ed25519 keypair."""
        try:
            from solders.keypair import Keypair as SoldersKeypair
            kp = SoldersKeypair()
            return {"pubkey": str(kp.pubkey()), "secret": str(kp)}
        except ImportError:
            pass
        try:
            from nacl.signing import SigningKey
            import base58
            sk = SigningKey.generate()
            full_key = sk.encode() + sk.verify_key.encode()
            return {
                "pubkey": base58.b58encode(sk.verify_key.encode()).decode(),
                "secret": base58.b58encode(full_key).decode(),
            }
        except ImportError:
            raise RuntimeError("Install solders or pynacl+base58 for keypair generation")

    async def _sign_and_submit(self, tx_data: bytes, wallet_address: str, mint_pubkey: str = "") -> str:
        """Sign and submit unsigned TX via onchainos TEE wallet."""
        import base58 as b58
        tx_b58 = b58.b58encode(tx_data).decode()
        try:
            cmd = [
                onchainos_bin(), "wallet", "contract-call",
                "--chain", "501",
                "--to", mint_pubkey or wallet_address,
                "--unsigned-tx", tx_b58,
                "--biz-type", "dex",
                "--strategy", "one-click-token-launch",
            ]
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode != 0:
                print(f"  [LetsBonk] contract-call failed: {stderr.decode().strip()}")
                return ""
            output = json.loads(stdout.decode())
            data = output.get("data", {})
            if isinstance(data, list) and data:
                data = data[0]
            return data.get("txHash", "") or output.get("txHash", "")
        except Exception as e:
            print(f"  [LetsBonk] Sign/submit error: {e}")
            return ""

    async def _wait_confirmation(self, tx_hash: str, wallet_address: str, max_retries: int = 5) -> bool:
        """Poll for TX confirmation via onchainos wallet history."""
        for i in range(max_retries):
            await asyncio.sleep(5)
            try:
                proc = await asyncio.create_subprocess_exec(
                    onchainos_bin(), "wallet", "history",
                    "--chain", "501",
                    "--tx-hash", tx_hash,
                    "--address", wallet_address,
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
                    print(f"  [LetsBonk] Confirmed! ({i + 1} polls)")
                    return True
            except Exception:
                pass
        return False
