import tempfile
import unittest
from pathlib import Path

from quality_production_sources import main, production_possible, strip_rust


class ProductionSelectionTests(unittest.TestCase):
    def test_cfg_evaluation_preserves_possible_production(self):
        self.assertFalse(production_possible('all(test, target_os = "macos")'))
        self.assertTrue(production_possible('any(target_os = "macos", all(test, unix))'))
        self.assertTrue(production_possible('not(test)'))

    def test_masks_test_items_and_preserves_following_lines(self):
        source = b'fn before() {}\n#[cfg(test)]\nmod tests { fn test() { let _ = "}"; } }\nfn after() {}\n'
        result, _ = strip_rust(source)
        self.assertEqual(result.count(b'\n'), source.count(b'\n'))
        self.assertEqual(result.splitlines()[3], b'fn after() {}')
        self.assertNotIn(b'mod tests', result)

    def test_masks_namespaced_test_without_removing_next_function(self):
        source = b'#[tokio::test]\nasync fn test() {}\nfn real() {}\n'
        result, _ = strip_rust(source)
        self.assertNotIn(b'async fn test', result)
        self.assertIn(b'fn real()', result)

    def test_external_test_module_is_excluded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / 'project'
            for directory in ('src', 'crates', 'macos/Sources/BuildHunter'):
                (root / directory).mkdir(parents=True)
            (root / 'src/lib.rs').write_text('#[cfg(test)] mod parity;\nfn real() {}\n')
            (root / 'src/parity.rs').write_text('#[test] fn test() {}\n')
            main(root, Path(tmp) / 'out')
            self.assertFalse((Path(tmp) / 'out/src/parity.rs').exists())
            self.assertIn('fn real()', (Path(tmp) / 'out/src/lib.rs').read_text())


if __name__ == '__main__':
    unittest.main()
