from flask import Flask, request, Response
import yaml
import typing
import logging
import os
import tempfile
import json

logger = logging.getLogger("mock-server")

app = Flask(__name__)

type MockType = dict[str, dict[str, typing.Any]]


def setup_logging() -> None:
    log_path = os.path.join(tempfile.gettempdir(), "mocks.log")

    logging.basicConfig(
        level=logging.DEBUG,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        handlers=[
            logging.StreamHandler(),  # stderr
            logging.FileHandler(log_path, mode="a", encoding="utf-8"),
        ],
    )
    logger.info(f"Logging initialized. File: {log_path}")


def load_config(path: str) -> MockType:
    try:
        with open(path, "r") as f:
            return yaml.safe_load(f) or {}
    except Exception as e:
        logger.error(f"Failed to load YAML config: {e}")
        return {}


config: MockType = load_config("mocks.yaml")


@app.route("/<path:path>", methods=["GET", "POST", "PUT", "DELETE"])
def mock(path: str) -> Response:
    method = request.method.lower()
    entry: dict[str, typing.Any] | None = config.get(path, {}).get(method)

    logger.debug(f"Incoming {method.upper()} request to /{path}")
    logger.debug(f"Request headers: {dict(request.headers)}")
    logger.debug(f"Request body: {request.get_data(as_text=True)}")

    if not entry:
        logger.warning(f"No mock entry for /{path} [{method.upper()}]")
        return Response("Not found", status=404, content_type="text/plain")

    response_data: dict[str, typing.Any] = entry.get("response", {})
    content_type: str = response_data.get("content_type", "application/json")
    body: str | list[dict[str, typing.Any]] | dict[str, typing.Any] = response_data.get("body", {})
    status: int = entry.get("status", 200)

    if isinstance(body, (dict, list)):
        body_str = json.dumps(body)
    else:
        body_str = str(body)

    logger.info(f"Responding to /{path} [{method.upper()}] with {status}")
    return Response(body_str, status=status, content_type=content_type)


if __name__ == "__main__":
    setup_logging()
    app.run(port=8443, ssl_context=("../testcerts/cert.pem", "../testcerts/key.pem"))