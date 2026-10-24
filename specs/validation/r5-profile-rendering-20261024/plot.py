"""Render saved numerical receipts only. Requires matplotlib; not a CI job."""
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
root=Path(__file__).resolve().parent
fig,ax=plt.subplots(figsize=(10.5,5.5),layout="constrained")
for name,label,color in [("animated-rss","Numeric text interpolation","#cc6633"),("immediate-rss","Immediate numeric labels","#168b79")]:
    data=json.loads((root/(name+".json")).read_text())
    ax.plot([p["elapsed_seconds"] for p in data["samples"]],[p["rss_bytes"]/1048576 for p in data["samples"]],label=label,color=color,lw=2)
ax.set(xlabel="Seconds since each monitor started (includes folder picker)",ylabel="Sampled process RSS (MiB)",
       title="BuildHunter Release GUI: performance popover",xlim=(0,240),ylim=(0,750))
ax.grid(alpha=.2);ax.spines[["top","right"]].set_visible(False);ax.legend(loc="upper left",frameon=False)
fig.text(.5,-.045,"""Separate local runs; popovers opened at different times. Both scans stalled in two native opens.
200 ms nominal sampling; this is not kernel peak RSS, physical footprint or a completed-scan comparison.""",ha="center",fontsize=9)
fig.savefig(root/"gui-rss.png",dpi=170,bbox_inches="tight")
fig.savefig(root/"gui-rss.svg",bbox_inches="tight")
print("matplotlib",matplotlib.__version__)

svg=root/"gui-rss.svg"
svg.write_text("\n".join(line.rstrip() for line in svg.read_text().splitlines())+"\n")
