"""Prevent expensive hosted runners/benchmarks returning to automatic events."""
import json
from pathlib import Path
import re
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]


def workflow(name):
    # Preserve the GitHub key `on`; YAML 1.1 SafeLoader otherwise treats it as bool.
    return yaml.load((ROOT / '.github/workflows' / name).read_text(), Loader=yaml.BaseLoader)


def rust_matrix(event, full_platform_checks):
    expression = workflow('ci.yml')['jobs']['check']['strategy']['matrix']['include']
    match = re.fullmatch(
        r"\$\{\{ fromJSON\(github\.event_name == 'workflow_dispatch' && "
        r"inputs\.full_platform_checks && '(.*)' \|\| '(.*)'\) \}\}", expression)
    if not match:
        raise AssertionError('matrix must gate opt-in runners on dispatch plus typed boolean')
    return json.loads(match.group(1 if event == 'workflow_dispatch' and full_platform_checks else 2))


class WorkflowCostPolicyTests(unittest.TestCase):
    def test_automatic_events_cannot_select_opt_in_runners(self):
        for event in ['push', 'pull_request', 'schedule', 'pull_request_target']:
            for supplied in [False, True]:
                with self.subTest(event=event, supplied=supplied):
                    self.assertEqual(rust_matrix(event, supplied), [{'os': 'ubuntu-latest', 'suffix': ''}])

    def test_manual_rust_checks_default_to_ubuntu_and_require_boolean_opt_in(self):
        inputs = workflow('ci.yml')['on']['workflow_dispatch']['inputs']
        self.assertEqual(inputs['full_platform_checks']['type'], 'boolean')
        self.assertEqual(inputs['full_platform_checks']['default'], 'false')
        self.assertEqual([x['os'] for x in rust_matrix('workflow_dispatch', False)], ['ubuntu-latest'])
        self.assertEqual([x['os'] for x in rust_matrix('workflow_dispatch', True)],
                         ['ubuntu-latest', 'macos-26', 'windows-latest'])

    def test_xcode_and_benchmarks_have_no_automatic_triggers(self):
        for name in ['macos.yml', 'performance.yml']:
            with self.subTest(workflow=name):
                self.assertEqual(set(workflow(name)['on']), {'workflow_dispatch'})
                self.assertNotIn('changes', workflow(name)['jobs'])

    def test_manual_xcode_runs_only_selected_suite_by_default(self):
        config = workflow('macos.yml')
        suite = config['on']['workflow_dispatch']['inputs']['suite']
        self.assertEqual(suite['type'], 'choice')
        self.assertEqual(suite['default'], 'release')
        self.assertEqual(suite['options'], ['release', 'tests', 'performance', 'all'])
        for name in ['tests', 'performance', 'release']:
            self.assertEqual(config['jobs'][name]['if'], f"inputs.suite == '{name}' || inputs.suite == 'all'")

    def test_benchmarks_select_one_runner_not_a_matrix(self):
        config = workflow('performance.yml')
        runner = config['on']['workflow_dispatch']['inputs']['runner']
        self.assertEqual(runner['type'], 'choice')
        self.assertEqual(runner['default'], 'ubuntu-latest')
        self.assertEqual(runner['options'], ['ubuntu-latest', 'macos-26'])
        self.assertEqual(config['jobs']['scanner']['runs-on'], '${{ inputs.runner }}')
        self.assertNotIn('strategy', config['jobs']['scanner'])

    def test_cancel_superseded_rust_runs(self):
        concurrency = workflow('ci.yml')['concurrency']
        self.assertEqual(concurrency['cancel-in-progress'], 'true')
        self.assertIn('github.event.pull_request.number || github.ref', concurrency['group'])

    def test_no_other_automatic_workflows_bypass_cost_policy(self):
        automatic = {'push', 'pull_request', 'pull_request_target', 'schedule'}
        for path in (ROOT / '.github/workflows').glob('*.yml'):
            config = workflow(path.name)
            triggers = config['on']
            if set(triggers) & automatic:
                self.assertEqual(path.name, 'ci.yml', 'new automatic workflow needs cost review')
                for name, job in config['jobs'].items():
                    if 'runs-on' in job:
                        expected = '${{ matrix.os }}' if name == 'check' else 'ubuntu-latest'
                        self.assertEqual(job['runs-on'], expected)


if __name__ == '__main__':
    unittest.main()
