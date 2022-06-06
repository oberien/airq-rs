#!/usr/bin/env python3
import matplotlib.pyplot as plt
import pandas as pd
import json

def render(name, file, columns):
    with open(file) as f:
        data = json.load(f)

    # clean data
    errors = 0
    measuretime = 0
    responsetime = 0
    for m in data:
        if m["errors"] != errors:
            errors = m["errors"]
            m["had_error"] = 1
        else:
            m["had_error"] = 0
        # measuretime is 0 on error, work around that
        if m["measuretime"] == 0:
            m["measuretime"] = measuretime
        else:
            measuretime = m["measuretime"]
        # remove outliers
        if m["response_time"] > 50000:
            m["response_time"] = responsetime
        else:
            responsetime = m["response_time"]

    df = pd.DataFrame(data)
    df["error_pct_moving_average"] = df["had_error"].rolling(window=500).mean() * 100
    df["measuretime_moving_average"] = df["measuretime"].rolling(window=150).mean()
    df["response_time_moving_average"] = df["response_time"].rolling(window=150).mean()
    df = df.loc[:,df.columns.difference(["timestamp", "elapsed", "num", "had_error", "errors"])]

    if "error_pct_moving_average" in columns:
        error_pct = True
        columns.remove("error_pct_moving_average")
    else:
        error_pct = False
    ax = df[columns].plot(title=name, xlabel="Total Requests", ylabel="Time (ms)", color=["blue", "cyan", "orange", "brown"])
    if error_pct:
        ax2 = df[["error_pct_moving_average"]].plot(ax=ax, secondary_y=True, color=["red"])
        ax2.set_ylabel("Error (%)")
        ax2.set_ylim(0, 101)
    plt.savefig(file[13:-5] + ".png", dpi=250)

cols = ["error_pct_moving_average", "response_time", "response_time_moving_average", "measuretime", "measuretime_moving_average"]
render("/data (3 parallel requests)", "measurements-data.json", cols)
cols = ["error_pct_moving_average", "response_time", "response_time_moving_average"];
render("/log (3 parallel requests)", "measurements-log.json", cols.copy())
render("/fetch_recrypt (3 parallel requests)", "measurements-fetch_recrypt.json", cols)
plt.show()
