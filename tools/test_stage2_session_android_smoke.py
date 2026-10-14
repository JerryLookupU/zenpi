#!/usr/bin/env python3
import copy
import sys
import subprocess
import re
from pathlib import Path
import unittest
from unittest.mock import patch
import stage2_session_android_smoke as harness
from test_stage2_instrumentation import output_for


class SessionEvidenceTests(unittest.TestCase):
    def test_access_change_requires_exact_identity_grant_and_successive_revisions(self):
        before = {"id":"device", "name":"phone", "public_key":"public", "revoked":False,
                  "capabilities":["read", "control"], "projects":["project"], "revision":2}
        records = [before, before | {"projects":["session-access-hidden"], "revision":3}, before | {"revision":4}]
        self.assertTrue(harness.access_change_evidence(records,"project"))
        for index in range(3):
            for key, value in (("id","other"), ("public_key","other"), ("name","other"), ("revoked",True),
                ("projects",["foreign"]), ("capabilities",["read"]), ("revision",2), ("revision",2.0)):
                changed = copy.deepcopy(records); changed[index][key] = value
                if changed == records and type(value) is not float: continue
                self.assertFalse(harness.access_change_evidence(changed,"project"), (index,key))
        self.assertFalse(harness.access_change_evidence(records[:2],"project"))
        changed = copy.deepcopy(records); changed[1]["extra"] = True
        self.assertFalse(harness.access_change_evidence(changed,"project"))

    def test_session_suite_contains_catalog_and_matches_current_source_methods(self):
        self.assertIn("zenpi.SessionCatalogScreenTest", harness.INSTRUMENTATION_CLASSES)
        self.assertEqual(len(harness.INSTRUMENTATION_CLASSES), len(set(harness.INSTRUMENTATION_CLASSES)))
        root = Path(__file__).resolve().parents[1]
        count = sum(len(re.findall(r"(?m)^\s*@Test\b", (root / "android/app/src/androidTest/java"
            / (name.replace(".", "/") + ".kt")).read_text())) for name in harness.INSTRUMENTATION_CLASSES)
        self.assertEqual(harness.EXPECTED_INSTRUMENTATION_TESTS, count)

    def fixture(self):
        rename = {"type":"remote_session_name_admission_v1", "device_id":"device", "request_id":"rename",
                  "fingerprint":"a" * 64, "state":"reserved", "result":None}
        old = [{"kind":"session", "session_id":"old"},
               {"kind":"turn", "turn":{"role":"assistant", "content":"STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT"}},
               {"event":copy.deepcopy(rename)}, {"event":{"type":"session_renamed", "name":"Android development"}},
               {"event":rename | {"state":"succeeded", "result":{"request_id":"rename", "project_id":"project", "session_id":"old", "name":"Android development", "journal_revision":6}}}]
        new = [{"kind":"session", "session_id":"new"}, {"event":{"type":"session_new_transition", "request_id":"new-request", "fingerprint":"0"*64,
               "data":{"old_session_id":"old", "new_session_id":"new", "route":"remote"}}}]
        receipts = []
        for i, before, after in ((0,"old","new"),(1,"new","old")):
            request = "new-request" if i == 0 else "resume-request"
            receipts.append({"device_id":"device", "request_id":request, "fingerprint":str(i) * 64,
                "result":{"request_id":request, "project_id":"project", "old_session_id":before, "new_session_id":after,
                          "active_project_id":"project", "required_owner":"c"*64, "workspace_revision":4+i}})
        checkpoint = {"remote_workspace":{"revision":5, "receipts":receipts,
            "prompt_bindings":[{"operation":"session_rename", "device_id":"device", "request_id":"rename", "fingerprint":"a" * 64,
                                "project_id":"project", "session_id":"old"}]}}
        return old, new, checkpoint

    def verify(self, fixture):
        return harness.session_evidence(*fixture, "project", "device")

    def test_full_journal_and_mapping_chain_passes(self):
        self.assertTrue(self.verify(self.fixture()))

    def test_metadata_only_or_duplicate_effect_does_not_pass(self):
        for change in (
            lambda old,new,c: old.pop(1), lambda old,new,c: old.append(copy.deepcopy(old[3])),
            lambda old,new,c: new.append({"kind":"turn"}), lambda old,new,c: new.pop(),
            lambda old,new,c: c["remote_workspace"]["receipts"].append(copy.deepcopy(c["remote_workspace"]["receipts"][0])),
            lambda old,new,c: c["remote_workspace"]["prompt_bindings"].clear()):
            value = self.fixture(); change(*value)
            with self.subTest(change=change): self.assertFalse(self.verify(value))

    def test_wrong_scope_identity_receipt_revision_and_name_fail(self):
        for change in (
            lambda old,new,c: old[4]["event"].update(device_id="other"),
            lambda old,new,c: old[4]["event"]["result"].update(session_id="other"),
            lambda old,new,c: old[4]["event"]["result"].update(name="other"),
            lambda old,new,c: old[4]["event"]["result"].update(request_id="other"),
            lambda old,new,c: old[4]["event"]["result"].update(journal_revision="6"),
            lambda old,new,c: new[1]["event"]["data"].update(route="local"),
            lambda old,new,c: new[1]["event"].update(fingerprint="wrong"),
            lambda old,new,c: c["remote_workspace"]["receipts"][1]["result"].update(new_session_id="other"),
            lambda old,new,c: c["remote_workspace"]["receipts"][1]["result"].update(required_owner="other"),
            lambda old,new,c: c["remote_workspace"]["receipts"][1]["result"].update(workspace_revision=4),
            lambda old,new,c: c["remote_workspace"].update(revision=6),
            lambda old,new,c: c["remote_workspace"]["prompt_bindings"][0].update(fingerprint="other")):
            value = self.fixture(); change(*value)
            with self.subTest(change=change): self.assertFalse(self.verify(value))

    def test_missing_provenance_fails_before_adb_or_any_binary(self):
        with patch.object(sys,"argv",["session-smoke","--serial","fixture","--expected-api","35","--receipt","unused.json"]), \
             patch.object(harness,"require_current_build",side_effect=ValueError("stale APK")), \
             patch.object(harness.subprocess,"run") as run, patch.object(harness.subprocess,"Popen") as popen:
            with self.assertRaisesRegex(ValueError,"stale APK"): harness.main()
            run.assert_not_called(); popen.assert_not_called()

    def test_desktop_local_switch_requires_local_journal_and_preserved_remote_receipts(self):
        old, _, before = self.fixture()
        before["sessions"] = {"project":"/fixture/old.jsonl"}
        after = copy.deepcopy(before)
        after["remote_workspace"]["revision"] += 1
        after["sessions"]["project"] = "/fixture/local.jsonl"
        local = [{"kind":"session","session_id":"local"}, {"event":{"type":"session_new_transition","data":{
            "route":"local","project_id":"project","old_session_id":"old","new_session_id":"local",
            "old_session_path":"/fixture/old.jsonl","new_session_path":"/fixture/local.jsonl"}}}]
        self.assertTrue(harness.local_switch_evidence(old,local,before,after,"project"))
        for change in (
            lambda value: value[1].append({"kind":"turn"}),
            lambda value: value[1][1]["event"]["data"].update(route="remote"),
            lambda value: value[1][1]["event"]["data"].update(project_id="foreign"),
            lambda value: value[1][1]["event"]["data"].update(old_session_id="foreign"),
            lambda value: value[1].append(copy.deepcopy(value[1][1])),
            lambda value: value[3]["remote_workspace"].update(revision=5),
            lambda value: value[3]["remote_workspace"]["receipts"].clear(),
            lambda value: value[3]["remote_workspace"]["prompt_bindings"].clear(),
            lambda value: value[3]["remote_workspace"].update(unexpected=True),
            lambda value: value[3].update(subtabs={"foreign":True}),
            lambda value: value[3]["sessions"].update(foreign="/fixture/foreign.jsonl"),
            lambda value: value[3]["remote_workspace"].update(revision=6.0),
            lambda value: value[0][0].update(kind="event"),
            lambda value: value[3]["sessions"].update(project="/fixture/foreign.jsonl")):
            value = copy.deepcopy((old,local,before,after));change(value)
            self.assertFalse(harness.local_switch_evidence(*value,"project"))

    def test_instrumentation_requires_exact_count_order_and_desktop_callbacks(self):
        output = "\n".join("INSTRUMENTATION_STATUS: zenpi_phase=" + phase for phase in harness.PHASES) + "\n" + output_for(harness.INSTRUMENTATION_CLASSES)
        marker = "INSTRUMENTATION_STATUS: zenpi_session_search_ui=body_snippets_confirmation_offline_verified\n"
        host_marker = "INSTRUMENTATION_STATUS: zenpi_session_body_search=production_host_readonly_resume_verified\n"
        history_ui = "INSTRUMENTATION_STATUS: zenpi_session_history_ui=paging_message_return_offline_verified\n"
        history_host = "INSTRUMENTATION_STATUS: zenpi_session_history_host=production_revision_detail_readonly_verified\n"
        output += marker + host_marker + history_ui + history_host
        harness.require_workflow_result(subprocess.CompletedProcess([],0,output,""),list(harness.PHASES))
        for changed, observed, code in (
            (output.replace(f"OK ({harness.EXPECTED_INSTRUMENTATION_TESTS} tests)","OK (1 test)"),list(harness.PHASES),0),
            (output.replace(f"OK ({harness.EXPECTED_INSTRUMENTATION_TESTS} tests)","OK (7 tests)"),list(harness.PHASES),0),
            (output + "FAILURES",list(harness.PHASES),0),
            (output.replace(marker,""),list(harness.PHASES),0),
            (output + marker,list(harness.PHASES),0),
            (output.replace(host_marker,""),list(harness.PHASES),0),
            (output + host_marker,list(harness.PHASES),0),
            (output.replace(history_ui,""),list(harness.PHASES),0),
            (output + history_ui,list(harness.PHASES),0),
            (output.replace(history_host,""),list(harness.PHASES),0),
            (output + history_host,list(harness.PHASES),0),
            (output,[],0), (output,list(harness.PHASES),1),
            (output.replace(harness.PHASES[1],harness.PHASES[0]),list(harness.PHASES),0),
            (output.replace("INSTRUMENTATION_STATUS: zenpi_phase=" + harness.PHASES[0],""),list(harness.PHASES),0)):
            with self.assertRaises(RuntimeError):
                harness.require_workflow_result(subprocess.CompletedProcess([],code,changed,""),observed)


if __name__ == "__main__": unittest.main()
