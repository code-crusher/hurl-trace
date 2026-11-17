from app import app
from flask import Response


@app.route("/assert-match-list")
def assert_match_list():
    headers = {
        # A header with multiple values is queried as a list.
        "x-fruit": ["Banana", "Lemon", "Grape", "Strawberry"],
    }
    resp = Response("", headers=headers)
    resp.set_cookie("XSRF-TOKEN", "value1")
    resp.set_cookie("session", "value2")
    return resp
