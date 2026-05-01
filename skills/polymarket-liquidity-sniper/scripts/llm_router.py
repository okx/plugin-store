"""
llm_router.py — Multi-LLM fallback router for Polymarket Liquidity Sniper
Priority chain: Groq (2 models) -> OpenRouter (8 models)
"""

import os, time, json, logging
from datetime import datetime
from pathlib import Path
from dotenv import load_dotenv

load_dotenv()

Path("logs").mkdir(exist_ok=True)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler("logs/llm_router.log"),
    ],
)
log = logging.getLogger("llm_router")

MODEL_CHAIN = [
    ("groq",       "llama-3.3-70b-versatile",                  128_000, 30, 100_000),
    ("groq",       "llama-3.1-8b-instant",                     128_000, 30, 100_000),
    ("openrouter", "meta-llama/llama-3.3-70b-instruct:free",   131_072, 20,  50_000),
    ("openrouter", "google/gemini-flash-1.5",                1_000_000, 15, 200_000),
    ("openrouter", "mistralai/mistral-7b-instruct:free",        32_768, 20,  50_000),
    ("openrouter", "nousresearch/hermes-2-pro-mistral-7b:free",  4_096, 20,  50_000),
    ("openrouter", "anthropic/claude-3-haiku",                 200_000, 50, 1_000_000),
    ("openrouter", "deepseek/deepseek-chat",                   128_000, 60, 1_000_000),
    ("openrouter", "mistralai/mistral-small",                  128_000, 60, 1_000_000),
    ("openrouter", "meta-llama/llama-3.1-70b-instruct",        131_072, 20,  200_000),
]

GROQ_API_URL       = "https://api.groq.com/openai/v1/chat/completions"
OPENROUTER_API_URL = "https://openrouter.ai/api/v1/chat/completions"

MARKET_ANALYSIS_PROMPT = """\
You are a prediction market probability analyst. Analyze this market and return a calibrated probability estimate.

Market Question: {question}
Current YES price (implied probability): {yes_price:.2f}
Current NO price (implied probability): {no_price:.2f}
24h Trading Volume: ${volume_24h:.0f} USDC
Resolution Date: {end_date}
Description: {description}

Instructions:
- Think carefully about base rates, current events, and market dynamics.
- The current market price reflects crowd wisdom — only diverge if you have strong reason.
- Return ONLY valid JSON, no preamble or markdown.

Required JSON format:
{{
  "probability_yes": 0.00,
  "confidence": 0.00,
  "reasoning": "one sentence max",
  "edge_direction": "YES" | "NO" | "NONE"
}}
"""


class LLMRouter:

    def __init__(self):
        self.groq_key       = os.getenv("GROQ_API_KEY", "")
        self.openrouter_key = os.getenv("OPENROUTER_API_KEY", "")
        self._state         = {m[1]: {"errors": 0, "blocked_until": 0, "tokens_used": 0}
                               for m in MODEL_CHAIN}
        self._usage_log     = Path("logs/llm_usage.jsonl")
        self._session_calls  = 0
        self._session_tokens = 0

    def analyze_market(self, market):
        prompt = MARKET_ANALYSIS_PROMPT.format(
            question    = market.get("question", ""),
            yes_price   = float(market.get("yes_price", 0.5)),
            no_price    = float(market.get("no_price", 0.5)),
            volume_24h  = float(market.get("volume_24h", 0)),
            end_date    = market.get("end_date", "unknown"),
            description = market.get("description", "")[:400],
        )

        for provider, model_id, _, rpm, dtl in MODEL_CHAIN:
            state = self._state[model_id]
            if time.time() < state["blocked_until"]:
                continue
            if state["tokens_used"] >= dtl * 0.9:
                continue
            try:
                result = self._call_model(provider, model_id, prompt)
                if result:
                    state["errors"] = 0
                    self._log_usage(provider, model_id, result.get("_tokens", 0))
                    result.pop("_tokens", None)
                    return result
            except RateLimitError:
                log.warning(f"Rate limit on {model_id} — blocking 90s")
                state["blocked_until"] = time.time() + 90
            except Exception as e:
                state["errors"] += 1
                log.warning(f"{model_id} error #{state['errors']}: {e}")
                if state["errors"] >= 3:
                    state["blocked_until"] = time.time() + 300

        log.error("All LLMs exhausted. Waiting 60s before retry.")
        time.sleep(60)
        return self._safe_fallback()

    def get_status(self):
        now = time.time()
        return {
            "session_calls":  self._session_calls,
            "session_tokens": self._session_tokens,
            "models": [
                {
                    "model":       m[1],
                    "provider":    m[0],
                    "available":   now >= self._state[m[1]]["blocked_until"],
                    "errors":      self._state[m[1]]["errors"],
                    "tokens_used": self._state[m[1]]["tokens_used"],
                }
                for m in MODEL_CHAIN
            ]
        }

    def _call_model(self, provider, model_id, prompt):
        import requests
        if provider == "groq":
            if not self.groq_key:
                return None
            url     = GROQ_API_URL
            headers = {"Authorization": f"Bearer {self.groq_key}", "Content-Type": "application/json"}
        else:
            if not self.openrouter_key:
                return None
            url     = OPENROUTER_API_URL
            headers = {
                "Authorization": f"Bearer {self.openrouter_key}",
                "Content-Type":  "application/json",
                "HTTP-Referer":  "https://github.com/polymarket-liquidity-sniper",
                "X-Title":       "Polymarket Liquidity Sniper",
            }

        payload = {
            "model": model_id,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 256,
            "temperature": 0.1,
        }

        resp = requests.post(url, headers=headers, json=payload, timeout=20)
        if resp.status_code == 429:
            raise RateLimitError(f"429 from {model_id}")
        if resp.status_code != 200:
            raise Exception(f"HTTP {resp.status_code}: {resp.text[:200]}")

        data    = resp.json()
        content = data["choices"][0]["message"]["content"].strip()
        tokens  = data.get("usage", {}).get("total_tokens", 100)
        parsed  = self._parse_json_response(content)
        if parsed is None:
            return None
        parsed["model_used"] = f"{provider}/{model_id}"
        parsed["_tokens"]    = tokens
        return parsed

    def _parse_json_response(self, content):
        import re
        content = re.sub(r"```json\s*|\s*```", "", content).strip()
        try:
            raw = json.loads(content)
            return {
                "probability":    float(raw.get("probability_yes", 0.5)),
                "confidence":     float(raw.get("confidence", 0.5)),
                "edge_direction": str(raw.get("edge_direction", "NONE")),
                "reasoning":      str(raw.get("reasoning", "")),
            }
        except (json.JSONDecodeError, KeyError, ValueError) as e:
            log.debug(f"JSON parse failed: {e} | content: {content[:100]}")
            return None

    def _log_usage(self, provider, model_id, tokens):
        self._state[model_id]["tokens_used"] += tokens
        self._session_calls  += 1
        self._session_tokens += tokens
        entry = {"ts": datetime.utcnow().isoformat(), "provider": provider, "model": model_id, "tokens": tokens}
        with self._usage_log.open("a") as f:
            f.write(json.dumps(entry) + "\n")

    def _safe_fallback(self):
        return {
            "probability": 0.5, "confidence": 0.0,
            "edge_direction": "NONE",
            "reasoning": "All LLMs unavailable — neutral fallback",
            "model_used": "fallback/neutral",
        }


class RateLimitError(Exception):
    pass
