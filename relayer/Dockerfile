# Use an official Python runtime as a parent image
FROM python:3.12-slim

WORKDIR /app
RUN pip install uv
COPY . .
RUN uv pip install --no-cache --system -r requirements.lock

EXPOSE 8000

# CMD ["rye", "run", "uvicorn", "src.relayer.main:app", "--host", "0.0.0.0", "--port", "8000", "--reload"]
ENTRYPOINT [ "python3", "-m", "src.relayer.main"]