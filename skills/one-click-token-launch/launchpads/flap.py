"""
一键发币 v1.0 — Flap.sh adapter (BSC, direct contract interaction).

Flow:
  1. Upload image + metadata to IPFS (Pinata)
  2. Call Flap Portal contract newTokenV6() via onchainos wallet contract-call
  3. Supports tax tokens (buy/sell tax), vanity addresses, PCS V2/V3 migration
  4. Wait for confirmation

Portal: 0xe2cE6ab80874Fa9Fa2aAE65D277Dd6B8e65C9De0 (BNB Mainnet)
Docs:   https://docs.flap.sh/flap/developers/launch-a-token
"""
from __future__ import annotations

import asyncio
import json

import config as C
from .base import LaunchpadAdapter, LaunchParams, LaunchResult, onchainos_bin

_BSC_EXPLORER = "https://bscscan.com/tx"
_FLAP_TRADE = "https://flap.sh/token"
_ZERO_ADDR = "0x0000000000000000000000000000000000000000"
_ZERO_BYTES32 = "0x" + "00" * 32


class FlapAdapter(LaunchpadAdapter):

    @property
    def name(self) -> str:
        return "flap"

    @property
    def display_name(self) -> str:
        return "Flap.sh"

    @property
    def chain(self) -> str:
        return "bsc"

    def _fee_estimate(self, params: LaunchParams) -> float:
        return 0.015  # BNB gas

    async def launch(self, params: LaunchParams) -> LaunchResult:
        """Launch a token on Flap.sh (BSC) via newTokenV6."""

        if C.DRY_RUN:
            return LaunchResult(
                success=True,
                token_address="DRY_RUN_FLAP_NO_TOKEN",
                tx_hash="DRY_RUN_FLAP_NO_TX",
                error="DRY_RUN mode — no on-chain TX sent",
            )

        portal = C.FLAP_PORTAL
        extras = params.extras

        # ── Build newTokenV6 parameters ───────────────────────────────
        buy_tax = extras.get("buy_tax", C.FLAP_BUY_TAX)
        sell_tax = extras.get("sell_tax", C.FLAP_SELL_TAX)
        tax_duration = extras.get("tax_duration", C.FLAP_TAX_DURATION)
        anti_farmer = extras.get("anti_farmer", C.FLAP_ANTI_FARMER)
        migrator_type = extras.get("migrator_type", C.FLAP_MIGRATOR_TYPE)
        dex_id = extras.get("dex_id", C.FLAP_DEX_ID)
        lp_fee_profile = extras.get("lp_fee_profile", C.FLAP_LP_FEE_PROFILE)
        token_version = extras.get("token_version", C.FLAP_TOKEN_VERSION)
        salt = extras.get("salt", _ZERO_BYTES32)
        beneficiary = extras.get("beneficiary", params.wallet_address)

        # Tax allocation split
        mkt_bps = extras.get("mkt_bps", C.FLAP_MKT_BPS)
        deflation_bps = extras.get("deflation_bps", C.FLAP_DEFLATION_BPS)
        dividend_bps = extras.get("dividend_bps", C.FLAP_DIVIDEND_BPS)
        lp_bps = extras.get("lp_bps", C.FLAP_LP_BPS)

        buy_wei = int(params.buy_amount * 10**18) if params.buy_amount > 0 else 0

        # newTokenV6 struct parameter (ABI-encoded as tuple)
        # We pass all fields as a JSON array for onchainos contract-call
        v6_params = {
            "name": params.name,
            "symbol": params.symbol,
            "meta": params.metadata_cid,  # IPFS CID (not full URI)
            "dexThresh": 0,               # Default DEX listing threshold
            "salt": salt,
            "migratorType": migrator_type,
            "quoteToken": _ZERO_ADDR,     # address(0) = native BNB
            "quoteAmt": buy_wei,
            "beneficiary": beneficiary,
            "permitData": "0x",
            "extensionID": _ZERO_BYTES32,
            "extensionData": "0x",
            "dexId": dex_id,
            "lpFeeProfile": lp_fee_profile,
            "buyTaxRate": buy_tax,
            "sellTaxRate": sell_tax,
            "taxDuration": tax_duration,
            "antiFarmerDuration": anti_farmer,
            "mktBps": mkt_bps,
            "deflationBps": deflation_bps,
            "dividendBps": dividend_bps,
            "lpBps": lp_bps,
            "minimumShareBalance": 0,
            "dividendToken": _ZERO_ADDR,
            "commissionReceiver": _ZERO_ADDR,
            "tokenVersion": token_version,
        }

        print(f"  [Flap] Calling newTokenV6 on portal {portal[:10]}...")
        if buy_tax > 0 or sell_tax > 0:
            print(f"  [Flap] Tax config: buy={buy_tax}bps sell={sell_tax}bps duration={tax_duration}s")
        if salt != _ZERO_BYTES32:
            print(f"  [Flap] Vanity salt: {salt[:10]}...")

        # ABI-encode newTokenV6((tuple)) call data
        input_data = self._encode_new_token_v6(
            name=params.name,
            symbol=params.symbol,
            meta=params.metadata_cid,
            dex_thresh=0,
            salt=salt,
            migrator_type=migrator_type,
            quote_token=_ZERO_ADDR,
            quote_amt=buy_wei,
            beneficiary=beneficiary,
            permit_data=b"",
            extension_id=_ZERO_BYTES32,
            extension_data=b"",
            dex_id=dex_id,
            lp_fee_profile=lp_fee_profile,
            buy_tax=buy_tax,
            sell_tax=sell_tax,
            tax_duration=tax_duration,
            anti_farmer=anti_farmer,
            mkt_bps=mkt_bps,
            deflation_bps=deflation_bps,
            dividend_bps=dividend_bps,
            lp_bps=lp_bps,
            min_share_balance=0,
            dividend_token=_ZERO_ADDR,
            commission_receiver=_ZERO_ADDR,
            token_version=token_version,
        )

        cmd = [
            onchainos_bin(), "wallet", "contract-call",
            "--chain", "56",
            "--to", portal,
            "--input-data", input_data,
            "--biz-type", "dex",
            "--strategy", "one-click-token-launch",
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
                    error=f"Flap contract-call failed: {err}",
                )

            output = json.loads(stdout.decode())
            tx_hash = output.get("data", {}).get("txHash", "") or output.get("txHash", "")

        except Exception as e:
            return LaunchResult(success=False, error=f"Flap launch error: {e}")

        if not tx_hash:
            return LaunchResult(success=False, error="No tx hash returned from contract-call")

        # ── Wait for BSC confirmation ─────────────────────────────────
        print(f"  [Flap] TX submitted: {tx_hash}")
        confirmed, token_address = await self._wait_and_parse(tx_hash, params.wallet_address)

        return LaunchResult(
            success=confirmed,
            token_address=token_address,
            tx_hash=tx_hash,
            explorer_url=f"{_BSC_EXPLORER}/{tx_hash}",
            trade_page_url=f"{_FLAP_TRADE}/{token_address}" if token_address else "",
            error="" if confirmed else "Transaction not confirmed within timeout",
        )

    @staticmethod
    def _encode_new_token_v6(**kw) -> str:
        """ABI-encode newTokenV6((tuple)) call data.

        Selector: keccak256("newTokenV6((string,string,string,uint8,bytes32,
        uint8,address,uint256,address,bytes,bytes32,bytes,uint8,uint8,
        uint16,uint16,uint256,uint256,uint16,uint16,uint16,uint16,
        uint256,address,address,uint8))")[:4] = 0x363eb8e6
        """
        selector = "363eb8e6"

        def _pad32(val: int, signed: bool = False) -> str:
            return val.to_bytes(32, "big", signed=signed).hex()

        def _pad_addr(addr: str) -> str:
            a = addr.lower().replace("0x", "")
            return a.rjust(64, "0")

        def _pad_bytes32(b32: str) -> str:
            h = b32.replace("0x", "")
            return h.ljust(64, "0")

        def _encode_string(s: str) -> str:
            data = s.encode("utf-8")
            length = len(data)
            padded_len = ((length + 31) // 32) * 32
            return _pad32(length) + data.hex().ljust(padded_len * 2, "0")

        def _encode_bytes(b: bytes) -> str:
            length = len(b)
            padded_len = ((length + 31) // 32) * 32
            return _pad32(length) + b.hex().ljust(padded_len * 2, "0") if length else _pad32(0)

        # The struct is encoded as a tuple — outer offset pointer then tuple data
        # For a single tuple param, offset = 32 (0x20)
        outer_offset = _pad32(32)

        # Within the tuple: fixed fields inline, dynamic fields (string, bytes) as offsets
        # Field layout (26 fields):
        #  0: name (string, dynamic)
        #  1: symbol (string, dynamic)
        #  2: meta (string, dynamic)
        #  3: dexThresh (uint8)
        #  4: salt (bytes32)
        #  5: migratorType (uint8)
        #  6: quoteToken (address)
        #  7: quoteAmt (uint256)
        #  8: beneficiary (address)
        #  9: permitData (bytes, dynamic)
        # 10: extensionID (bytes32)
        # 11: extensionData (bytes, dynamic)
        # 12-25: uint8/uint16/uint256 (all static)

        # 26 slots of 32 bytes each for heads
        head_slots = 26
        head_size = head_slots * 32  # bytes

        # Encode dynamic data and compute offsets
        dyn_parts = []
        dyn_offset = head_size

        def _add_dynamic(encoded: str) -> str:
            nonlocal dyn_offset
            offset_hex = _pad32(dyn_offset)
            byte_len = len(encoded) // 2
            dyn_offset += byte_len
            dyn_parts.append(encoded)
            return offset_hex

        heads = []
        # 0: name
        heads.append(_add_dynamic(_encode_string(kw["name"])))
        # 1: symbol
        heads.append(_add_dynamic(_encode_string(kw["symbol"])))
        # 2: meta
        heads.append(_add_dynamic(_encode_string(kw["meta"])))
        # 3: dexThresh
        heads.append(_pad32(kw["dex_thresh"]))
        # 4: salt
        heads.append(_pad_bytes32(kw["salt"]))
        # 5: migratorType
        heads.append(_pad32(kw["migrator_type"]))
        # 6: quoteToken
        heads.append(_pad_addr(kw["quote_token"]))
        # 7: quoteAmt
        heads.append(_pad32(kw["quote_amt"]))
        # 8: beneficiary
        heads.append(_pad_addr(kw["beneficiary"]))
        # 9: permitData
        heads.append(_add_dynamic(_encode_bytes(kw["permit_data"])))
        # 10: extensionID
        heads.append(_pad_bytes32(kw["extension_id"]))
        # 11: extensionData
        heads.append(_add_dynamic(_encode_bytes(kw["extension_data"])))
        # 12: dexId
        heads.append(_pad32(kw["dex_id"]))
        # 13: lpFeeProfile
        heads.append(_pad32(kw["lp_fee_profile"]))
        # 14: buyTaxRate
        heads.append(_pad32(kw["buy_tax"]))
        # 15: sellTaxRate
        heads.append(_pad32(kw["sell_tax"]))
        # 16: taxDuration
        heads.append(_pad32(kw["tax_duration"]))
        # 17: antiFarmerDuration
        heads.append(_pad32(kw["anti_farmer"]))
        # 18: mktBps
        heads.append(_pad32(kw["mkt_bps"]))
        # 19: deflationBps
        heads.append(_pad32(kw["deflation_bps"]))
        # 20: dividendBps
        heads.append(_pad32(kw["dividend_bps"]))
        # 21: lpBps
        heads.append(_pad32(kw["lp_bps"]))
        # 22: minimumShareBalance
        heads.append(_pad32(kw["min_share_balance"]))
        # 23: dividendToken
        heads.append(_pad_addr(kw["dividend_token"]))
        # 24: commissionReceiver
        heads.append(_pad_addr(kw["commission_receiver"]))
        # 25: tokenVersion
        heads.append(_pad32(kw["token_version"]))

        tuple_data = "".join(heads) + "".join(dyn_parts)
        return "0x" + selector + outer_offset + tuple_data

    async def _wait_and_parse(self, tx_hash: str, wallet_address: str = "", max_retries: int = 6) -> tuple:
        """Wait for BSC TX confirmation and extract token address.

        Flap emits TokenCreated(ts, creator, nonce, token, name, symbol, meta)
        The `token` parameter is the new token address.
        """
        for i in range(max_retries):
            await asyncio.sleep(3)
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
                    token_addr = ""
                    logs = data.get("logs", [])
                    for log in logs:
                        topics = log.get("topics", [])
                        if len(topics) >= 1 and log.get("address", "").lower() == C.FLAP_PORTAL.lower():
                            log_data = log.get("data", "")
                            if len(log_data) >= 130:
                                addr_hex = log_data[90:130]
                                token_addr = "0x" + addr_hex[-40:]

                    if not token_addr:
                        token_addr = data.get("contractAddress", "")

                    print(f"  [Flap] Confirmed! Token: {token_addr or 'parsing...'}")
                    return True, token_addr

            except Exception:
                pass

        return False, ""
