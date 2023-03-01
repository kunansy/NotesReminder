#!/bin/sh

useradd -ms /bin/bash reminder
mkdir /app
chown -R reminder /app
