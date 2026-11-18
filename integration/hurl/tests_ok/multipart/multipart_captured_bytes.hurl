# Captured bytes can be re-injected in a multipart form data param: the value is sent
# as-is, without any loss (see <https://github.com/Orange-OpenSource/hurl/issues/3752>).
GET http://localhost:8000/multipart-form-data-backup
HTTP 200
[Captures]
backup: bytes

POST http://localhost:8000/multipart-form-data-field
[Multipart]
backup: {{backup}}
HTTP 200


# Captured bytes can also be re-injected in a multipart form data built with a multiline
# string body, giving full control on the parts (filenames, content types etc...).
GET http://localhost:8000/multipart-form-data-backup
HTTP 200
[Captures]
backup: bytes

POST http://localhost:8000/multipart-form-data-upload
Content-Type: multipart/form-data; boundary="boundary"
```
--boundary
Content-Disposition: form-data; name="backup"; filename="backup.bin"
Content-Type: application/octet-stream

{{backup}}
--boundary--
```
HTTP 200
