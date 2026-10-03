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

Select a sector or a folder in the sidebar to explore it. **Up**, the chart center,
and **All artifacts** return to ancestors. The center does nothing at the target root.
Hovering a sector shows its folder name beside the cursor and temporarily previews
that folder's children in the sidebar. Leaving the chart restores the committed
folder and its filter; hovering never navigates or changes colors. Sidebar rows have
an explicit hover highlight. The heading shows the absolute filesystem path with a
copy button immediately after it; synthetic mock reports have no path to copy.

The chart displays at most three levels. On first display, a dense sibling group
uses six largest children plus **Other**. Already visible children retain their
slots and order during streaming; newly measured folders append into free slots.
If all seven children were already visible before an eighth arrived, retain all
seven plus Other (at most eight sectors per parent). Later large arrivals stay in
Other instead of ejecting a visible folder. On deliberate navigation, the selected
folder's largest child is made visible if it was previously grouped. Other preserves
all remaining bytes; select it to see the complete, filterable folder list.
Sizes and angle widths continue updating, so boundaries can move, but sibling order
and membership do not reshuffle with each measurement. The diagram stops at the
artifact roots, just like the table.

Ring thickness depends only on hierarchy depth. Gaps and corner radii shrink for
narrow sectors according to their width at the inner edge, so decoration does not
consume the sector. Angular shares remain proportional to bytes, with no artificial
minimum size. Very small folders can still form narrow slices; choose them in the
sidebar to zoom in and compare their children.

At each displayed level, immediate child branches receive contrasting colors;
every descendant shares its branch's swatch across all visible rings. Entering a
folder starts a new palette: **only its largest child inherits the color of the
folder selected**, and the other child branches receive contrasting hues. The same
rule repeats at every deeper level. A tie uses the relative path; an unmeasured
folder waits for its first positive child size before choosing the inheritor.

The palette is saved for that folder and entry color. Going Up restores the
previous view; streamed measurements do not transfer its colors to a new size
leader. A deliberate new descent chooses the largest child at that moment and
reuses the saved palette when that inheritor is unchanged. Directly selecting an outer-ring folder carries the color actually clicked, even
if that folder was previously entered through a differently colored parent view.
**Other** stays neutral gray. A new report or a reopened diagram starts fresh.
Color helps orientation, while path labels remain authoritative; very dense reports
have more branches than easily distinguishable hues.

Updates animate with stable path identities.
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
- `ScanTableModel` coalesces a separate, sorted table projection outside the main
  actor. All four column headers support ascending/descending order; Size compares
  numeric byte counts and keeps unknown sizes last in either direction. Path and
  identity break ties, and the scanner's row indices remain untouched. This uses
  SwiftUI's [native table sorting](https://developer.apple.com/documentation/swiftui/table).
- Closing the diagram cancels its update task and leaves scanning active. Closing
  the scan window stops its scan; an open diagram retains the final partial report.
  Closing both releases the session. Reports and window targets are not restored.

## Verification

Unit tests cover size conservation, ring boundaries, narrow-sector decoration at
multiple chart sizes, stable identities, branch colors through navigation and
streaming, sibling branch hue separation, palette reset, large totals,
unknown/partial values, dense reports, navigation, stale scan events, and window
ownership, branch color inheritance, stable streaming membership, hover restoration,
absolute paths, and sorting alongside late measurements. UI tests exercise the
companion window lifecycle, hover preview, center navigation, column-header sorting,
and copying a real selected folder's full path. They attach 14 screenshots.
A Release performance test measures snapshot/layout/palette preparation and table
sorting for 10,000 rows.
These checks do not establish animation frame rate or signed sandbox runtime behavior.

The explicit angle ranges and nested-radius construction follow the technique
described in [Building a sunburst diagram in Swift Charts](https://nilcoalescing.com/blog/BuildingASunburstDiagramInSwiftCharts/).
