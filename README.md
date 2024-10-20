# trading-results-rs

..

## Connecting docker container to postgres in container
The db commands in the Taskfile will create a Docker network that is properly attached when running the container.
To connect the container to the Docker instance of postgres, the 'host' part of the DATABASE_URL must be switched
from 'localhost' to the name of the postgres container (is 'postgres' right now). This can be resolved becuase they're on
the same Docker network.

This should be injected properly by some settings/configurations - that is to-do!
