#!/usr/bin/env python3
"""Sample one already-running macOS app; never launch or operate its UI.

Select/start the scan and popover separately with native UI tools. This manual
measurement adds no hosted CI job. RSS is sampled, not kernel peak or footprint.
"""
import argparse
import hashlib
import json
import subprocess
import time
from pathlib import Path


def cpu_seconds(value):
    days, separator, clock = value.partition("-")
    if not separator:
        clock, days = days, "0"
    return int(days)*86400 + sum(float(v)*60**i for i,v in enumerate(reversed(clock.split(":"))))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid",type=int,required=True)
    parser.add_argument("--executable",type=Path,required=True)
    parser.add_argument("--seconds",type=float,default=240)
    parser.add_argument("--interval",type=float,default=0.2)
    parser.add_argument("--output",type=Path,required=True)
    args=parser.parse_args()
    if args.pid<=0 or not 0<args.seconds<=3600 or not 0.1<=args.interval<=5:
        parser.error("require positive PID, duration <=3600s and 0.1..5s interval")
    executable=args.executable.resolve(strict=True)
    output=args.output.resolve();output.parent.mkdir(parents=True,exist_ok=True)
    # Exclusive creation preserves an existing receipt.
    with output.open("x") as file:
        samples=[];started=time.monotonic();changed=False;interrupted=False
        digest=hashlib.sha256(executable.read_bytes()).hexdigest()
        try:
            while time.monotonic()-started<args.seconds:
                result=subprocess.run(["ps","-p",str(args.pid),"-o","rss=,time=,command="],capture_output=True,text=True)
                if result.returncode:break
                fields=result.stdout.strip().split(maxsplit=2)
                if len(fields)!=3 or fields[2]!=str(executable):
                    changed=True;break
                samples.append({"elapsed_seconds":time.monotonic()-started,
                    "rss_bytes":int(fields[0])*1024,"cpu_seconds":cpu_seconds(fields[1])})
                time.sleep(args.interval)
        except KeyboardInterrupt:
            interrupted=True
        receipt={"pid":args.pid,"binary_sha256":digest,"sampling_seconds":args.interval,
            "scope":"one process sampled RSS and cumulative CPU via macOS ps; not kernel peak or physical footprint",
            "process_changed":changed,"interrupted":interrupted,"samples":samples,
            "peak_sampled_rss_bytes":max((s["rss_bytes"] for s in samples),default=0)}
        json.dump(receipt,file,indent=2);file.write("\n")
    print(json.dumps({k:v for k,v in receipt.items() if k!="samples"}),flush=True)
    return 0 if samples and not changed else 2


if __name__=="__main__":
    raise SystemExit(main())
