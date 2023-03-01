FROM python:3.11-slim-buster

LABEL maintainer="<kolobov.kirill@list.ru>"
ENV PYTHONUNBUFFERED 1
ENV PYTHONPATH .

COPY --from=umputun/cronn:latest /srv/cronn /srv/cronn

RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install gcc -y \
    && pip install poetry --no-cache-dir \
    && rm -rf /var/lib/apt/lists/*

COPY poetry.lock pyproject.toml entrypoint.sh /
RUN poetry config virtualenvs.create false \
    && poetry install --no-dev -n \
    && rm poetry.lock pyproject.toml entrypoint.sh

USER reminder
WORKDIR /app

COPY VERSION /app/VERSION
COPY /reminder /app/reminder
