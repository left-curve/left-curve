"""Tests for dango.api.API."""

import time

import pytest
from pytest_httpserver import HTTPServer
from werkzeug.wrappers import Request, Response

from dango.api import API
from dango.utils.error import ClientError, GraphQLError, ServerError


def _api(httpserver: HTTPServer) -> API:
    return API(httpserver.url_for("/").rstrip("/"))


def _slow_handler(_request: Request) -> Response:
    time.sleep(0.2)
    return Response('{"data":{}}', mimetype="application/json")


class TestQuery:
    def test_success_returns_data(self, httpserver: HTTPServer) -> None:
        """200 OK with a `data` field returns the data dict."""

        httpserver.expect_request("/graphql", method="POST").respond_with_json(
            {"data": {"foo": "bar"}}
        )
        result = _api(httpserver).query("query { foo }")

        assert result == {"foo": "bar"}

    def test_sends_json_body(self, httpserver: HTTPServer) -> None:
        """The query and variables are POSTed as the JSON body."""

        httpserver.expect_request(
            "/graphql",
            method="POST",
            json={"query": "query { foo }", "variables": {"x": 1}},
        ).respond_with_json({"data": {"foo": "bar"}})

        _api(httpserver).query("query { foo }", variables={"x": 1})

    def test_variables_default_to_empty_dict(self, httpserver: HTTPServer) -> None:
        """Omitting variables sends `variables: {}` (not `null` or absent)."""

        httpserver.expect_request(
            "/graphql",
            method="POST",
            json={"query": "query { foo }", "variables": {}},
        ).respond_with_json({"data": {"foo": "bar"}})

        _api(httpserver).query("query { foo }")

    def test_graphql_errors_array_raises(self, httpserver: HTTPServer) -> None:
        """A non-empty `errors` array on a 200 response raises GraphQLError."""

        httpserver.expect_request("/graphql").respond_with_json(
            {"errors": [{"message": "boom", "path": ["foo"]}]}
        )
        with pytest.raises(GraphQLError, match="boom"):
            _api(httpserver).query("query { foo }")

    def test_4xx_raises_client_error(self, httpserver: HTTPServer) -> None:
        """Any 4xx HTTP response is mapped to ClientError."""

        httpserver.expect_request("/graphql").respond_with_data("bad", status=400)
        with pytest.raises(ClientError, match="400"):
            _api(httpserver).query("query { foo }")

    def test_5xx_raises_server_error(self, httpserver: HTTPServer) -> None:
        """Any 5xx HTTP response is mapped to ServerError."""

        httpserver.expect_request("/graphql").respond_with_data("oops", status=500)
        with pytest.raises(ServerError, match="500"):
            _api(httpserver).query("query { foo }")

    def test_non_json_response_raises_server_error(self, httpserver: HTTPServer) -> None:
        """A 200 response with a non-JSON body is treated as a server malformation."""

        httpserver.expect_request("/graphql").respond_with_data("not json", status=200)
        with pytest.raises(ServerError, match="non-JSON"):
            _api(httpserver).query("query { foo }")

    def test_missing_data_and_errors_raises(self, httpserver: HTTPServer) -> None:
        """An empty `{}` response (no data, no errors) raises GraphQLError."""

        httpserver.expect_request("/graphql").respond_with_json({})
        with pytest.raises(GraphQLError):
            _api(httpserver).query("query { foo }")

    def test_non_dict_json_raises_server_error(self, httpserver: HTTPServer) -> None:
        """A 200 response with JSON-but-not-a-dict (e.g. array) raises ServerError."""

        httpserver.expect_request("/graphql").respond_with_json([{"data": {}}])
        with pytest.raises(ServerError, match="not a JSON object"):
            _api(httpserver).query("query { foo }")

    def test_connection_refused_raises_server_error(self) -> None:
        """A network-level connection failure is wrapped as ServerError, not leaked."""

        api = API("http://127.0.0.1:1")
        with pytest.raises(ServerError, match="request to http://127.0.0.1:1"):
            api.query("query { foo }")

    def test_timeout_raises_server_error(self, httpserver: HTTPServer) -> None:
        """A request that exceeds `timeout` is wrapped as ServerError."""

        httpserver.expect_request("/graphql").respond_with_handler(_slow_handler)
        api = API(httpserver.url_for("").rstrip("/"), timeout=0.05)
        with pytest.raises(ServerError, match="request to"):
            api.query("query { foo }")


class TestQueryTyped:
    def test_returns_cast_value(self, httpserver: HTTPServer) -> None:
        """query_typed returns the same data dict as query, just typed."""

        httpserver.expect_request("/graphql").respond_with_json({"data": {"x": 42}})
        result: dict[str, int] = _api(httpserver).query_typed(
            "query { x }", response_type=dict[str, int]
        )
        assert result == {"x": 42}

    def test_does_not_validate_response_type(self, httpserver: HTTPServer) -> None:
        """query_typed is cast-only: a mismatched response_type does NOT raise."""

        httpserver.expect_request("/graphql").respond_with_json({"data": {"x": 42}})
        result: object = _api(httpserver).query_typed("query { x }", response_type=int)
        assert result == {"x": 42}


class TestConstruction:
    def test_strips_trailing_slash(self) -> None:
        """A trailing slash on base_url is stripped to avoid `//graphql` in the URL."""

        api = API("http://example.com/")
        assert api.base_url == "http://example.com"

    def test_timeout_defaults_to_none(self) -> None:
        """timeout=None means no client-side timeout (matches requests.post default)."""

        api = API("http://example.com")
        assert api.timeout is None
