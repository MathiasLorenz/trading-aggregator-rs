# trading-results-rs

Proof-of-concept of aggregating some financial measures in Rust :happy-crustacian:

The project uses [sqlx](https://github.com/launchbadge/sqlx) for database connections.

Database is restored from a dump. I don't plan on building something which could generate data - but maybe at some point.
Could be cool to test larger data sets.

## Todos

- I have this idea of a channel based solution, where the db layer would create three producers and one consumer, each producer
sent to a spawned function to stream over a database table and send a trade back for each entry. The db layer would then aggregate the
three channels and send them all into a new channel that could be read from and then aggregated. The reason I think there needs to be two
separate sets of tx/rx is that we need to need when _all_ three table streams have finished before the report is finished, so I think there
needs to be a compononent to read the three input channels, pass them on and close the report channel when all three input channels are
exhausted.

## Connecting docker container to postgres in container
The db commands in the Taskfile will create a Docker network that is properly attached when running the container.
To connect the container to the Docker instance of postgres, the 'host' part of the DATABASE_URL must be switched
from 'localhost' to the name of the postgres container (is 'postgres' right now). This can be resolved becuase they're on
the same Docker network.

This should be injected properly by some settings/configurations - that is to-do!
