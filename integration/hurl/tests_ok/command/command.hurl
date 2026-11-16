# Run a command before the HTTP request and a command after the response,
# then assert on their return code, standard output and standard error.

GET http://localhost:8000/hello
[Command]
pre-request: echo hello
post-request: echo world
HTTP 200
[Asserts]
command "pre-request" ret-code == 0
command "pre-request" stdout contains "hello"
command "pre-request" stderr == ""
command "post-request" ret-code == 0
command "post-request" stdout contains "world"


# Commands are templatized with variables.
GET http://localhost:8000/hello
[Command]
pre-request: echo {{name}}
HTTP 200
[Asserts]
command "pre-request" stdout == "Bob\n"


# A command can write on standard error.
GET http://localhost:8000/hello
[Command]
pre-request: echo oops 1>&2
HTTP 200
[Asserts]
command "pre-request" stdout == ""
command "pre-request" stderr contains "oops"
