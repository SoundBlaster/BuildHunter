"""Stage production sources while preserving Rust source line numbers."""

from pathlib import Path
import re
import sys

from tree_sitter import Language, Parser
import tree_sitter_rust

PARSER = Parser(Language(tree_sitter_rust.language()))
TOKEN = re.compile(r'\s*(\w+|"(?:\\.|[^"\\])*"|[(),=])')


def production_possible(expression):
    """Evaluate cfg with test=false and other predicates unknown (three values)."""
    tokens = []
    while expression.strip():
        match = TOKEN.match(expression)
        if not match:
            raise ValueError(f"unsupported cfg expression: {expression}")
        tokens.append(match.group(1))
        expression = expression[match.end():]
    position = 0

    def parse():
        nonlocal position
        name = tokens[position]
        position += 1
        if position < len(tokens) and tokens[position] == "(":
            position += 1
            children = []
            while tokens[position] != ")":
                children.append(parse())
                if tokens[position] == ",":
                    position += 1
                elif tokens[position] != ")":
                    raise ValueError("invalid cfg expression")
            position += 1
            if name == "all":
                return False if False in children else (True if all(x is True for x in children) else None)
            if name == "any":
                return True if True in children else (False if all(x is False for x in children) else None)
            if name == "not" and len(children) == 1:
                return None if children[0] is None else not children[0]
            raise ValueError(f"unsupported cfg operator: {name}")
        if position < len(tokens) and tokens[position] == "=":
            position += 2
            return None
        return False if name == "test" else None

    result = parse()
    if position != len(tokens):
        raise ValueError("trailing cfg tokens")
    return result is not False


def test_only(attribute):
    text = attribute.decode("utf-8")
    if re.fullmatch(r"#\s*\[\s*(?:\w+::)*test(?:\s*\([^]]*\))?\s*\]", text):
        return True
    cfg = re.fullmatch(r"#\s*\[\s*cfg\s*\((.*)\)\s*\]", text, re.S)
    if cfg:
        return not production_possible(cfg.group(1))
    if re.search(r"\bcfg_attr\s*\(\s*test\b", text):
        raise ValueError(f"unsupported test-dependent attribute: {text}")
    return False


def strip_rust(source):
    tree = PARSER.parse(source)
    if tree.root_node.has_error:
        raise ValueError("could not parse Rust source")
    spans = []
    external_modules = []

    def visit(node):
        attributes = []
        for child in node.named_children:
            if child.type == "attribute_item":
                attributes.append(child)
                continue
            if child.type in ("line_comment", "block_comment"):
                continue
            if attributes and any(test_only(source[a.start_byte:a.end_byte]) for a in attributes):
                spans.append((attributes[0].start_byte, child.end_byte))
                if child.type == "mod_item" and child.child_by_field_name("body") is None:
                    external_modules.append(source[child.child_by_field_name("name").start_byte:child.child_by_field_name("name").end_byte].decode())
            else:
                visit(child)
            attributes = []

    visit(tree.root_node)
    result = bytearray(source)
    for start, end in spans:
        for index in range(start, end):
            if result[index] not in (10, 13):
                result[index] = 32
    return bytes(result), external_modules


def main(root, destination):
    root, destination = Path(root), Path(destination)
    files = [p for base in ("src", "crates", "macos/Sources/BuildHunter")
             for p in (root / base).rglob("*") if p.is_file()
             and p.suffix in (".rs", ".swift") and "tests" not in p.relative_to(root).parts]
    staged = {}
    excluded = set()
    for path in files:
        content = path.read_bytes()
        if path.suffix == ".rs":
            try:
                content, modules = strip_rust(content)
            except ValueError as error:
                raise ValueError(f"{path}: {error}") from error
            for module in modules:
                excluded.add(path.parent / f"{module}.rs")
                excluded.add(path.parent / module / "mod.rs")
        staged[path] = content
    for path, content in staged.items():
        if path in excluded:
            continue
        target = destination / path.relative_to(root)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)


if __name__ == "__main__":
    main(*sys.argv[1:])
