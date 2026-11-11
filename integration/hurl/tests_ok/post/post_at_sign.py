from app import app
from flask import request


@app.route("/post-at-sign", methods=["POST"])
def post_at_sign():
    # The request body is the literal text `@filename\n`: it must not be
    # interpreted by curl as a file reference.
    assert request.data == b"@filename\n"
    return ""
