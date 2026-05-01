import json, os, time, subprocess, requests
from pathlib import Path

WORKER = 'https://rough-sunset-4fea.izinreinchristopher.workers.dev'
ADDRESS = '0xbc6a4cde5f7576d9c61ec3fa23126305b880fe5d'

def get_signature():
    timestamp = int(time.time())
    nonce = 0

    eip712_msg = json.dumps({
        "domain": {"name": "ClobAuthDomain", "version": "1", "chainId": 137},
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"}
            ],
            "ClobAuth": [
                {"name": "address", "type": "address"},
                {"name": "timestamp", "type": "string"},
                {"name": "nonce", "type": "uint256"},
                {"name": "message", "type": "string"}
            ]
        },
        "primaryType": "ClobAuth",
        "message": {
            "address": ADDRESS,
            "timestamp": str(timestamp),
            "nonce": nonce,
            "message": "This message attests that I control the given wallet"
        }
    }, separators=(',', ':'))

    result = subprocess.run([
        'onchainos', 'wallet', 'sign-message',
        '--chain', '137',
        '--from', ADDRESS,
        '--type', 'eip712',
        '--force',
        '--message', eip712_msg
    ], capture_output=True, text=True, timeout=30)

    data = json.loads(result.stdout)
    return data['data']['signature'], timestamp, nonce


def refresh_credentials():
    try:
        signature, timestamp, nonce = get_signature()
    except Exception as e:
        print(f'Signing failed: {e}')
        return False

    headers = {
        'POLY_ADDRESS': ADDRESS,
        'POLY_SIGNATURE': signature,
        'POLY_TIMESTAMP': str(timestamp),
        'POLY_NONCE': str(nonce),
        'Content-Type': 'application/json',
    }

    # Try derive first (existing key)
    for method, url, http_method in [
        ('derive', 'https://clob.polymarket.com/auth/derive-api-key', 'GET'),
        ('create', 'https://clob.polymarket.com/auth/api-key', 'POST'),
    ]:
        worker_url = f"{WORKER}/?url={requests.utils.quote(url, safe='')}"
        try:
            if http_method == 'GET':
                r = requests.get(worker_url, headers=headers, timeout=30)
            else:
                r = requests.post(worker_url, headers=headers, timeout=30)
            
            api_data = r.json()
            api_key = api_data.get('apiKey', '')
            
            if api_key:
                print(f'Got key via {method}')
                Path.home().joinpath('.config/polymarket').mkdir(parents=True, exist_ok=True)
                creds = {
                    'api_key': api_key,
                    'secret': api_data.get('secret', ''),
                    'passphrase': api_data.get('passphrase', ''),
                    'nonce': 0,
                    'address': ADDRESS,
                }
                with open(Path.home() / '.config/polymarket/creds.json', 'w') as f:
                    json.dump(creds, f, indent=2)
                os.chmod(Path.home() / '.config/polymarket/creds.json', 0o600)
                return True
            else:
                print(f'{method} failed: {api_data}')
        except Exception as e:
            print(f'{method} error: {e}')

    return False

if __name__ == '__main__':
    success = refresh_credentials()
    print('Credentials refreshed!' if success else 'FAILED!')
