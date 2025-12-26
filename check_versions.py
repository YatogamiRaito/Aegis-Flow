
import urllib.request
import json
import time

crates = [
    "tokio", "hyper", "hyper-util", "tower", "bytes", "s2n-quic", "rustls", "tokio-rustls",
    "pqcrypto-mlkem", "pqcrypto-mldsa", "pqcrypto-traits", "x25519-dalek", "tracing",
    "tracing-subscriber", "metrics", "metrics-exporter-prometheus", "serde", "serde_json",
    "thiserror", "anyhow", "criterion", "rand", "aes-gcm", "chacha20poly1305", "hkdf", "sha2",
    "serde_yaml", "toml", "x509-parser", "rcgen", "ring", "webpki-roots", "time",
    "http-body-util", "h3", "hex", "async-trait", "chrono", "num_cpus", "tempfile",
    "reqwest", "moka", "wiremock", "arrow", "arrow-schema", "arrow-array", "arrow-buffer",
    "polars", "noodles", "wasmtime", "wat", "pem"
]

results = {}

print(f"Checking {len(crates)} crates...")

opener = urllib.request.build_opener()
opener.addheaders = [('User-Agent', 'Antigravity-Audit-Bot (bot@antigravity.com)')]
urllib.request.install_opener(opener)

for crate in crates:
    try:
        url = f"https://crates.io/api/v1/crates/{crate}"
        with urllib.request.urlopen(url) as response:
            if response.status == 200:
                data = json.loads(response.read().decode())
                max_ver = data['crate']['max_version']
                results[crate] = max_ver
            else:
                results[crate] = f"Error: {response.status}"
    except Exception as e:
        results[crate] = f"Exception: {e}"
    time.sleep(0.1) # Be nice to API

print(json.dumps(results, indent=2))
