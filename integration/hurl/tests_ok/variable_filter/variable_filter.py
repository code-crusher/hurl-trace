from app import app
from flask import request


@app.route("/variable-filter", methods=["POST"])
def variable_filter():
    s = request.data.decode("utf-8")
    assert s == '{"records":[{"message":"dGVzdA=="}]}\n'
    return ""