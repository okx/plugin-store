"""
一键发币 v1.0 — IPFS Upload

Supports two providers:
  1. pump.fun /api/ipfs — free, no API key, uploads image + creates metadata in one call
  2. Pinata — requires PINATA_JWT, separate image + metadata uploads

pump.fun provider is preferred (zero setup). Falls back to Pinata if pump.fun fails.
"""
from __future__ import annotations

import json
import os
import sys
import mimetypes
from pathlib import Path
from typing import Optional

import httpx

# Ensure skill directory is on sys.path
_SKILL_DIR = str(Path(__file__).resolve().parent)
if _SKILL_DIR not in sys.path:
    sys.path.insert(0, _SKILL_DIR)

import config as C

_PINATA_API = "https://api.pinata.cloud"
_PUMPFUN_IPFS = "https://pump.fun/api/ipfs"


# ══════════════════════════════════════════════════════════════════════
# pump.fun IPFS (preferred — free, no API key needed)
# ══════════════════════════════════════════════════════════════════════

def upload_via_pumpfun(
    image_path: str,
    name: str,
    symbol: str,
    description: str,
    website: str = "",
    twitter: str = "",
    telegram: str = "",
) -> dict:
    """Upload image + metadata to IPFS via pump.fun's endpoint.

    This is a single call that:
      - Uploads the image to IPFS
      - Creates Metaplex-standard metadata JSON
      - Uploads metadata to IPFS
      - Returns both URIs

    Args:
        image_path: Local file path or URL to the image
        name, symbol, description: Token info
        website, twitter, telegram: Optional social links

    Returns:
        dict with keys: "image_uri", "metadata_uri", "image_cid", "metadata_cid"
    """
    # Read image
    img_data, filename, content_type = _read_image(image_path)

    # Build form data
    form_data = {
        "name": name,
        "symbol": symbol,
        "description": description,
        "showName": "true",
    }
    if twitter:
        form_data["twitter"] = twitter
    if telegram:
        form_data["telegram"] = telegram
    if website:
        form_data["website"] = website

    print(f"  [IPFS] Uploading via pump.fun (free, no key needed)...")

    resp = httpx.post(
        _PUMPFUN_IPFS,
        files={"file": (filename, img_data, content_type)},
        data=form_data,
        timeout=C.IPFS_TIMEOUT,
    )
    resp.raise_for_status()
    result = resp.json()

    metadata = result.get("metadata", {})
    metadata_uri = result.get("metadataUri", "")
    image_uri = metadata.get("image", "")

    # Extract CIDs from URIs (https://ipfs.io/ipfs/QmXxx → QmXxx)
    image_cid = image_uri.split("/ipfs/")[-1] if "/ipfs/" in image_uri else image_uri
    metadata_cid = metadata_uri.split("/ipfs/")[-1] if "/ipfs/" in metadata_uri else metadata_uri

    print(f"  [IPFS] Image:    {image_uri}")
    print(f"  [IPFS] Metadata: {metadata_uri}")

    return {
        "image_uri": image_uri,
        "metadata_uri": metadata_uri,
        "image_cid": image_cid,
        "metadata_cid": metadata_cid,
    }


# ══════════════════════════════════════════════════════════════════════
# Pinata IPFS (fallback — requires PINATA_JWT)
# ══════════════════════════════════════════════════════════════════════

_jwt: str = ""


def _get_jwt() -> str:
    global _jwt
    if not _jwt:
        _jwt = C.PINATA_JWT or os.environ.get("PINATA_JWT", "")
    if not _jwt:
        raise RuntimeError(
            "PINATA_JWT not set. Get a free key at https://app.pinata.cloud/developers/api-keys\n"
            "Then: export PINATA_JWT='your_jwt_token'"
        )
    return _jwt


def _pinata_headers() -> dict:
    return {"Authorization": f"Bearer {_get_jwt()}"}


def upload_image_pinata(image_path: str) -> str:
    """Upload image to Pinata IPFS. Returns CID."""
    img_data, filename, content_type = _read_image(image_path)

    resp = httpx.post(
        f"{_PINATA_API}/pinning/pinFileToIPFS",
        headers=_pinata_headers(),
        files={"file": (filename, img_data, content_type)},
        timeout=C.IPFS_TIMEOUT,
    )
    resp.raise_for_status()
    cid = resp.json()["IpfsHash"]
    print(f"  [IPFS/Pinata] Image uploaded: {cid}")
    return cid


def upload_metadata_pinata(
    name: str, symbol: str, description: str, image_cid: str,
    website: str = "", twitter: str = "", telegram: str = "",
) -> str:
    """Upload metadata JSON to Pinata IPFS. Returns CID."""
    metadata = {
        "name": name,
        "symbol": symbol,
        "description": description,
        "image": f"ipfs://{image_cid}",
    }
    if website:
        metadata["website"] = website
    if twitter:
        metadata["twitter"] = twitter
    if telegram:
        metadata["telegram"] = telegram

    payload = json.dumps(metadata, ensure_ascii=False).encode("utf-8")

    resp = httpx.post(
        f"{_PINATA_API}/pinning/pinFileToIPFS",
        headers=_pinata_headers(),
        files={"file": (f"{symbol}_metadata.json", payload, "application/json")},
        timeout=C.IPFS_TIMEOUT,
    )
    resp.raise_for_status()
    cid = resp.json()["IpfsHash"]
    print(f"  [IPFS/Pinata] Metadata uploaded: {cid}")
    return cid


# ══════════════════════════════════════════════════════════════════════
# Smart upload — tries pump.fun first, falls back to Pinata
# ══════════════════════════════════════════════════════════════════════

def upload_all(
    image_path: str,
    name: str,
    symbol: str,
    description: str,
    website: str = "",
    twitter: str = "",
    telegram: str = "",
) -> dict:
    """Upload image + metadata to IPFS.

    Tries pump.fun endpoint first (free, no API key).
    Falls back to Pinata if pump.fun fails.

    Returns:
        dict with: "image_cid", "metadata_cid", "metadata_uri", "image_uri"
    """
    # Try pump.fun first
    try:
        return upload_via_pumpfun(
            image_path, name, symbol, description,
            website, twitter, telegram,
        )
    except Exception as e:
        print(f"  [IPFS] pump.fun upload failed: {e}")
        print(f"  [IPFS] Falling back to Pinata...")

    # Fallback: Pinata (requires PINATA_JWT)
    image_cid = upload_image_pinata(image_path)
    metadata_cid = upload_metadata_pinata(
        name, symbol, description, image_cid,
        website, twitter, telegram,
    )
    return {
        "image_uri": f"{C.PINATA_GATEWAY}/{image_cid}",
        "metadata_uri": f"{C.PINATA_GATEWAY}/{metadata_cid}",
        "image_cid": image_cid,
        "metadata_cid": metadata_cid,
    }


# ══════════════════════════════════════════════════════════════════════
# Helpers
# ══════════════════════════════════════════════════════════════════════

def _read_image(image_path: str) -> tuple:
    """Read image from path, URL, or base64/data-URI.

    Returns (data: bytes, filename: str, content_type: str).

    Supported inputs:
      - "/path/to/image.png"                  → read from disk
      - "https://example.com/dog.png"         → download
      - "data:image/png;base64,iVBOR…"        → decode inline
      - raw base64 string (len > 500)         → decode inline
    """
    # ── URL ────────────────────────────────────────────────────────────
    if image_path.startswith("http://") or image_path.startswith("https://"):
        resp = httpx.get(image_path, timeout=C.IPFS_TIMEOUT, follow_redirects=True)
        resp.raise_for_status()
        data = resp.content
        content_type = resp.headers.get("content-type", "image/png")
        ext = mimetypes.guess_extension(content_type.split(";")[0].strip()) or ".png"
        return data, f"token_image{ext}", content_type

    # ── Data URI: data:image/png;base64,xxxxx ──────────────────────────
    if image_path.startswith("data:"):
        import base64 as b64
        try:
            header, b64data = image_path.split(",", 1)
            content_type = header.split(":")[1].split(";")[0]
        except (ValueError, IndexError):
            content_type = "image/png"
            b64data = image_path.split(",")[-1]
        data = b64.b64decode(b64data)
        ext = mimetypes.guess_extension(content_type) or ".png"
        if len(data) > C.IMAGE_MAX_SIZE:
            raise ValueError(f"Image too large: {len(data) / 1024 / 1024:.1f} MB (max {C.IMAGE_MAX_SIZE / 1024 / 1024:.0f} MB)")
        return data, f"token_image{ext}", content_type

    # ── Raw base64 (long string, not a file path) ─────────────────────
    if len(image_path) > 500 and not os.path.exists(image_path):
        import base64 as b64
        data = b64.b64decode(image_path)
        if len(data) > C.IMAGE_MAX_SIZE:
            raise ValueError(f"Image too large: {len(data) / 1024 / 1024:.1f} MB (max {C.IMAGE_MAX_SIZE / 1024 / 1024:.0f} MB)")
        return data, "token_image.png", "image/png"

    # ── File path ──────────────────────────────────────────────────────
    p = Path(image_path)
    if not p.exists():
        raise FileNotFoundError(f"Image not found: {image_path}")

    size = p.stat().st_size
    if size > C.IMAGE_MAX_SIZE:
        raise ValueError(f"Image too large: {size / 1024 / 1024:.1f} MB (max {C.IMAGE_MAX_SIZE / 1024 / 1024:.0f} MB)")

    ext = p.suffix.lower().lstrip(".")
    if ext not in C.IMAGE_FORMATS:
        raise ValueError(f"Unsupported image format: .{ext} (supported: {C.IMAGE_FORMATS})")

    data = p.read_bytes()
    content_type = mimetypes.guess_type(str(p))[0] or "image/png"
    return data, p.name, content_type


def gateway_url(cid: str) -> str:
    """Convert IPFS CID to gateway URL."""
    return f"https://ipfs.io/ipfs/{cid}"


def ipfs_uri(cid: str) -> str:
    """Convert IPFS CID to ipfs:// URI."""
    return f"ipfs://{cid}"
