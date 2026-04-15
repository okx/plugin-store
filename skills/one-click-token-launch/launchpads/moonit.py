"""
一键发币 v1.0 — Moonit adapter (Official SDK / REST API).

Flow:
  1. Prepare mint TX via Moonit API (prepareMintTx equivalent)
  2. Sign via onchainos wallet
  3. Submit via Moonit API (submitMintTx equivalent)
  4. Wait for confirmation

SDK ref: https://github.com/gomoonit/moonit-sdk
"""
from __future__ import annotations

import asyncio
import json

import httpx

import config as C
from .base import LaunchpadAdapter, LaunchParams, LaunchResult, onchainos_bin

_SOLANA_EXPLORER = "https://solscan.io/tx"
_MOONIT_TRADE = "https://moon.it/token"


class MoonitAdapter(LaunchpadAdapter):

    @property
    def name(self) -> str:
        return "moonit"

    @property
    def display_name(self) -> str:
        return "Moonit"

    @property
    def chain(self) -> str:
        return "solana"

    def _fee_estimate(self, params: LaunchParams) -> float:
        return 0.015  # Moonit fees + rent

    async def launch(self, params: LaunchParams) -> LaunchResult:
        """Launch a token on Moonit."""

        if C.DRY_RUN:
            return LaunchResult(
                success=True,
                token_address="DRY_RUN_MOONIT_NO_TOKEN",
                tx_hash="DRY_RUN_MOONIT_NO_TX",
                error="DRY_RUN mode — no on-chain TX sent",
            )

        api = C.MOONIT_API_BASE
        timeout = httpx.Timeout(30.0)
        migration_dex = params.extras.get("migration_dex", C.MOONIT_MIGRATION_DEX)

        async with httpx.AsyncClient(timeout=timeout) as client:

            # ── 1. Prepare mint transaction ───────────────────────────
            # Moonit SDK's prepareMintTx() — we call the REST equivalent
            print("  [Moonit] Preparing mint transaction...")

            # Convert buy amount to lamports
            buy_lamports = int(params.buy_amount * 1_000_000_000) if params.buy_amount > 0 else 0

            prepare_resp = await client.post(
                f"{api}/v1/token/prepare-mint",
                json={
                    "creator": params.wallet_address,
                    "name": params.name,
                    "symbol": params.symbol,
                    "metadataUri": params.metadata_uri,
                    "migrationDex": migration_dex,
                    "buyAmountLamports": buy_lamports,
                    "slippageBps": params.slippage_bps,
                },
            )

            if prepare_resp.status_code != 200:
                return LaunchResult(
                    success=False,
                    error=f"Moonit prepare-mint failed {prepare_resp.status_code}: {prepare_resp.text}",
                )

            prepare_data = prepare_resp.json()
            serialized_tx = prepare_data.get("transaction", "")
            token_mint = prepare_data.get("tokenMint", "")

            if not serialized_tx:
                return LaunchResult(
                    success=False,
                    error="Moonit returned empty transaction",
                )

            print(f"  [Moonit] Token mint: {token_mint}")

            # ── 2-3. Sign and broadcast via onchainos TEE wallet ─────
            print("  [Moonit] Signing and broadcasting via TEE wallet...")
            tx_hash = await self._sign_and_broadcast(serialized_tx, params.wallet_address, token_mint)

            if not tx_hash:
                return LaunchResult(
                    success=False,
                    error="Failed to submit transaction",
                )

            # ── 4. Wait for confirmation ──────────────────────────────
            print(f"  [Moonit] TX submitted: {tx_hash}")
            confirmed = await self._wait_confirmation(tx_hash, params.wallet_address)

            return LaunchResult(
                success=confirmed,
                token_address=token_mint,
                tx_hash=tx_hash,
                explorer_url=f"{_SOLANA_EXPLORER}/{tx_hash}",
                trade_page_url=f"{_MOONIT_TRADE}/{token_mint}",
                error="" if confirmed else "Transaction not confirmed within timeout",
            )

    async def _sign_and_broadcast(self, serialized_tx: str, wallet_address: str, to_address: str = "") -> str:
        """Sign unsigned TX via onchainos TEE wallet and broadcast."""
        try:
            cmd = [
                onchainos_bin(), "wallet", "contract-call",
                "--chain", "501",
                "--to", to_address or wallet_address,
                "--unsigned-tx", serialized_tx,
            ]
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()
            if proc.returncode != 0:
                print(f"  [Moonit] contract-call failed: {stderr.decode().strip()}")
                return ""
            output = json.loads(stdout.decode())
            data = output.get("data", {})
            if isinstance(data, list) and data:
                data = data[0]
            return data.get("txHash", "") or output.get("txHash", "")
        except Exception as e:
            print(f"  [Moonit] Sign/broadcast error: {e}")
            return ""

    async def _wait_confirmation(self, tx_hash: str, wallet_address: str = "", max_retries: int = 5) -> bool:
        """Poll for TX confirmation via onchainos wallet history."""
        for i in range(max_retries):
            await asyncio.sleep(5)
            try:
                cmd = [
                    onchainos_bin(), "wallet", "history",
                    "--chain", "501",
                    "--tx-hash", tx_hash,
                ]
                if wallet_address:
                    cmd.extend(["--address", wallet_address])
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
                    print(f"  [Moonit] Confirmed! ({i + 1} polls)")
                    return True
            except Exception:
                pass
        return False
