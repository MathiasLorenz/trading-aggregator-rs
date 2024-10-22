# trading-results-rs

Proof-of-concept of aggregating some financial measures in Rust :happy-crustacian:

The project uses

- [sqlx](https://docs.rs/sqlx/latest/sqlx/) for database connections
- [chrono](https://docs.rs/chrono/latest/chrono/) and [chrono-tz](https://docs.rs/chrono-tz/latest/chrono_tz/) timestamps and timezones
- [rust_decimal](https://docs.rs/rust_decimal/latest/rust_decimal/) for decimals for no-roundoff errors math when necessary

Database is restored from a dump and should be running as a local Docker instance of Postgres -
convinience functions are provided in the [Taskfile.yaml](Taskfile.yaml).
I don't plan on building something which can generate data, but maybe at some point.
That could be cool to test larger data sets.

There are no tests as I can compare aggregations against a ground truth. Would be cool to add some tests, though :p

The database 'schema' and enum <-> string shenanigans in [db.rs](src/db.rs) are due to how the database schema is set up in the baseline data
and is simply something we have to work with.

## 'Benchmarks'

Current (very not scientific) benchmarks for aggregating around 600K trades:

- `main/create_report`: 'Naive' version in main, 570ms to get trades from db, 75ms to aggregate all into report = 645ms in total
- `main/create_report_from_simple_trade`: 'Naive' with minimal set of properties, 440ms to get trades from db, 75ms to aggregate = 515ms in total
- `main/create_report_stream`: Stream based solution where result are streamed from the database: 625ms in total (as entries are processed as they are retrieved)

So generating a report takes around 75ms, where we can 'assimilate' some of that cost into the retrievel stage when
using the stream solution. Of course there could be created a streaming `TradeForReport` solution, which would have the naive solution
as baseline and we'd (probably) see the same improvements as the for the 'naive' -> 'stream' solution. I think the channel based solution mentioned below
is more interesting to look at than this.

## Todos

- I have this idea of a channel based solution, where the db layer would create three producers and one consumer, each producer
sent to a spawned function to stream over a database table and send a trade back for each entry. The db layer would then aggregate the
three channels and send them all into a new channel that could be read from and then aggregated. The reason I think there needs to be two
separate sets of tx/rx is that we need to need when _all_ three table streams have finished before the report is finished, so I think there
needs to be a compononent to read the three input channels, pass them on and close the report channel when all three input channels are
exhausted.
- Make a more robus benchmark setup. It would also be interesting to see how solutions do 'without' the db as
that is a clear bottleneck right now (of course it will always be in the real world, but the aggregation performance is interesting anyway).
- Maybe create a CLI and/or webapp to call this from?
Could be split into three projects/crates: core aggregator (report and db access), CLI and web app. The CLI/webapp projects would then take
the core aggregator as a dependency and could expose APIs and CLIs with benchmarks etc. Obvious choices for technologies would
be `clap` and `axum` which I have used a bit before.
- Inject settings/configurations - would be cool for differences in database url for running project locally and
in a Docker container.

## Connecting docker container to postgres in container
The db commands in the `Taskfile` will create a Docker network that is properly attached when running the container.
To connect the container to the Docker instance of postgres, the 'host' part of the `DATABASE_URL` must be switched
from 'localhost' to the name of the postgres container (is 'postgres' right now). This can be resolved becuase they're on
the same Docker network.

This should be injected properly by some settings/configurations - that is to-do!
