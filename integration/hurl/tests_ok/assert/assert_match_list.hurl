# When a response header is present multiple times, the header query returns a list.
# The `matches` predicate succeeds if at least one item of the list matches the regex.
GET http://localhost:8000/assert-match-list
HTTP 200
[Asserts]
header "X-Fruit" matches /Lemon/
header "X-Fruit" matches /^Grape$/
header "X-Fruit" matches "(?i)strawberry"
header "X-Fruit" not matches /Apple/
header "Set-Cookie" matches "(?i)xsrf"
header "Set-Cookie" matches /^session=/
header "Set-Cookie" not matches /(?i)csrf/
