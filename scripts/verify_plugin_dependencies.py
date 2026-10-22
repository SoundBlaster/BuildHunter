"""Check active locked Cargo graphs without changing pins or lockfiles."""
import json
from pathlib import Path
import re
import subprocess

from verify_hunter_vendor import verify as verify_vendor

ROOT = Path(__file__).resolve().parents[1]
HUNTER = "https://github.com/SoundBlaster/Hunter.git"
RIFLE = "https://github.com/SoundBlaster/Rifle.git"


def dependency_revision(manifest, package, repository):
    match = re.search(r"^" + re.escape(package) + r' = \{ git = "' + re.escape(repository)
                      + r'", rev = "([0-9a-f]{40})" \}$', manifest.read_text(), re.M)
    if not match:
        raise ValueError(f"Missing exact Git pin for {package}")
    return match[1]


def active_packages(metadata):
    packages = {p["id"]: p for p in metadata["packages"]}
    graph = metadata["resolve"]
    nodes = {node["id"]: node for node in graph["nodes"]}
    pending, seen = [graph["root"]], set()
    while pending:
        package = pending.pop()
        if package in seen:
            continue
        if package not in packages or package not in nodes:
            raise ValueError("Incomplete Cargo resolve graph")
        seen.add(package)
        pending.extend(dep["pkg"] for dep in nodes[package]["deps"])
    return [packages[key] for key in sorted(seen)]


def verify(metadata, root, kind, unix, hunter_revision, rifle_revision):
    packages = active_packages(metadata)
    expected = {
        "hunter-fs-core": int(kind != "policy"),
        "hunter-fs": int(kind == "scanner" and unix),
        "rifle-rules": int(kind != "default"),
    }
    for name, count in expected.items():
        matches = [p for p in packages if p["name"] == name]
        if len(matches) != count:
            raise ValueError(f"{kind}: expected {count} active {name}, got {len(matches)}")
        for package in matches:
            if name == "hunter-fs-core":
                wanted = (root / "vendor/hunter-fs-core/Cargo.toml").resolve()
                if package["source"] is not None or Path(package["manifest_path"]).resolve() != wanted:
                    raise ValueError("Core must be the application's verified vendored package")
            else:
                repository, revision = ((HUNTER, hunter_revision) if name == "hunter-fs"
                                        else (RIFLE, rifle_revision))
                if package["source"] != f"git+{repository}?rev={revision}#{revision}":
                    raise ValueError(f"{name}: unexpected Git source/revision")
    return {p["name"]: {"id": p["id"], "source": p["source"]}
            for p in packages if p["name"] in expected}


def main():
    pin = verify_vendor(ROOT / "vendor/hunter-fs-core")
    scanner = ROOT / "crates/buildhunter-plugin-scanner/Cargo.toml"
    policy = ROOT / "crates/buildhunter-plugin-policy/Cargo.toml"
    if dependency_revision(scanner, "hunter-fs", HUNTER) != pin:
        raise ValueError("Hunter facade and vendored core must pin the same upstream revision")
    rifle = dependency_revision(policy, "rifle-rules", RIFLE)
    host = next(line.removeprefix("host: ") for line in
                subprocess.check_output(["rustc", "-vV"], text=True).splitlines()
                if line.startswith("host: "))
    unix = "unix" in subprocess.check_output(["rustc", "--print", "cfg"], text=True).splitlines()
    result = {}
    for kind, manifest in [("default", ROOT / "Cargo.toml"), ("policy", policy), ("scanner", scanner)]:
        metadata = json.loads(subprocess.check_output([
            "cargo", "metadata", "--locked", "--offline", "--format-version", "1",
            "--filter-platform", host, "--manifest-path", str(manifest)], text=True))
        result[kind] = verify(metadata, ROOT, kind, unix, pin, rifle)
    print(json.dumps({"target": host, "graphs": result}, indent=2))


if __name__ == "__main__":
    main()
