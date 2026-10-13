import tempfile
import unittest
from pathlib import Path

from analyze_allocations import analyze


class AllocationAnalysisTests(unittest.TestCase):
    def analyze_rows(self, rows):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "allocations.xml"
            path.write_text(f"<trace-query-result><node>{rows}</node></trace-query-result>")
            return analyze(path)

    def test_reused_backtrace_keeps_projection_attribution(self):
        result = self.analyze_rows('''
            <row live="true" category="_ContiguousArrayStorage&lt;ScanRow&gt;" size="64">
              <backtrace id="stack1"><frame id="frame1" name="ScanTableProjection.init"/></backtrace>
            </row>
            <row live="true" category="_ContiguousArrayStorage&lt;ScanRow&gt;" size="32">
              <backtrace ref="stack1"/>
            </row>''')
        self.assertEqual(result["live_bytes"], 96)
        self.assertEqual(result["scanrow_stacks"], [
            {"stack": ("ScanTableProjection.init",), "bytes": 96, "count": 2}])

    def test_freed_row_can_define_reused_stack_and_frame(self):
        result = self.analyze_rows('''
            <row live="false" category="Freed" size="999">
              <backtrace id="stack1"><frame id="frame1" name="ScanTableModel.refresh"/></backtrace>
            </row>
            <row live="true" category="_ContiguousArrayStorage&lt;ScanRow&gt;" size="64">
              <backtrace id="stack2"><frame ref="frame1"/></backtrace>
            </row>
            <row live="true" category="_ContiguousArrayStorage&lt;ScanRow&gt;" size="32">
              <backtrace ref="stack2"/>
            </row>''')
        self.assertEqual(result["live_bytes"], 96)
        self.assertEqual(result["scanrow_stacks"][0]["stack"], ("ScanTableModel.refresh",))
        self.assertEqual(result["scanrow_stacks"][0]["count"], 2)

    def test_large_allocations_keep_reused_stack(self):
        result = self.analyze_rows('''
            <row live="false" category="Freed" size="8">
              <backtrace id="stack1"><frame name="Scanner.enqueue"/></backtrace>
            </row>
            <row live="true" category="Malloc" size="33554432">
              <backtrace ref="stack1"/>
            </row>''')
        self.assertEqual(result["large_allocations"][0]["stack"], ["Scanner.enqueue"])


if __name__ == "__main__":
    unittest.main()
