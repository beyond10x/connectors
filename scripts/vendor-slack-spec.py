#!/usr/bin/env python3
"""Vendor Slack's live Swagger 2 source and derive the exact reviewed Web API projections.

The live vendor document is evidence, not selection. This script deliberately knows the closed
operation inventory below, removes credential-bearing `token` parameters, scrubs all examples, and
converts only those operations into the OpenAPI 3 shape the offline connector compiler accepts.
Changing the inventory or the pinned upstream hash is a review action in this file.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import pathlib
import sys
import urllib.request
from typing import Any

SOURCE_URL = "https://api.slack.com/specs/openapi/v2/slack_web.json"
UPSTREAM_VERSION = "1.7.0"
FETCHED_AT = "2026-08-15T00:00:00Z"
UPSTREAM_SHA256 = "1f41356634b6636d1cd64ba68b72aa12b76989bde483c334f6069e2828e9f2d5"

GROUPS = {
    "web": (
        "chat_postMessage",
        "conversations_history",
        "users_info",
        "reactions_add",
    ),
    "admin": (
        "admin_apps_requests_list",
        "admin_conversations_search",
        "admin_teams_list",
        "admin_users_list",
    ),
}

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "specs" / "slack"
VENDORED = OUT / "web-api-1.7.0-2026-08-15.swagger.json"
DERIVED = {
    "web": OUT / "web-api-selected-1.7.0-2026-08-15.openapi.json",
    "admin": OUT / "admin-api-selected-1.7.0-2026-08-15.openapi.json",
}


def compact_hash(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_source(path: pathlib.Path | None) -> bytes:
    if path is not None:
        return path.read_bytes()
    with urllib.request.urlopen(SOURCE_URL, timeout=30) as response:  # noqa: S310 - fixed URL
        return response.read()


def scrub(value: Any) -> Any:
    """Remove example material recursively; it is neither schema nor safe repository data."""
    if isinstance(value, dict):
        return {
            key: scrub(child)
            for key, child in value.items()
            if key not in {"example", "examples"}
        }
    if isinstance(value, list):
        return [scrub(child) for child in value]
    return value


def rewrite_refs(value: Any) -> Any:
    if isinstance(value, dict):
        rewritten = {key: rewrite_refs(child) for key, child in value.items()}
        ref = rewritten.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/definitions/"):
            rewritten["$ref"] = ref.replace(
                "#/definitions/", "#/components/schemas/", 1
            )
        return rewritten
    if isinstance(value, list):
        return [rewrite_refs(child) for child in value]
    return value


SCHEMA_KEYS = {
    "type",
    "format",
    "items",
    "enum",
    "default",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
    "uniqueItems",
    "pattern",
}


def parameter_schema(parameter: dict[str, Any]) -> dict[str, Any]:
    if "schema" in parameter:
        return rewrite_refs(copy.deepcopy(parameter["schema"]))
    schema = {
        key: copy.deepcopy(value)
        for key, value in parameter.items()
        if key in SCHEMA_KEYS
    }
    if not schema:
        raise ValueError(f"parameter {parameter.get('name')!r} has no schema")
    return rewrite_refs(schema)


def response_schema(operation: dict[str, Any]) -> dict[str, Any] | None:
    responses = operation.get("responses", {})
    success = responses.get("200", {}).get("schema")
    failure = responses.get("default", {}).get("schema")
    if success is None and failure is None:
        return None

    variants: list[dict[str, Any]] = []
    if success is not None:
        success = rewrite_refs(copy.deepcopy(success))
        # Slack's live source omits `ok` from a few success objects even though Web API responses
        # use it. This reviewed repair also keeps the HTTP-200 error contract discriminable.
        if success.get("type") == "object":
            success.setdefault("properties", {}).setdefault(
                "ok", {"type": "boolean", "const": True}
            )
            required = success.setdefault("required", [])
            if "ok" not in required:
                required.insert(0, "ok")
        variants.append(success)
    if failure is not None:
        failure = rewrite_refs(copy.deepcopy(failure))
        if failure.get("type") == "object":
            failure.setdefault("properties", {}).setdefault(
                "error",
                {
                    "type": "string",
                    "description": "Slack's machine-readable error code",
                },
            )
            required = failure.setdefault("required", [])
            if "error" not in required:
                required.append("error")
        variants.append(failure)
    return variants[0] if len(variants) == 1 else {"oneOf": variants}


def convert_operation(operation: dict[str, Any]) -> dict[str, Any]:
    converted: dict[str, Any] = {
        "operationId": operation["operationId"],
        "description": operation.get("description", ""),
        "responses": {"200": {"description": "Slack Web API response"}},
    }
    if operation.get("externalDocs"):
        converted["externalDocs"] = copy.deepcopy(operation["externalDocs"])

    parameters: list[dict[str, Any]] = []
    body_parameters: list[dict[str, Any]] = []
    body_schema: dict[str, Any] | None = None
    body_required = False
    for parameter in operation.get("parameters", []):
        parameter = copy.deepcopy(parameter)
        if parameter.get("name") == "token":
            continue
        position = parameter.get("in")
        if position == "body":
            body_schema = parameter_schema(parameter)
            body_required = bool(parameter.get("required"))
            continue
        if position == "formData":
            body_parameters.append(parameter)
            continue
        if position not in {"query", "header", "path"}:
            raise ValueError(
                f"{operation['operationId']}: unsupported parameter position {position!r}"
            )
        parameters.append(
            {
                "name": parameter["name"],
                "in": position,
                "required": bool(parameter.get("required", position == "path")),
                "description": parameter.get("description", ""),
                "schema": parameter_schema(parameter),
            }
        )
    if parameters:
        converted["parameters"] = parameters

    if body_parameters:
        properties = {
            parameter["name"]: {
                **parameter_schema(parameter),
                **(
                    {"description": parameter["description"]}
                    if parameter.get("description")
                    else {}
                ),
            }
            for parameter in body_parameters
        }
        required = [
            parameter["name"]
            for parameter in body_parameters
            if parameter.get("required")
        ]
        body_schema = {"type": "object", "properties": properties}
        if required:
            body_schema["required"] = required
        body_required = bool(required)

    if body_schema is not None:
        consumes = operation.get("consumes", [])
        # Slack documents JSON for the curated methods that need a body. Prefer it so opaque text
        # cannot be reinterpreted as form structure; retain form only when JSON is unavailable.
        media_type = (
            "application/json"
            if "application/json" in consumes
            else "application/x-www-form-urlencoded"
        )
        converted["requestBody"] = {
            "required": body_required,
            "content": {media_type: {"schema": body_schema}},
        }

    schema = response_schema(operation)
    if schema is not None:
        converted["responses"]["200"]["content"] = {
            "application/json": {"schema": schema}
        }
    return converted


def selected_operations(source: dict[str, Any]) -> dict[str, tuple[str, str, dict[str, Any]]]:
    selected: dict[str, tuple[str, str, dict[str, Any]]] = {}
    wanted = {operation_id for group in GROUPS.values() for operation_id in group}
    for path, path_item in source["paths"].items():
        for method, operation in path_item.items():
            if not isinstance(operation, dict):
                continue
            operation_id = operation.get("operationId")
            if operation_id in wanted:
                selected[operation_id] = (path, method, operation)
    missing = sorted(wanted - selected.keys())
    if missing:
        raise ValueError(f"live Slack source no longer declares selected operations: {missing}")
    return selected


def collect_component_names(value: Any) -> set[str]:
    names: set[str] = set()
    if isinstance(value, dict):
        ref = value.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/components/schemas/"):
            names.add(ref.rsplit("/", 1)[1])
        for child in value.values():
            names.update(collect_component_names(child))
    elif isinstance(value, list):
        for child in value:
            names.update(collect_component_names(child))
    return names


def close_components(
    document: dict[str, Any], source_definitions: dict[str, Any]
) -> dict[str, Any]:
    components: dict[str, Any] = {}
    pending = sorted(collect_component_names(document))
    while pending:
        name = pending.pop(0)
        if name in components:
            continue
        if name not in source_definitions:
            raise ValueError(f"selected projection references missing definition {name!r}")
        schema = rewrite_refs(copy.deepcopy(source_definitions[name]))
        components[name] = schema
        for dependency in sorted(collect_component_names(schema)):
            if dependency not in components and dependency not in pending:
                pending.append(dependency)
        pending.sort()
    return dict(sorted(components.items()))


def projection(
    source: dict[str, Any],
    selected: dict[str, tuple[str, str, dict[str, Any]]],
    group: str,
) -> dict[str, Any]:
    paths: dict[str, Any] = {}
    for operation_id in GROUPS[group]:
        path, method, operation = selected[operation_id]
        # The Swagger document's basePath is `/api`; embed it in each derived path because the
        # connector's stable base URL remains `https://slack.com`.
        paths[f"/api{path}"] = {method: convert_operation(operation)}
    document: dict[str, Any] = {
        "openapi": "3.0.3",
        "info": {
            "title": f"Slack {'Admin' if group == 'admin' else 'Web'} API — reviewed selection",
            "version": source["info"]["version"],
            "description": (
                "B10x-derived exact projection of Slack's live Swagger 2 source. "
                "Selection and risk/auth overlays remain in providers/slack.toml."
            ),
        },
        "servers": [{"url": "https://slack.com"}],
        "paths": paths,
    }
    schemas = close_components(document, source.get("definitions", {}))
    if schemas:
        document["components"] = {"schemas": schemas}
    return document


def encoded(document: dict[str, Any]) -> bytes:
    return (json.dumps(document, indent=2, sort_keys=True) + "\n").encode()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--source",
        type=pathlib.Path,
        help="Replay from already-fetched upstream bytes instead of using the network",
    )
    args = parser.parse_args()

    upstream = read_source(args.source)
    measured = compact_hash(upstream)
    if measured != UPSTREAM_SHA256:
        print(
            f"Slack source drift: expected {UPSTREAM_SHA256}, got {measured}; "
            "review the upstream diff before updating the pin",
            file=sys.stderr,
        )
        return 1
    source = json.loads(upstream)
    if source.get("swagger") != "2.0" or source.get("info", {}).get("version") != UPSTREAM_VERSION:
        raise ValueError("Slack source no longer has the pinned Swagger/version identity")
    scrubbed = scrub(source)
    # The current hosted document carries no example keys, so preserve its exact bytes. If Slack
    # later adds any, the pin update must also decide and review the value scrub before vendoring.
    if scrubbed != source:
        raise ValueError(
            "Slack source now carries example material; review and declare the scrub before updating"
        )
    source = scrubbed
    selected = selected_operations(source)

    OUT.mkdir(parents=True, exist_ok=True)
    VENDORED.write_bytes(upstream)
    for group, path in DERIVED.items():
        path.write_bytes(encoded(projection(source, selected, group)))

    print(f"upstream_sha256={UPSTREAM_SHA256}")
    for path in (VENDORED, *DERIVED.values()):
        data = path.read_bytes()
        print(f"{path.relative_to(ROOT)} sha256={compact_hash(data)} bytes={len(data)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
