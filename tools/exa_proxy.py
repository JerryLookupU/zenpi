#!/usr/bin/env python3
"""Async Exa rate-limit proxy: shared 25 QPS budget, thousands of waiters.

The previous threaded proxy created one OS thread per in-flight worker; with
2048 workers the GIL made the machine load explode. This version keeps every
waiter as a cheap asyncio task and caps upstream calls with a thread pool.
"""
import asyncio
import json
import os
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor

UPSTREAM = os.environ.get("EXA_UPSTREAM", "https://api.exa.ai/search")
PORT = int(os.environ.get("EXA_PROXY_PORT", "8081"))
QPS = float(os.environ.get("EXA_QPS", "25"))
POOL = int(os.environ.get("EXA_POOL", "64"))
LOG = os.environ.get("EXA_PROXY_LOG", "/tmp/exa_proxy.log")


def load_key():
    key = os.environ.get("EXA_API_KEY")
    if key:
        return key.strip()
    path = os.path.expanduser("~/.zenpi/exa.env")
    for line in open(path, encoding="utf-8"):
        line = line.strip().removeprefix("export ")
        if line.startswith("EXA_API_KEY="):
            return line.split("=", 1)[1].strip().strip('"').strip("'")
    raise SystemExit("EXA_API_KEY not configured")


KEY = load_key()
STATS = {"ok": 0, "err": 0}


class AsyncBucket:
    def __init__(self, rate):
        self.rate = rate
        self.tokens = rate
        self.updated = time.monotonic()
        self.lock = asyncio.Lock()

    async def take(self):
        while True:
            async with self.lock:
                now = time.monotonic()
                self.tokens = min(self.rate, self.tokens + (now - self.updated) * self.rate)
                self.updated = now
                if self.tokens >= 1:
                    self.tokens -= 1
                    return
            await asyncio.sleep(0.002)


BUCKET = AsyncBucket(QPS)
EXECUTOR = ThreadPoolExecutor(max_workers=POOL)


def upstream_call(body: bytes):
    request = urllib.request.Request(
        UPSTREAM, data=body,
        headers={"content-type": "application/json", "x-api-key": KEY},
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            return response.read(), response.status
    except urllib.error.HTTPError as error:
        return error.read(), error.code
    except Exception as error:  # network failure -> 502
        return json.dumps({"error": str(error)}).encode(), 502


async def handle(reader, writer):
    loop = asyncio.get_running_loop()
    try:
        while True:
            header = b""
            while b"\r\n\r\n" not in header:
                chunk = await reader.read(4096)
                if not chunk:
                    return
                header += chunk
                if len(header) > 64 * 1024:
                    return
            head, _, rest = header.partition(b"\r\n\r\n")
            lines = head.split(b"\r\n")
            request_line = lines[0].decode("latin1") if lines else ""
            headers = {}
            for line in lines[1:]:
                name, _, value = line.partition(b":")
                headers[name.strip().lower()] = value.strip()
            length = int(headers.get(b"content-length", b"0"))
            body = rest
            while len(body) < length:
                chunk = await reader.read(length - len(body))
                if not chunk:
                    return
                body += chunk

            if request_line.startswith("GET "):
                payload = json.dumps({"ok": True, "qps": QPS, **STATS}).encode()
                code = 200
            else:
                await BUCKET.take()
                payload, code = await loop.run_in_executor(EXECUTOR, upstream_call, body)
                STATS["ok" if code == 200 else "err"] += 1
                with open(LOG, "a", encoding="utf-8") as log:
                    log.write(f"{time.time():.3f} {code} {STATS['ok']} {STATS['err']}\n")

            response = (
                f"HTTP/1.1 {code} {'OK' if code == 200 else 'Error'}\r\n"
                f"content-type: application/json\r\n"
                f"content-length: {len(payload)}\r\n"
                f"connection: keep-alive\r\n\r\n"
            ).encode() + payload
            writer.write(response)
            await writer.drain()
            if headers.get(b"connection", b"").lower() == b"close":
                return
    except (ConnectionResetError, BrokenPipeError, asyncio.IncompleteReadError):
        pass
    finally:
        try:
            writer.close()
        except Exception:
            pass


async def main():
    server = await asyncio.start_server(handle, "127.0.0.1", PORT)
    print(f"async exa proxy on 127.0.0.1:{PORT} qps={QPS} pool={POOL}", flush=True)
    async with server:
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())
