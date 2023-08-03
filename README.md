# AirQ (Server)

# Superseded by [iot2db](https://github.com/oberien/iot2db)

Migration:
* configure airq and iot2db as explained in the iot2db device examples
* modify postgres from `psql` like this
    ```sql
    CREATE USER airq;
    ALTER DATABASE airq OWNER TO airq;
    \c airq
    SET ROLE airq;
    ALTER TABLE measurements OWNER TO airq;
    REVOKE CONNECT ON DATABASE airq FROM public;
    ALTER TABLE measurements DROP CONSTRAINT measurements_file_fkey;
    DROP TABLE files;
    DROP INDEX measurements_manual_readout;
    ALTER TABLE measurements ADD COLUMN persistent bool NOT NULL GENERATED ALWAYS AS (file IS NOT NULL) STORED;
    ALTER TABLE measurements ALTER COLUMN persistent DROP EXPRESSION;
    -- use the create table command from the iot2db documentation with a different name
    CREATE TABLE measurements2 (...) PARTITION BY ...;
    BEGIN TRANSACTION;
    LOCK TABLE measurements;
    ALTER TABLE measurements RENAME TO measurements_old;
    INSERT INTO measurements2 (
        timestamp, persistent, health, performance, tvoc, humidity, humidity_abs, temperature, dewpt, sound, pressure, no2, co, co2, pm1, pm2_5, pm10, oxygen, o3, so2
    ) SELECT timestamp, persistent, health, performance, tvoc, humidity, humidity_abs, temperature, dewpt, sound, pressure, no2, co, co2, pm1, pm2_5, pm10, oxygen, o3, so2 FROM measurements_old;
    ALTER TABLE measurements2 RENAME TO measurements;
    COMMIT;
    DROP TABLE measurements_old;
    ```

---

A server for displaying current AirQ Data, saving all data from the Airq in PostgreSQL,
and displaying historical data as graphs.
Postgres contains the data received from downloading the measurements stored within the airq, which equals one
data point every 2 minutes.
Additionally, for the last 7 days, manual readouts every 5 seconds are stored as well (`measuremnts.file` is NULL for those).

## Setup

0. Requirements:
   * avahi-daemon (arch: `avahi`, then enable and start `avahi-daemon.service`)
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
   
# Supported AirQ Firmware Versions

* **1.79**: 
* **1.75**: Waiting for this version
* **1.74**: Bug in the `/fetch_recrypt` API where lines aren't separated by `\n`
* **1.73**: OOM error whenever trying to use the `/fetch_recrypt` API
   
# Licensing

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by
you shall be dual licensed as above, without any additional terms or conditions.
