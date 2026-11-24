"""
一键发币 v1.0 — Bags.fm adapter (Official REST API + Meteora DBC).

Flow:
  1. Upload token info + metadata via Bags API
  2. Create fee share config (creator % + optional co-earners)
  3. Create launch transaction with optional initial buy
  4. Sign via onchainos wallet
  5. Submit and wait for confirmation

Docs: https://docs.bags.fm/how-to-guides/launch-token
"""
from __future__ import annotations

import asyncio
import base64
import json
import os

import httpx

import config as C
from .base import LaunchpadAdapter, LaunchParams, LaunchResult, onchainos_bin

_SOLANA_EXPLORER = "https://solscan.io/tx"
_BAGS_TRADE = "https://bags.fm/token"


class BagsAdapter(LaunchpadAdapter):

    @property
    def name(self) -> str:
        return "bags"

    @property
    def display_name(self) -> str:
        return "Bags.fm"

    @property
    def chain(self) -> str:
        return "solana"

    def _fee_estimate(self, params: LaunchParams) -> float:
        return 0.015  # Bags fees + rent

    async def launch(self, params: LaunchParams) -> LaunchResult:
        """Launch a token on Bags.fm."""

        if C.DRY_RUN:
            return LaunchResult(
                success=True,
                token_address="DRY_RUN_BAGS_NO_TOKEN",
                tx_hash="DRY_RUN_BAGS_NO_TX",
                error="DRY_RUN mode — no on-chain TX sent",
            )

        api = C.BAGS_API_BASE
        timeout = httpx.Timeout(30.0)

        async with httpx.AsyncClient(timeout=timeout) as client:

            # ── 1. Create token info + metadata ───────────────────────
            print("  [Bags] Creating token info & metadata...")
            token_info_resp = await client.post(
                f"{api}/token-launch/create-token-info",
                json={
                    "name": params.name,
                    "symbol": params.symbol,
                    "description": params.description,
                    "imageUrl": f"https://ipfs.io/ipfs/{params.image_cid}" if params.image_cid else params.metadata_uri,
                    "twitter": params.twitter or None,
                    "website": params.website or None,
                    "telegram": params.telegram or None,
                },
            )
            if token_info_resp.status_code != 200:
                return LaunchResult(
                    success=False,
                    error=f"Bags create-token-info failed {token_info_resp.status_code}: {token_info_resp.text}",
                )

            token_info = token_info_resp.json()
            metadata_url = token_info.get("metadataUrl", "")
            token_mint = token_info.get("tokenMint", "")
            print(f"  [Bags] Token mint: {token_mint}")
            print(f"  [Bags] Metadata URL: {metadata_url}")

            # ── 2. Create fee share config ────────────────────────────
            print("  [Bags] Creating fee share config...")
            fee_claimers = params.extras.get("fee_claimers", C.BAGS_FEE_CLAIMERS)

            # If no fee claimers specified, 100% to creator
            if not fee_claimers:
                fee_claimers = [{"user": params.wallet_address, "userBps": 10000}]

            # Validate total bps = 10000
            total_bps = sum(fc.get("userBps", 0) for fc in fee_claimers)
            if total_bps != 10000:
                return LaunchResult(
                    success=False,
                    error=f"Fee share BPS must total 10000, got {total_bps}",
                )

            fee_config_resp = await client.post(
                f"{api}/fee-share/config",
                json={
                    "payer": params.wallet_address,
                    "baseMint": token_mint,
                    "feeClaimers": fee_claimers,
                },
            )
            if fee_config_resp.status_code != 200:
                return LaunchResult(
                    success=False,
                    error=f"Bags fee-share config failed {fee_config_resp.status_code}: {fee_config_resp.text}",
                )

            config_key = fee_config_resp.json().get("configKey", "")
            print(f"  [Bags] Fee share config: {config_key}")

            # ── 3. Create launch transaction ──────────────────────────
            print("  [Bags] Creating launch transaction...")

            # Convert buy amount to lamports (1 SOL = 1_000_000_000 lamports)
            initial_buy_lamports = int(params.buy_amount * 1_000_000_000)

            # Get launch wallet from Bags
            wallet_resp = await client.get(
                f"{api}/token-launch/fee-share/wallet/v2",
                params={"walletAddress": params.wallet_address},
            )
            launch_wallet = params.wallet_address
            if wallet_resp.status_code == 200:
                lw = wallet_resp.json().get("launchWallet")
                if lw:
                    launch_wallet = lw

            launch_tx_resp = await client.post(
                f"{api}/token-launch/create-launch-transaction",
                json={
                    "metadataUrl": metadata_url,
                    "tokenMint": token_mint,
                    "launchWallet": launch_wallet,
                    "initialBuyLamports": initial_buy_lamports,
                    "configKey": config_key,
                },
            )
            if launch_tx_resp.status_code != 200:
                return LaunchResult(
                    success=False,
                    error=f"Bags create-launch-tx failed {launch_tx_resp.status_code}: {launch_tx_resp.text}",
                )

            tx_data = launch_tx_resp.json()
            serialized_tx = tx_data.get("transaction", "")

            if not serialized_tx:
                return LaunchResult(
                    success=False,
                    error="Bags returned empty transaction",
                )

            # ── 4. Sign and submit via onchainos ──────────────────────
            print("  [Bags] Signing and submitting...")
            tx_hash = await self._sign_and_submit(serialized_tx, params.wallet_address, token_mint)

            if not tx_hash:
                return LaunchResult(
                    success=False,
                    error="Failed to sign/submit via onchainos wallet",
                )

            # ── 5. Wait for confirmation ──────────────────────────────
            print(f"  [Bags] TX submitted: {tx_hash}")
            confirmed = await self._wait_confirmation(tx_hash, params.wallet_address)

            return LaunchResult(
                success=confirmed,
                token_address=token_mint,
                tx_hash=tx_hash,
                explorer_url=f"{_SOLANA_EXPLORER}/{tx_hash}",
                trade_page_url=f"{_BAGS_TRADE}/{token_mint}",
                error="" if confirmed else "Transaction not confirmed within timeout",
            )

    async def _sign_and_submit(self, serialized_tx: str, wallet_address: str, to_address: str = "") -> str:
        """Sign and submit unsigned transaction via onchainos TEE wallet."""
        try:
            cmd = [
                onchainos_bin(), "wallet", "contract-call",
                "--chain", "501",
                "--to", to_address or wallet_address,
                "--unsigned-tx", serialized_tx,
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
                print(f"  [Bags] contract-call failed: {stderr.decode().strip()}")
                return ""
            output = json.loads(stdout.decode())
            data = output.get("data", {})
            if isinstance(data, list) and data:
                data = data[0]
            return data.get("txHash", "") or output.get("txHash", "")
        except Exception as e:
            print(f"  [Bags] Sign/submit error: {e}")
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
                    print(f"  [Bags] Confirmed! ({i + 1} polls)")
                    return True
            except Exception:
                pass
        return False
