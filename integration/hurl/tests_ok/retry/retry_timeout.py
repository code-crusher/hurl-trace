from datetime import datetime, timezone

from app import app

last = None
count = 0


@app.route("/retry-timeout-init")
def retry_timeout_init():
    global last, count
    last = None
    count = 0
    return ""


@app.route("/retry-timeout")
def retry_timeout():
    global last, count
    now = datetime.now(timezone.utc)
    count += 1
    if count == 1:
        # The first call fails, so the entry is retried.
        last = now
        return {"state": "RUNNING"}, 500
    # The second call is done after a sleep bounded by the retry timeout, not by the retry
    # interval (which is set to 10 seconds in the Hurl file).
    diff = (now - last).total_seconds()
    assert 0.2 < diff < 3, f"unexpected delay between retries: {diff}"
    return {"state": "COMPLETED"}
