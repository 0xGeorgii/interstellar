# Deploying with GHCR images

This folder includes a Docker Compose manifest that pulls the pre-built images
from GitHub Container Registry (GHCR) instead of building them locally.

## Prerequisites

- You must have pushed the `ui`, `resolver`, and `relayer` images to GHCR using the
  provided GitHub Actions workflow (`.github/workflows/docker-publish.yml`).
- Set the following environment variables in your shell or a `.env` file:
  - `EVM_SECRET`
  - `STELLAR_SECRET`
  - `ETHEREUM_RPC`
  - `ETHEREUM_ESCROW_ABI`
  - `ETHEREUM_ESCROW_ADDRESS`
  - `STELLAR_RPC`
  - `STELLAR_CONTRACT_ID`
  - `MODE` (optional, defaults to `S`)

## Usage

Run the following command from the repo root:

```bash
docker-compose -f deploy/docker-compose-ghcr.yml up
```
