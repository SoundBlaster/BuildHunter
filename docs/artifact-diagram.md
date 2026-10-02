# Live artifact diagram

Open **Diagram** in a scan window, or use **Show Artifact Diagram** (⇧⌘D).
Each scan window has one companion diagram. Opening it again brings that window
forward. Both windows share the same in-memory report, target, and scan status.

The animated Swift Charts sunburst groups table rows by relative path. The inner
ring contains folders under the selected target; successive rings show descendants.
Sector area represents the known size of artifact roots, including partial sizes
as lower bounds. Source files and unreported folders do not contribute. Measuring,
unknown, and zero-byte artifacts remain in the counts and folder list without
invented sector area. The footer explains these limits.

Select a sector or a folder in the sidebar to explore it. **Up** and **All artifacts**
return to ancestors. The sidebar supports filtering and shows all immediate children.
To keep dense reports usable, the chart displays at most three levels and seven
sectors per parent: six largest children plus **Other** when needed. Other preserves
the remaining size; select it to show its parent's complete, filterable folder list.
The diagram stops at the artifact roots, just like the table.

Ring thickness depends only on hierarchy depth. Gaps and corner radii shrink for
narrow sectors according to their width at the inner edge, so decoration does not
consume the sector. Angular shares remain proportional to bytes, with no artificial
minimum size. Very small folders can still form narrow slices; choose them in the
sidebar to zoom in and compare their children.

Updates animate with stable path identities and deterministic branch colors.
Reduce Motion disables those animations. Accessible sector descriptions include
path, size, and partial status; the sidebar offers standard buttons for navigation.

## Data and window lifecycle

- `WindowScanModel` remains the only scan owner and event reducer. Opening the
  diagram performs no additional filesystem work and obtains no new permissions.
- While the diagram is open, `ArtifactDiagramModel` coalesces report changes at
  a 100 ms interval. Snapshot aggregation runs outside the main actor. The view
  receives a bounded layout instead of rebuilding the hierarchy per scan event.
- A revision counter avoids rebuilding an unchanged report. A separate report
  identity resets navigation after target replacement/rescan, while Stop retains
  the selected folder. Generation checks reject an obsolete in-flight snapshot.
- Closing the diagram cancels its update task and leaves scanning active. Closing
  the scan window stops its scan; an open diagram retains the final partial report.
  Closing both releases the session. Reports and window targets are not restored.

## Verification

Unit tests cover size conservation, ring boundaries, narrow-sector decoration at
multiple chart sizes, stable identities, large totals,
unknown/partial values, dense reports, navigation, stale scan events, and window
ownership. UI tests exercise one companion window, changing mock states, and closing
the diagram while scanning; they attach completed, scanning, and partial screenshots.
A Release performance test measures snapshot/layout preparation for 10,000 rows.
These checks do not establish animation frame rate or signed sandbox runtime behavior.

The explicit angle ranges and nested-radius construction follow the technique
described in [Building a sunburst diagram in Swift Charts](https://nilcoalescing.com/blog/BuildingASunburstDiagramInSwiftCharts/).
