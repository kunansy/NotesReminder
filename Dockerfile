FROM python:3.11-slim-buster as reading-tracker

LABEL maintainer="<kolobov.kirill@list.ru>"
ENV PYTHONUNBUFFERED 1
ENV PYTHONPATH .

RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install gcc -y \
    && pip install poetry --no-cache-dir \
    && rm -rf /var/lib/apt/lists/*

COPY --from=umputun/cronn:latest /srv/cronn /srv/cronn

WORKDIR /app

COPY poetry.lock pyproject.toml /app/
RUN poetry config virtualenvs.create false \
    && poetry install --no-dev -n

COPY VERSION /app/VERSION
COPY /reminder /app/reminder
