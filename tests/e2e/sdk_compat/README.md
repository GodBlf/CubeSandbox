# SDK Compatibility E2E Tests

This directory contains live end-to-end tests that exercise the same user flows
through multiple Python SDKs:

- `cubesandbox`: CubeSandbox Python SDK from `sdk/python`.
- `e2b`: E2B Python SDK (`e2b-code-interpreter` or `e2b`) against the same CubeSandbox-compatible backend.

The suite is opt-in. Test collection is safe by default and all cases are skipped
unless `--run-e2e` is passed. Live runs default to the `cubesandbox` backend;
set `SDK_E2E_BACKENDS=e2b,cubesandbox` or pass `--sdk-e2e-backends` for dual SDK
compatibility runs.

## Layout

- `adapters/`: SDK-specific shims that expose a small shared test surface.
- `framework/`: configuration, preflight checks, capability flags, cleanup, assertions, and reporting helpers.
- `cases/`: backend-neutral test cases, split by SDK capability domain.
- `reports/`: local JSONL execution events. This directory is ignored by Git.

## Test Classification

This suite follows the same classification style used by `CubeSandboxTest`:

- `smoke`: minimal live-environment checks, intended to prove that CubeAPI, the template, and at least one SDK backend are usable.
- `p0`: PR-gate compatibility coverage. These tests should be small, deterministic, and safe to run often.
- `p1`: daily compatibility regression. These tests cover broader lifecycle or CubeSandbox-specific behavior.
- `p2`: weekly compatibility coverage for slower or more specialized SDK features.
- `p3`: release qualification and long-running scenarios.

The current case layout is:

- `cases/lifecycle/`: sandbox create/info smoke checks, plus `p1` pause/resume coverage for backends that support `pause_resume`.
- `cases/commands/`: command stdout/stderr/exit-code handling, environment access, special characters, multiline output, and missing command behavior.
- `cases/filesystem/`: file API read/write, overwrite, multiline content, and interoperability between file APIs and shell commands.
- `cases/run_code/`: Code Interpreter execution, expression text, stdout capture, kernel statefulness, and Python error reporting.

The broader `CubeSandboxTest` repository uses this module split:

- `smoke`: health and minimal CRUD
- `api`: REST API lifecycle and template coverage
- `sdk`: SDK commands, filesystem, info, and compatibility
- `network`: outbound DNS/IP/network policy
- `proxy`: CubeProxy routing
- `isolation`: sandbox isolation and security boundaries
- `extensions`: host mount and browser sandbox
- `performance`: latency and concurrency
- `stability`: TTL and long-running checks
- `resilience`: component restart and recovery
- `cli`: operational CLI coverage

For this in-repo SDK compatibility suite, keep test modules backend-neutral and use markers/capabilities for backend-specific behavior. As coverage grows, add new capability domains next to the existing directories, for example `network/`, `proxy/`, `snapshot/`, and `metrics/`.

## Quick Start

Run only the CubeSandbox SDK backend:

```bash
cd tests/e2e/sdk_compat
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
export SDK_E2E_BACKENDS=cubesandbox
export CUBE_API_URL=http://10.0.1.5:3000
export CUBE_TEMPLATE_ID=tpl-xxxxxxxxxxxxxxxxxxxxxxxx
export CUBE_PROXY_NODE_IP=10.0.1.2
pytest --run-e2e
```

Run only PR-gate compatibility tests:

```bash
pytest --run-e2e -m p0
```

Run smoke only:

```bash
pytest --run-e2e -m smoke
```

Run daily regression scope:

```bash
pytest --run-e2e -m "p0 or p1"
```

Run both backends after installing E2B:

```bash
pip install e2b-code-interpreter
export SSL_CERT_FILE=/root/.local/share/mkcert/rootCA.pem
export SDK_E2E_BACKENDS=e2b,cubesandbox
pytest --run-e2e
```

If the self-hosted sandbox HTTPS certificates are signed by a local CA, prefer
trusting that CA through `SSL_CERT_FILE`. This keeps TLS verification enabled
for the E2B SDK command/files/run_code transport:

```bash
export SSL_CERT_FILE=/root/.local/share/mkcert/rootCA.pem
export SDK_E2E_E2B_INSECURE_TLS=false
export SDK_E2E_BACKENDS=e2b,cubesandbox
pytest --run-e2e
```

For local test environments where the CA is unavailable, use
`SDK_E2E_E2B_INSECURE_TLS=true` as a fallback.

## Preflight

When `--run-e2e` is enabled, the suite runs a session preflight before creating
per-test sandboxes:

- `CUBE_TEMPLATE_ID` or `--cube-template-id` is present.
- `GET /health` on `CUBE_API_URL` is reachable.
- `GET /templates/{template_id}` returns the selected template.
- If the template response exposes `status` or `state`, it must be ready-like
  (`ready`, `active`, or `available`).

Preflight failures are recorded in `reports/sdk-dual/events.jsonl` as
`preflight_failed` and stop the run early with a single diagnostic message.

You can also pass options instead of environment variables:

```bash
pytest --run-e2e \
  --sdk-e2e-backends=cubesandbox \
  --cube-api-url=http://10.0.1.5:3000 \
  --cube-template-id=tpl-xxxxxxxxxxxxxxxxxxxxxxxx
```

## Required Environment

- `CUBE_API_URL`: CubeAPI endpoint.
- `CUBE_TEMPLATE_ID`: ready template ID used for sandbox creation.
- `CUBE_PROXY_NODE_IP`: optional, but useful when wildcard sandbox DNS is not available from the runner.
- `CUBE_PROXY_PORT_HTTP`: defaults to `80`.
- `CUBE_SANDBOX_DOMAIN`: defaults to `cube.app`.
- `SSL_CERT_FILE`: optional CA bundle path for self-hosted sandbox HTTPS certificates, for example `/root/.local/share/mkcert/rootCA.pem`.
- `SDK_E2E_E2B_INSECURE_TLS`: disables TLS certificate verification for the E2B SDK sandbox transport. Prefer `SSL_CERT_FILE` when the local CA is available. This option defaults to `true` when `CUBE_API_URL` starts with `http://`, which matches local/self-hosted CubeSandbox test deployments.

## Reporting

The suite writes JSONL events to `SDK_E2E_REPORT_DIR/events.jsonl`, defaulting to
`reports/sdk-dual/events.jsonl`.

Current event types:

- `preflight_passed` / `preflight_failed`: live environment readiness checks.
- `sandbox_created`: sandbox ID, backend, and pytest node ID.
- `sandbox_cleanup` / `sandbox_kept`: teardown outcome.
- `test_result`: pytest phase, outcome, duration, backend, sandbox ID, and failure
  diagnostics. Failed test results include `error` and best-effort
  `sandbox_info` when available.

## Cleanup

Each test creates its own sandbox and destroys it in teardown. If SDK teardown
fails, the suite falls back to `DELETE /sandboxes/{sandboxID}` against `CUBE_API_URL`.
Set `SDK_E2E_KEEP_SANDBOX_ON_FAILURE=true` to preserve sandboxes while debugging.

## Capability Markers

Use `@pytest.mark.requires_capability("<name>")` for features that are not shared
by every backend. Current common coverage includes lifecycle create/delete,
commands, filesystem, and `run_code`; CubeSandbox-specific coverage currently
includes pause/resume.
