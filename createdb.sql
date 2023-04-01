CREATE TABLE IF NOT EXISTS files (
    id serial8 PRIMARY KEY NOT NULL,
    year int2 NOT NULL,
    month int2 NOT NULL,
    day int2 NOT NULL,
    timestamp int8 NOT NULL,
    UNIQUE (year, month, day, timestamp)
);
CREATE TABLE IF NOT EXISTS measurements (
    timestamp timestamp with time zone PRIMARY KEY NOT NULL,
    file int4 NOT NULL,
    health float8 NOT NULL,
    performance float8 NOT NULL,
    tvoc float8,
    humidity float8 NOT NULL,
    humidity_abs float8 NOT NULL,
    temperature float8 NOT NULL,
    dewpt float8 NOT NULL,
    sound float8 NOT NULL,
    pressure float8 NOT NULL,
    no2 float8,
    co float8,
    co2 float8 NOT NULL,
    pm1 float8 NOT NULL,
    pm2_5 float8 NOT NULL,
    pm10 float8 NOT NULL,
    oxygen float8 NOT NULL,
    o3 float8,
    so2 float8,
    FOREIGN KEY (file) REFERENCES files (id)
);
