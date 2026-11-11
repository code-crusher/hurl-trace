# First, reset the server state
GET http://localhost:8000/retry-timeout-init
HTTP 200

# The first call fails, the entry is retried. The retry interval is much higher than the retry
# timeout, so the sleep between the two calls must be bounded by the retry timeout.
GET http://localhost:8000/retry-timeout
[Options]
retry: 10
retry-interval: 10s
retry-timeout: 1s
HTTP 200
[Asserts]
jsonpath "$.state" == "COMPLETED"
