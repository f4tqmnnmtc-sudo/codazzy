FROM python:3.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y git build-essential curl && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY app/ ./app/
COPY config/ ./config/
COPY data/ ./data/

RUN mkdir -p /app/data/input /app/data/output /app/logs

EXPOSE 8000

ENV PYTHONPATH=/app
ENV MODEL_NAME=amazon/chronos-t5-base
ENV LOG_LEVEL=INFO

HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
