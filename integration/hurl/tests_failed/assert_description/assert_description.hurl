GET http://localhost:8000/error-assert-value
HTTP 200
[Asserts]
jsonpath "$.count" == 0  # The count must be zero for an empty response
header "content-type" == "XXX" # Content type is always JSON
