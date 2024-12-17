#!/usr/bin/env bash

RUNNING_RQBIT_CONTAINER=$(docker ps --filter 'name=rqbit' --format '{{.ID}}')
if [[ -n $RUNNING_RQBIT_CONTAINER ]]; then
  # Remove the existing rqbit container
  docker rm -f $RUNNING_RQBIT_CONTAINER
fi

# Launch rqbit using Docker
docker run \
    -p 3030:3030 \
    --env-file .env \
    -v "$(pwd)/rqbit_db:/home/rqbit/db" \
    -v "$(pwd)/rqbit_cache:/home/rqbit/cache" \
    -v "/goinfre/Downloads:/home/rqbit/downloads" \
    -d \
    --name "rqbit" \
    ikatson/rqbit