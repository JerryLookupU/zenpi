import copy
import json
from pathlib import Path
import tempfile
import unittest

from generate_stage2_android_schema import CONTRACT, OUTPUT, ROOT, generate, render


class AndroidSchemaTests(unittest.TestCase):
    def setUp(self):
        self.raw = (ROOT / CONTRACT).read_bytes()

    def test_checked_in_source_matches_deterministic_generation(self):
        self.assertEqual(render(self.raw), render(self.raw))
        self.assertEqual((ROOT / OUTPUT).read_bytes(), render(self.raw))
        generate(check=True)

    def test_stale_missing_or_changed_source_refused_without_writing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / CONTRACT
            source.parent.mkdir(parents=True)
            source.write_bytes(self.raw)
            with self.assertRaises(ValueError): generate(root, check=True)
            self.assertFalse((root / OUTPUT).exists())
            generate(root)
            generated = (root / OUTPUT).read_bytes()
            generate(root, check=True)
            source.write_bytes(self.raw + b"\n")
            with self.assertRaises(ValueError): generate(root, check=True)
            self.assertEqual(generated, (root / OUTPUT).read_bytes())
            generate(root)
            generate(root, check=True)
            (root / OUTPUT).write_text("edited output")
            with self.assertRaises(ValueError): generate(root, check=True)
            self.assertEqual("edited output", (root / OUTPUT).read_text())

    def test_invalid_contracts_are_not_rendered_into_kotlin(self):
        original = json.loads(self.raw)
        mutations = [
            lambda value: value.update(schema_version=True),
            lambda value: value.update(schema_version=0),
            lambda value: value.update(schema_version=65536),
            lambda value: value.update(extra=[]),
            lambda value: value.pop("error_codes"),
            lambda value: value.update(read_operations=[]),
            lambda value: value.update(read_operations="inspect"),
            lambda value: value["read_operations"].append("inspect"),
            lambda value: value["write_operations"].append("inspect"),
            lambda value: value["error_codes"].append("bad\"; injected"),
            lambda value: value["execution_states"].append(None),
            lambda value: value.update(error_codes=[f"code_{i}" for i in range(129)]),
            lambda value: value.update(messages=[]),
            lambda value: value.update(messages={}),
            lambda value: value["messages"]["hello"].update(channel="other"),
            lambda value: value["messages"]["hello"].update(fields=["type", "type"]),
            lambda value: value["messages"]["hello"].update(fields=["type", "bad\"name"]),
            lambda value: value["messages"]["hello"].update(fields=["supported_versions"]),
            lambda value: value["messages"]["hello"]["payload"].update(type="other"),
            lambda value: value["messages"]["hello"]["payload"].update(extra=True),
            lambda value: value["messages"]["hello"].update(envelope=[]),
            lambda value: value["messages"]["hello"]["envelope"].update(channel="bulk"),
            lambda value: value["messages"].update({"bad\"name":value["messages"]["hello"]}),
        ]
        for mutate in mutations:
            value = copy.deepcopy(original)
            mutate(value)
            with self.subTest(value=value), self.assertRaises(ValueError): render(json.dumps(value).encode())
        for raw in (b"[]", b"{}", b" " * 65537, self.raw.replace(b'"schema_version": 1', b'"schema_version": 1, "schema_version": 1')):
            with self.assertRaises(ValueError): render(raw)

    def test_source_and_output_symlinks_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / CONTRACT
            source.parent.mkdir(parents=True)
            source.symlink_to(ROOT / CONTRACT)
            with self.assertRaises(ValueError): generate(root)
            source.unlink(); source.write_bytes(self.raw)
            target = root / OUTPUT
            target.parent.mkdir(parents=True)
            target.symlink_to(ROOT / OUTPUT)
            with self.assertRaises(ValueError): generate(root)


if __name__ == "__main__":
    unittest.main()
