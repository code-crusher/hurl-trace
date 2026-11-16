# Variables can be transformed with filters in the `[Options]` section:
# the `base64_data` variable is base64 encoded with the `utf8Encode` and
# `base64Encode` filters before being used in the request body.
POST http://localhost:8000/variable-filter
[Options]
variable: base64_data="test" utf8Encode base64Encode
```
{"records":[{"message":"{{base64_data}}"}]}
```
HTTP 200
