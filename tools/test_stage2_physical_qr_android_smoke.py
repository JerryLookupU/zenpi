import subprocess
import unittest
import copy
import json
import tempfile
import base64
import hashlib
from pathlib import Path
from unittest.mock import patch
from stage2_physical_qr_android_smoke import CLASSES, PHASES, HARNESS_INPUTS, ROOT, PHONE_PROMPT, DESKTOP_ANSWER, LOCAL_PROMPT, LOCAL_ANSWER, BACKGROUND_PROMPT, BACKGROUND_ANSWER, digest, public_command, validate_receipt, require_physical_device, require_fresh_install, require_new_evidence, require_qr_invite, require_grant, require_prompt_evidence, provider_prompt, require_result
from stage2_instrumentation import expected_tests
from stage2_physical_qr_android_smoke import foreground_replay_evidence, require_revoke_evidence


class PhysicalQrTests(unittest.TestCase):
    def prompt(self):
        binding = {"device_id":"c" * 64, "project_id":"b" * 64, "session_id":"session", "request_id":"request", "fingerprint":"f" * 64}
        return {"session_id":"session", "provider_requests":3, "provider_prompt":PHONE_PROMPT, "provider_answer":DESKTOP_ANSWER,
            "provider_prompts":[LOCAL_PROMPT, BACKGROUND_PROMPT, PHONE_PROMPT], "provider_answers":[LOCAL_ANSWER, BACKGROUND_ANSWER, DESKTOP_ANSWER],
            "original_prefix_sha256":"d" * 64, "preserved_prefix_sha256":"d" * 64,
            "appended_turns":[{"id":"local-user", "role":"user", "content":LOCAL_PROMPT},
                {"id":"local-assistant", "role":"assistant", "content":LOCAL_ANSWER},
                {"id":"background-user", "role":"user", "content":BACKGROUND_PROMPT},
                {"id":"background-assistant", "role":"assistant", "content":BACKGROUND_ANSWER},
                {"id":"user", "role":"user", "content":PHONE_PROMPT}, {"id":"assistant", "role":"assistant", "content":DESKTOP_ANSWER}],
            "bindings":[binding], "admissions":[{"device_id":binding["device_id"], "request_id":binding["request_id"],
                "fingerprint":binding["fingerprint"], "type":"remote_prompt_admission_v1", "state":"accepted",
                "result":{field:binding[field] for field in ("request_id", "project_id", "session_id")}}],
            "progress":[{**{field:binding[field] for field in ("device_id", "request_id", "project_id", "session_id")},
                "type":"remote_prompt_progress_v1", "state":state} for state in ("running", "succeeded")]}

    def test_two_way_sync_requires_ordered_provider_requests_preserved_journal_and_bound_durable_turns(self):
        value = self.prompt(); require_prompt_evidence(value, "c" * 64, "b" * 64)
        for field, wrong in (("provider_requests",0), ("provider_requests",1), ("provider_requests",2), ("provider_requests",4), ("provider_requests",True),
                ("provider_prompts",[PHONE_PROMPT, LOCAL_PROMPT]), ("provider_answers",[DESKTOP_ANSWER, LOCAL_ANSWER]),
                ("provider_prompt","foreign"), ("provider_answer","foreign"), ("preserved_prefix_sha256","e" * 64),
                ("bindings",[]), ("admissions",[]), ("appended_turns",[]), ("session_id","other"), ("progress",[])):
            bad = copy.deepcopy(value); bad[field] = wrong
            with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)
        for field in ("device_id", "fingerprint", "request_id"):
            bad = copy.deepcopy(value); bad["admissions"][0][field] = "foreign"
            with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)
        for field in ("project_id", "session_id", "request_id"):
            bad = copy.deepcopy(value); bad["admissions"][0]["result"][field] = "foreign"
            with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)
        for field in ("device_id", "project_id", "session_id", "request_id", "type", "state"):
            bad = copy.deepcopy(value); bad["progress"][1][field] = "foreign"
            with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)
        for field in ("bindings", "admissions", "appended_turns"):
            bad = copy.deepcopy(value); bad[field] = [None]
            with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)
        bad = copy.deepcopy(value); bad["appended_turns"].reverse()
        with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)
        bad = copy.deepcopy(value); bad["appended_turns"] = bad["appended_turns"][2:]
        with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)
        bad = copy.deepcopy(value); bad["appended_turns"][2]["id"] = bad["appended_turns"][0]["id"]
        with self.assertRaises(ValueError): require_prompt_evidence(bad, "c" * 64, "b" * 64)

    def test_provider_requires_actual_latest_user_text_not_a_marker_in_json(self):
        for content in (PHONE_PROMPT, [{"type":"input_text", "text":PHONE_PROMPT}]):
            self.assertEqual(provider_prompt({"input":[{"role":"user", "content":"old"}, {"role":"user", "content":content}]}), PHONE_PROMPT)
        self.assertEqual(provider_prompt({"input":[{"role":"user", "content":PHONE_PROMPT + "suffix"}]}), PHONE_PROMPT + "suffix")
        self.assertEqual(provider_prompt({"input":[{"role":"user", "content":PHONE_PROMPT}, {"role":"user", "content":"foreign"}]}), "foreign")
        for body in (None, {}, {"input":None}, {"input":[]}, {"input":[None]},
                {"input":[{"role":"assistant", "content":PHONE_PROMPT}]},
                {"input":[{"role":"user", "content":[{"type":"input_image", "text":PHONE_PROMPT}]}]}):
            with self.assertRaises(ValueError): provider_prompt(body)

    def test_control_grant_is_same_device_and_project_after_read_revision(self):
        records = [{"id":"device", "revoked":False, "revision":index + 1,
            "capabilities":["read"] if index < 2 else ["read", "control"],
            "projects":[] if index == 0 else ["project"]} for index in range(3)]
        require_grant(records[:2], "device", "project")
        require_grant(records, "device", "project")
        for field, wrong in (("id","foreign"), ("projects",["foreign"]), ("revoked",True),
                ("revision",2), ("revision",True), ("capabilities",["control"]), ("capabilities",["read", "control", "approval"])):
            bad = copy.deepcopy(records); bad[2][field] = wrong
            with self.assertRaises(ValueError): require_grant(bad, "device", "project")
        for wrong in (records[:1], records + records[:1], None):
            with self.assertRaises(ValueError): require_grant(wrong, "device", "project")

    def test_receipt_and_output_are_not_overwritten_with_any_suffix_or_dangling_link(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ("receipt.json", "receipt.custom", "receipt"):
                path = Path(directory) / name
                require_new_evidence(path)
                for existing in (path, path.with_suffix(".instrumentation.txt")):
                    existing.write_text("preserve")
                    with self.assertRaises(ValueError): require_new_evidence(path)
                    self.assertEqual(existing.read_text(), "preserve")
                    existing.unlink()
                    existing.symlink_to(Path(directory) / "absent")
                    with self.assertRaises(ValueError): require_new_evidence(path)
                    existing.unlink()

    def test_decoded_qr_requires_exact_fresh_invite_lan_endpoint_and_certificate_pin(self):
        certificate = base64.urlsafe_b64encode(b"fixture-certificate").decode().rstrip("=")
        fingerprint = hashlib.sha256(b"fixture-certificate").hexdigest()
        token = base64.urlsafe_b64encode(bytes(range(32))).decode().rstrip("=")
        for ip, address in (("192.168.50.14", "192.168.50.14:7777"), ("fd00::14", "[fd00::14]:7777")):
            invite = f"zenpi://pair?v=1&address={address}&fingerprint={fingerprint}&token={token}&expires=1100"
            values, endpoint = require_qr_invite(invite, ip, certificate, now=1000)
            self.assertEqual(values["token"], token)
            self.assertEqual(endpoint, {"host":ip, "port":7777, "fingerprint":fingerprint})
            for bad in (invite.replace("v=1", "v=2"), invite + "&v=1", invite + "&unknown=value",
                    invite.replace("expires=1100", "expires=1000"), invite.replace("expires=1100", "expires=1301"),
                    invite.replace("7777", "0"), invite.replace(token, "invalid"), invite + "#fragment",
                    invite.replace(fingerprint, "a" * 64)):
                with self.assertRaises(ValueError): require_qr_invite(bad, ip, certificate, now=1000)
            with self.assertRaises(ValueError): require_qr_invite(invite, "192.168.50.99", certificate, now=1000)
            with self.assertRaises(ValueError): require_qr_invite(invite, ip, "not!base64", now=1000)

    def output(self):
        name, method = next(iter(expected_tests(CLASSES)))
        status = f"INSTRUMENTATION_STATUS: class={name}\nINSTRUMENTATION_STATUS: test={method}\nINSTRUMENTATION_STATUS: current=1\nINSTRUMENTATION_STATUS: numtests=1\n"
        return status + "INSTRUMENTATION_STATUS_CODE: 1\n" + "INSTRUMENTATION_STATUS: zenpi_background_event_cursor=3\nINSTRUMENTATION_STATUS: zenpi_recovered_event_cursor=5\nINSTRUMENTATION_STATUS: zenpi_replay_session_id=session\nINSTRUMENTATION_STATUS: zenpi_physical_revoke_request_id=12345678-1234-1234-1234-123456789abc\n" + "\n".join(
            "INSTRUMENTATION_STATUS: zenpi_phase=" + phase for phase in PHASES) + "\n" + status + "INSTRUMENTATION_STATUS_CODE: 0\nOK (1 test)\n"

    def test_saved_evidence_rechecks_identity_cleanup_output_invocation_and_current_builds(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"; output = Path(directory) / "output.txt"
            output.write_text(self.output())
            apk = {"source":"fixture"}; host = {"artifact":{"path":"target/debug/zenpi"}}
            receipt = {"schema_version":"stage2-physical-qr/v7", "passed":True, "failure":None, "whole_item_accepted":False,
                "self_revoke_verified":True, "revoke_record":{"id":"c" * 64, "revoked":True, "revision":4,
                    "capabilities":["read", "control"], "projects":["b" * 64], "revoke_request_id":"12345678-1234-1234-1234-123456789abc"},
                "activity_recreation_resume_verified":True,
                "desktop_live_update_verified":True, "foreground_replay_verified":True,
                "foreground_replay_evidence":foreground_replay_evidence(output.read_text(), "session"),
                "android_prompt_roundtrip_verified":True, "prompt_evidence":self.prompt(), "read_only_journal_sha256":"d" * 64,
                "physical_camera_scan_verified":True, "physical_phone_lan_verified":True, "local_cleanup_verified":True,
                "fixture_cleanup_verified":True, "adb_reverse_used":False, "serial":"phone", "emulator_property":"", "android_api":35,
                "expected_android_api":35, "endpoint":{"host":"192.168.50.14", "port":7777, "fingerprint":"a" * 64}, "project_id":"b" * 64,
                "argv":public_command("192.168.50.14:7777", "a" * 64, "b" * 64), "instrumentation_exit_code":0,
                "instrumentation":{"path":str(output), "sha256":digest(output)}, "phases":list(PHASES),
                "completed_tests":require_result(subprocess.CompletedProcess([], 0, output.read_text()), list(PHASES)),
                "artifacts":{name:digest(ROOT / name) for name in HARNESS_INPUTS}, "device_id":"c" * 64,
                "journal_before_sha256":"d" * 64, "journal_after_sha256":"e" * 64, "seed":{"exit_code":0, "provider_requests":1},
                "grant_records":[{"id":"c" * 64, "revoked":False, "revision":index + 1, "capabilities":["read"] if index < 2 else ["read", "control"],
                    "projects":[] if index == 0 else ["b" * 64]} for index in range(3)],
                "android_build_provenance":apk, "host_build_provenance":host}
            with patch("stage2_physical_qr_android_smoke.require_current_build", return_value=apk), patch("stage2_physical_qr_android_smoke.require_current_host", return_value=host):
                def check(value): path.write_text(json.dumps(value)); return validate_receipt(path)
                self.assertEqual(check(receipt), receipt)
                for key, value in (("passed",False), ("local_cleanup_verified",False), ("fixture_cleanup_verified",False),
                    ("whole_item_accepted",True), ("adb_reverse_used",True), ("instrumentation_exit_code",True),
                    ("journal_after_sha256","d" * 64), ("read_only_journal_sha256","e" * 64), ("android_prompt_roundtrip_verified",False),
                    ("prompt_evidence",{}), ("schema_version","stage2-physical-qr/v4"), ("schema_version","stage2-physical-qr/v5"), ("desktop_live_update_verified",False),
                    ("foreground_replay_verified",False), ("activity_recreation_resume_verified",False),
                    ("self_revoke_verified",False), ("revoke_record",{}), ("schema_version","stage2-physical-qr/v6"),
                    ("foreground_replay_evidence",{}),
                    ("android_build_provenance",{}), ("completed_tests",[]), ("grant_records",[]), ("phases",list(PHASES[:-1]))):
                    bad = copy.deepcopy(receipt); bad[key] = value
                    with self.assertRaises(ValueError): check(bad)
                bad = copy.deepcopy(receipt); bad["argv"] += ["-e", "invite", "private"]
                with self.assertRaises(ValueError): check(bad)
                bad = copy.deepcopy(receipt); bad["artifacts"][HARNESS_INPUTS[0]] = "0" * 64
                with self.assertRaises(ValueError): check(bad)
                output.write_text(self.output() + "zenpi://pair?token=private\n")
                bad = copy.deepcopy(receipt); bad["instrumentation"]["sha256"] = digest(output)
                with self.assertRaises(ValueError): check(bad)
                output.write_text("OK (1 test)\n")
                with self.assertRaises(ValueError): check(receipt)

    def test_only_explicit_physical_device_and_matching_api_are_admitted(self):
        for qemu in ("", "0"):
            require_physical_device("physical-serial", qemu, 35, 35)
        for serial, qemu, api, expected in (("emulator-5554", "0", 35, 35), ("phone", "1", 35, 35),
                ("phone", "unknown", 35, 35), ("phone", "0", 30, 30), ("phone", "0", 35, 34), ("", "0", 35, 35)):
            with self.assertRaises(ValueError): require_physical_device(serial, qemu, api, expected)

    def test_foreground_replay_requires_exact_same_session_and_strictly_advancing_u64_cursors(self):
        output = self.output()
        self.assertEqual(foreground_replay_evidence(output, "session")["recovered_event_cursor"], "5")
        for field, wrongs in (("background_event_cursor", ("-1", "03", "3.0", "18446744073709551616", "5", "6")),
                ("recovered_event_cursor", ("-1", "05", "5.0", "18446744073709551616", "3", "2")),
                ("replay_session_id", ("foreign", ""))):
            original = {"background_event_cursor":"3", "recovered_event_cursor":"5", "replay_session_id":"session"}[field]
            for wrong in wrongs:
                with self.assertRaises(ValueError): foreground_replay_evidence(output.replace(f"zenpi_{field}={original}", f"zenpi_{field}={wrong}"), "session")
            with self.assertRaises(ValueError): foreground_replay_evidence(output + f"INSTRUMENTATION_STATUS: zenpi_{field}={original}\n", "session")
            with self.assertRaises(ValueError): foreground_replay_evidence(output.replace(f"zenpi_{field}=", f"unrelated_{field}="), "session")

    def test_existing_or_uncertain_app_installation_is_never_replaced(self):
        with patch("stage2_physical_qr_android_smoke.subprocess.run", return_value=subprocess.CompletedProcess([], 0, "", "")) as run:
            require_fresh_install(["adb", "-s", "phone"])
            self.assertEqual(run.call_count, 2)
        for code, output, error in ((0, "package:/data/app/old.apk\n", ""), (2, "", ""), (0, "", "error")):
            with patch("stage2_physical_qr_android_smoke.subprocess.run", return_value=subprocess.CompletedProcess([], code, output, error)):
                with self.assertRaises(ValueError): require_fresh_install(["adb", "-s", "phone"])

    def test_named_test_and_all_camera_confirmation_registration_sync_phases_required(self):
        name, method = next(iter(expected_tests(CLASSES)))
        status = f"INSTRUMENTATION_STATUS: class={name}\nINSTRUMENTATION_STATUS: test={method}\nINSTRUMENTATION_STATUS: current=1\nINSTRUMENTATION_STATUS: numtests=1\n"
        output = status + "INSTRUMENTATION_STATUS_CODE: 1\n" + "\n".join(
            "INSTRUMENTATION_STATUS: zenpi_phase=" + phase for phase in PHASES) + "\n" + status + "INSTRUMENTATION_STATUS_CODE: 0\nOK (1 test)\n"
        self.assertEqual(require_result(subprocess.CompletedProcess([], 0, output), list(PHASES)), [f"{name}#{method}"])
        retired = "INSTRUMENTATION_STATUS: zenpi_phase=physical_qr_invitation_retired_verified\n"
        self.assertIn(retired, output)
        old_output = output.replace(retired, "")
        old_phases = [phase for phase in PHASES if phase != "physical_qr_invitation_retired_verified"]
        with self.assertRaises(ValueError): require_result(subprocess.CompletedProcess([], 0, old_output), old_phases)
        for bad in [output.replace(phase, "missing") for phase in PHASES] + [
                output + output, output.replace("CODE: 0", "CODE: -2"), "OK (1 test)\n"]:
            with self.assertRaises(ValueError): require_result(subprocess.CompletedProcess([], 0, bad), list(PHASES))
        with self.assertRaises(ValueError): require_result(subprocess.CompletedProcess([], 1, output), list(PHASES))
        with self.assertRaises(ValueError): require_result(subprocess.CompletedProcess([], 0, output), list(reversed(PHASES)))

    def test_activity_recreation_phase_is_required_once_in_the_actual_sequence(self):
        output = self.output()
        marker = "INSTRUMENTATION_STATUS: zenpi_phase=physical_qr_activity_recreation_resume_verified\n"
        self.assertIn(marker, output)
        late = "INSTRUMENTATION_STATUS: zenpi_phase=physical_qr_desktop_live_update_verified\n"
        reordered = output.replace(marker, "").replace(late, late + marker)
        for bad in (output.replace(marker, ""), output + marker, reordered):
            with self.assertRaises(ValueError): require_result(subprocess.CompletedProcess([], 0, bad), list(PHASES))

    def test_self_revoke_requires_exact_request_and_next_durable_registry_revision(self):
        granted = {"revision":3, "projects":["project"], "capabilities":["read", "control"]}
        record = {**granted, "revision":4, "id":"device", "revoked":True,
            "revoke_request_id":"12345678-1234-1234-1234-123456789abc"}
        output = self.output()
        require_revoke_evidence(record, output, "device", granted)
        for field, wrong in (("id","foreign"), ("revoked",False), ("revision",3), ("revision",5), ("revision",True),
                ("revoke_request_id","foreign"), ("projects",[]), ("capabilities",["read"])):
            with self.subTest(field=field), self.assertRaises(ValueError):
                require_revoke_evidence(record | {field:wrong}, output, "device", granted)
        marker = "INSTRUMENTATION_STATUS: zenpi_physical_revoke_request_id=12345678-1234-1234-1234-123456789abc\n"
        for bad in (output.replace(marker, ""), output + marker, output.replace("12345678-1234-1234-1234-123456789abc", "not-a-request")):
            with self.assertRaises(ValueError): require_revoke_evidence(record, bad, "device", granted)
        phase = "INSTRUMENTATION_STATUS: zenpi_phase=physical_qr_self_revoke_verified\n"
        for bad in (output.replace(phase, ""), output + phase):
            with self.assertRaises(ValueError): require_result(subprocess.CompletedProcess([], 0, bad), list(PHASES))


if __name__ == "__main__": unittest.main()
