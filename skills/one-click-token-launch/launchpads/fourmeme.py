"""
一键发币 v1.0 — Four.Meme adapter (BSC, direct contract interaction).

Flow:
  1. Upload image + metadata to IPFS (Pinata)
  2. Call Four.Meme factory contract via onchainos wallet contract-call
  3. Include msg.value for initial buy (bundled)
  4. Wait for confirmation

Four.Meme is the largest BSC launchpad. It doesn't expose a public REST API
for token creation, so we interact with the factory contract directly.
"""
from __future__ import annotations

import asyncio
import json

import config as C
from .base import LaunchpadAdapter, LaunchParams, LaunchResult, onchainos_bin

_BSC_EXPLORER = "https://bscscan.com/tx"
_FOURMEME_TRADE = "https://four.meme/token"


class FourMemeAdapter(LaunchpadAdapter):

    @property
    def name(self) -> str:
        return "fourmeme"

    @property
    def display_name(self) -> str:
        return "Four.Meme"

    @property
    def chain(self) -> str:
        return "bsc"

    def _fee_estimate(self, params: LaunchParams) -> float:
        return 0.015  # BNB gas

    async def launch(self, params: LaunchParams) -> LaunchResult:
        """Launch a token on Four.Meme (BSC)."""

        if C.DRY_RUN:
            return LaunchResult(
                success=True,
                token_address="DRY_RUN_FOURMEME_NO_TOKEN",
                tx_hash="DRY_RUN_FOURMEME_NO_TX",
                error="DRY_RUN mode — no on-chain TX sent",
            )

        factory = C.FOURMEME_FACTORY
        if not factory:
            return LaunchResult(
                success=False,
                error="FOURMEME_FACTORY address not configured in config.py. "
                      "Set the Four.Meme factory contract address.",
            )

        category = params.extras.get("category", C.FOURMEME_CATEGORY)
        gas_price = params.extras.get("gas_price", C.FOURMEME_GAS_PRICE)

        buy_wei = int(params.buy_amount * 10**18) if params.buy_amount > 0 else 0

        # ABI-encode createToken(string,string,string,string) call data
        input_data = self._encode_create_token(
            params.name, params.symbol, params.metadata_uri, category,
        )

        print("  [Four.Meme] Calling factory contract...")

        cmd = [
            onchainos_bin(), "wallet", "contract-call",
            "--chain", "56",
            "--to", factory,
            "--input-data", input_data,
        ]

        if buy_wei > 0:
            cmd.extend(["--amt", str(buy_wei)])

        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await proc.communicate()

            if proc.returncode != 0:
                err = stderr.decode().strip() if stderr else "unknown error"
                return LaunchResult(
                    success=False,
                    error=f"Four.Meme contract-call failed: {err}",
                )

            output = json.loads(stdout.decode())
            tx_hash = output.get("data", {}).get("txHash", "") or output.get("txHash", "")

        except Exception as e:
            return LaunchResult(success=False, error=f"Four.Meme launch error: {e}")

        if not tx_hash:
            return LaunchResult(success=False, error="No tx hash returned from contract-call")

        # ── Wait for confirmation (~3-5s on BSC) ──────────────────────
        print(f"  [Four.Meme] TX submitted: {tx_hash}")
        confirmed, token_address = await self._wait_and_parse(tx_hash, params.wallet_address)

        return LaunchResult(
            success=confirmed,
            token_address=token_address,
            tx_hash=tx_hash,
            explorer_url=f"{_BSC_EXPLORER}/{tx_hash}",
            trade_page_url=f"{_FOURMEME_TRADE}/{token_address}" if token_address else "",
            error="" if confirmed else "Transaction not confirmed within timeout",
        )

    @staticmethod
    def _encode_create_token(name: str, symbol: str, metadata_uri: str, category: str) -> str:
        """ABI-encode createToken(string,string,string,string) call data."""
        # Function selector: keccak256("createToken(string,string,string,string)")[:4]
        selector = "a0769659"

        def _encode_string(s: str) -> str:
            data = s.encode("utf-8")
            length = len(data)
            # 32-byte length prefix + data padded to 32-byte boundary
            padded_len = ((length + 31) // 32) * 32
            return (
                length.to_bytes(32, "big").hex()
                + data.hex().ljust(padded_len * 2, "0")
            )

        # 4 dynamic params → 4 offset pointers, then data
        strings = [name, symbol, metadata_uri, category]
        encoded_strings = [_encode_string(s) for s in strings]

        # Calculate offsets (each pointer is 32 bytes = 64 hex chars)
        base_offset = len(strings) * 32  # offset area size in bytes
        offsets = []
        running = base_offset
        for es in encoded_strings:
            offsets.append(running.to_bytes(32, "big").hex())
            running += len(es) // 2  # bytes = hex chars / 2

        return "0x" + selector + "".join(offsets) + "".join(encoded_strings)

    async def _wait_and_parse(self, tx_hash: str, wallet_address: str = "", max_retries: int = 6) -> tuple:
        """Wait for BSC TX confirmation and parse token address from logs.

        Returns (confirmed: bool, token_address: str).
        """
        for i in range(max_retries):
            await asyncio.sleep(3)  # BSC is ~3s blocks
            try:
                cmd = [
                    onchainos_bin(), "wallet", "history",
                    "--chain", "56",
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

                if status in ("confirmed", "finalized", "success", "1"):
                    # Try to extract token address from logs
                    token_addr = ""
                    logs = data.get("logs", [])
                    for log in logs:
                        # TokenCreated event contains the new token address
                        topics = log.get("topics", [])
                        if len(topics) >= 2:
                            addr = log.get("address", "")
                            if addr and addr != C.FOURMEME_FACTORY:
                                token_addr = addr
                                break

                    if not token_addr:
                        token_addr = data.get("contractAddress", "")

                    print(f"  [Four.Meme] Confirmed! Token: {token_addr or 'parsing...'}")
                    return True, token_addr

            except Exception:
                pass

        return False, ""
