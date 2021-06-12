# AirQ (Server)

A server for displaying current AirQ Data, saving all data from the Airq in PostgreSQL,
and displaying historical data as graphs.

## Setup

0. Requirements:
   * avahi-daemon
   * libavahi-client-dev
   * libclang
1. Create and initialize a postgres database for airq data
   ```sh
   echo "SELECT 'CREATE DATABASE airq' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'airq')\gexec" | sudo -u postgres psql
   cat createdb.sql | sudo -u postgres psql airq
   ```
2. Build and install airq (make sure to have rust installed)
   ```sh
   make
   sudo make install
   ```
2. Change environment variables as needed via `sudo systemctl edit airq`:
   ```sh
   [Service]
   Environment="DATABASE_URL=postgres://postgres@localhost/airq"
   Environment="AIRQ_PASSWORD=airqsetup"
   ```
3. Enable and start airq
   ```sh
   systemctl enable airq
   systemctl start airq
   ```
   
# Licensing

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by
you shall be dual licensed as above, without any additional terms or conditions.