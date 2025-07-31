# Use an official Python runtime as a parent image
FROM python:3.12-slim

WORKDIR /app
RUN apt-get update \
    && apt-get install -y curl \
    && rm -rf /var/lib/apt/lists/* \
    && pip install uv
COPY . .
RUN uv pip install --no-cache --system -r requirements.lock

EXPOSE 4000

ENTRYPOINT [ "python3", "-m", "src.resolver.main"]
