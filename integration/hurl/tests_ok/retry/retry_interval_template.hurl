# The retry interval can be set from a variable holding a duration with a unit.
# Create a new job, then pull its status until it is completed.
POST http://localhost:8000/jobs
HTTP 201
[Captures]
job_id: jsonpath "$.id"

GET http://localhost:8000/jobs/{{job_id}}
[Options]
retry: 10
retry-interval: {{interval}}
HTTP 200
[Asserts]
jsonpath "$.state" == "COMPLETED"
