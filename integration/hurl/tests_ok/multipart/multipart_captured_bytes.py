from app import app
from flask import request


BACKUP = b"\x00\x01\x02Hello World!\xff\xfe"


@app.route("/multipart-form-data-backup", methods=["GET"])
def multipart_form_data_backup():
    """Returns a binary backup, with a NUL byte and non UTF-8 bytes."""
    return BACKUP


@app.route("/multipart-form-data-field", methods=["POST"])
def multipart_form_data_field():
    """Receives a backup as a multipart form field value.

    The backup contains non UTF-8 bytes, so we assert on the raw request bytes: the
    captured bytes are re-injected in the field value without any loss.
    """
    assert b'name="backup"' in request.get_data()
    assert request.get_data().split(b"\r\n\r\n")[1].split(b"\r\n")[0] == BACKUP
    return ""


@app.route("/multipart-form-data-upload", methods=["POST"])
def multipart_form_data_upload():
    """Receives a backup as a multipart file upload."""
    upload = request.files["backup"]
    assert upload.filename == "backup.bin"
    assert upload.content_type == "application/octet-stream"
    assert upload.read() == BACKUP
    return ""
