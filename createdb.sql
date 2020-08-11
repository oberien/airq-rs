CREATE TABLE IF NOT EXISTS files (
    id serial8 PRIMARY KEY NOT NULL,
    year int2 NOT NULL,
    month int2 NOT NULL,
    day int2 NOT NULL,
    timestamp int8 NOT NULL,
    UNIQUE (year, month, day, timestamp)
);
CREATE TABLE IF NOT EXISTS measurements (
    timestamp int8 PRIMARY KEY NOT NULL,
    file int4 NOT NULL,
    health float8,
    performance float8,
    tvoc float8,
    humidity float8,
    humidity_abs float8,
    temperature float8,
    dewpt float8,
    sound float8,
    pressure float8,
    no2 float8,
    co float8,
    co2 float8,
    pm1 float8,
    pm2_5 float8,
    pm10 float8,
    oxygen float8,
    o3 float8,
    so2 float8,
    FOREIGN KEY (file) REFERENCES files (id)
);
