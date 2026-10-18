#!/usr/bin/env python3
import copy
import base64
import hashlib
import unittest
from stage2_headless_android_smoke import RunningStreamGate, ack_evidence, prompt_evidence, attachment_evidence, resource_evidence, selection_evidence, ATTACHMENT_BYTES, ATTACHMENT_PATH, ATTACHMENT_PROMPT, RESOURCE_NAME, RESOURCE_TEMPLATE, RESOURCE_PROMPT, RESOURCE_EXPANDED
from stage2_headless_android_smoke import concurrent_read_evidence


class ConcurrentReadEvidenceTests(unittest.TestCase):
    def test_exact_phase_and_current_read_count_required(self):
        phase = "INSTRUMENTATION_STATUS: zenpi_phase=headless_concurrent_reads_verified\n"
        count = "INSTRUMENTATION_STATUS: headless_concurrent_read_count=3\n"
        self.assertTrue(concurrent_read_evidence(count + phase))
        for output in (None, "", phase, count, phase + count + count, phase + phase + count,
                       phase + count.replace("=3", "=4"), phase + count.replace("=3", "=30"),
                       "summary: " + phase + count, phase + "summary: " + count):
            self.assertFalse(concurrent_read_evidence(output))


class RunningStreamGateTests(unittest.TestCase):
    def test_only_ordered_android_display_and_reattach_release_provider(self):
        gate = RunningStreamGate()
        self.assertFalse(gate.release.is_set())
        gate.record("provider_preview_flushed")
        gate.record(gate.phases[0])
        self.assertFalse(gate.release.is_set())
        self.assertFalse(gate.evidence()["verified"])
        gate.record(gate.phases[1])
        self.assertFalse(gate.release.is_set())
        gate.record(gate.phases[2])
        self.assertFalse(gate.release.is_set())
        gate.record(gate.phases[3])
        self.assertTrue(gate.release.is_set())
        self.assertFalse(gate.evidence()["verified"])
        gate.record("provider_completed_flushed")
        self.assertTrue(gate.evidence()["verified"])

    def test_out_of_order_duplicate_or_missing_provider_proof_fails(self):
        for events in (("headless_running_stream_verified",),
            ("provider_preview_flushed", "provider_completed_flushed"),
            ("provider_preview_flushed", "headless_running_stream_resumed_verified"),
            ("provider_preview_flushed", "headless_running_stream_verified", "headless_background_writes_fenced_verified", "headless_running_stream_resumed_verified"),
            ("provider_preview_flushed", "headless_running_stream_verified", "headless_running_stream_verified")):
            gate = RunningStreamGate()
            with self.assertRaises(RuntimeError):
                for event in events: gate.record(event)
            self.assertFalse(gate.release.is_set())
            self.assertFalse(gate.evidence()["verified"])

    def test_post_completion_duplicate_invalidates_proof(self):
        gate = RunningStreamGate()
        for event in ("provider_preview_flushed", *gate.phases, "provider_completed_flushed"): gate.record(event)
        with self.assertRaises(RuntimeError): gate.record(gate.phases[-1])
        self.assertFalse(gate.evidence()["verified"])

    def test_evidence_is_a_copy_and_shutdown_is_not_a_receipt(self):
        gate = RunningStreamGate()
        gate.evidence()["events"].extend(["provider_preview_flushed", *gate.phases, "provider_completed_flushed"])
        gate.release.set()
        self.assertFalse(gate.evidence()["verified"])


class EvidenceTests(unittest.TestCase):
    def fixture(self):
        requests, records, bindings = [], [], []
        for index, marker in enumerate(("STAGE2_HEADLESS_ANDROID_FIRST", "STAGE2_HEADLESS_ANDROID_SECOND")):
            request_id = f"prompt-{index}"
            fingerprint = str(index) * 64
            requests.append({"model":"gpt-5.2" if index == 0 else "gpt-5.2-2025-12-11", "reasoning":{"effort":"high" if index == 0 else "low"}, "input":[{"role":"user", "content":marker}]})
            if index == 0:
                requests[-1]["input"][-1]["content"] = [{"type":"input_text", "text":ATTACHMENT_PROMPT},
                    {"type":"input_file", "filename":"fixture note.txt", "file_data":"data:text/plain;base64," + base64.b64encode(ATTACHMENT_BYTES).decode()}]
            records.append({"event":{"type":"remote_prompt_admission_v1", "state":"accepted", "device_id":"device", "request_id":request_id,
                "fingerprint":fingerprint, "result":{"project_id":"project", "session_id":"session"}}})
            for state in ("running", "succeeded"):
                records.append({"event":{"type":"remote_prompt_progress_v1", "device_id":"device", "project_id":"project", "session_id":"session", "request_id":request_id, "state":state}})
            records.append({"kind":"turn", "turn":{"role":"assistant", "content":marker + "_DONE"}})
            bindings.append({"device_id":"device", "request_id":request_id, "project_id":"project", "session_id":"session", "fingerprint":fingerprint})
        selections = []
        for index, operation, field, value, effort in ((0,"model_select","model","gpt-5.2-2025-12-11","high"), (1,"reasoning_select","reasoning_effort","low","low")):
            request_id = "selection-" + str(index); fingerprint = str(index + 2) * 64; revision = 10 + index * 3
            kind = "remote_" + ("model" if index == 0 else "reasoning") + "_admission_v1"
            common = {"type":kind,"request_id":request_id,"device_id":"device","fingerprint":fingerprint,"code":None}
            selections.extend([{"kind":"event","seq":revision-1,"event":common | {"state":"reserved","result":None}},
                {"kind":"event","seq":revision,"event":{"type":"model_selected","model":"gpt-5.2-2025-12-11","reasoning_effort":effort}},
                {"kind":"event","seq":revision+1,"event":common | {"state":"succeeded","result":{"request_id":request_id,"project_id":"project","session_id":"session",field:value,"selection_revision":revision}}}])
            bindings.append({"operation":operation,"device_id":"device","request_id":request_id,"fingerprint":fingerprint,"project_id":"project","session_id":"session"})
        records[4:4] = selections
        records.append({"kind":"turn", "turn":{"role":"user", "content":ATTACHMENT_PROMPT, "metadata":{"attachments":[{
            "kind":"file", "mime_type":"text/plain", "path":ATTACHMENT_PATH, "url":None, "file_id":None,
            "size_bytes":len(ATTACHMENT_BYTES), "sha256":hashlib.sha256(ATTACHMENT_BYTES).hexdigest()}]}}})
        requests[1]["input"][-1]["content"] = RESOURCE_EXPANDED
        records.append({"kind":"turn", "turn":{"role":"user", "content":RESOURCE_PROMPT, "metadata":{"resource":{
            "kind":"template", "generation":1, "name":RESOURCE_NAME, "source":"/fixture/.zenpi/prompts/" + RESOURCE_NAME + ".md",
            "source_hash":hashlib.sha256(RESOURCE_TEMPLATE.encode()).hexdigest()}}}})
        return requests, records, {"remote_workspace":{"prompt_bindings":bindings}}

    def verify(self, value):
        return prompt_evidence(*value, "project", "device", "prompt-1")

    def test_complete_chain_passes(self):
        self.assertTrue(self.verify(self.fixture()))

    def test_duplicate_provider_call_fails(self):
        value = self.fixture(); value[0].append(copy.deepcopy(value[0][1]))
        self.assertFalse(self.verify(value))

    def test_repeated_admission_or_terminal_fails(self):
        for index in (0, 2):
            value = self.fixture(); value[1].append(copy.deepcopy(value[1][index]))
            self.assertFalse(self.verify(value))

    def test_wrong_owner_session_or_fingerprint_fails(self):
        for field in ("device_id", "session_id", "project_id", "fingerprint"):
            value = self.fixture(); value[2]["remote_workspace"]["prompt_bindings"][0][field] = "different"
            self.assertFalse(self.verify(value))

    def test_missing_result_or_wrong_last_user_fails(self):
        value = self.fixture(); value[1].pop(13)
        self.assertFalse(self.verify(value))
        value = self.fixture(); value[0][1]["input"].append({"role":"user", "content":"different"})
        self.assertFalse(self.verify(value))

    def test_progress_cannot_belong_to_another_owner_or_session(self):
        for field in ("device_id", "project_id", "session_id"):
            value = self.fixture(); value[1][1]["event"][field] = "foreign"
            self.assertFalse(self.verify(value))

    def test_attachment_missing_changed_duplicate_or_retained_into_second_turn_fails(self):
        self.assertTrue(attachment_evidence(*self.fixture()[:2]))
        for change in (
            lambda requests, records: requests[0]["input"][-1]["content"].pop(),
            lambda requests, records: requests[0]["input"][-1]["content"][1].update(file_data="data:text/plain;base64,"),
            lambda requests, records: requests[0]["input"][-1]["content"][1].update(filename="wrong.txt"),
            lambda requests, records: requests[0]["input"][-1]["content"].append(copy.deepcopy(requests[0]["input"][-1]["content"][1])),
            lambda requests, records: requests[1]["input"][-1].update(content=copy.deepcopy(requests[0]["input"][-1]["content"])),
            lambda requests, records: records[-2]["turn"]["metadata"]["attachments"][0].update(path="foreign.txt"),
            lambda requests, records: records[-2]["turn"]["metadata"]["attachments"][0].update(sha256="0" * 64),
            lambda requests, records: records[-2]["turn"]["metadata"].update(file_data=ATTACHMENT_BYTES.decode()),
            lambda requests, records: records[3]["turn"].update(metadata=copy.deepcopy(records[-2]["turn"]["metadata"])),
            lambda requests, records: records[-2]["turn"]["metadata"].update(raw_base64=base64.b64encode(ATTACHMENT_BYTES).decode()),
            lambda requests, records: records.append(copy.deepcopy(records[-2])),
        ):
            value = self.fixture(); change(value[0], value[1])
            self.assertFalse(self.verify(value))

    def test_template_requires_expanded_provider_input_original_journal_and_exact_provenance(self):
        self.assertTrue(resource_evidence(*self.fixture()[:2]))
        for change in (
            lambda requests, records: requests[1]["input"][-1].update(content=RESOURCE_PROMPT),
            lambda requests, records: requests[1]["input"][-1].update(content=RESOURCE_EXPANDED + " changed"),
            lambda requests, records: requests[0]["input"].append({"role":"user", "content":RESOURCE_EXPANDED}),
            lambda requests, records: records[-1]["turn"].update(content=RESOURCE_EXPANDED),
            lambda requests, records: records[-1]["turn"]["metadata"]["resource"].update(kind="skill"),
            lambda requests, records: records[-1]["turn"]["metadata"]["resource"].update(name="foreign"),
            lambda requests, records: records[-1]["turn"]["metadata"]["resource"].update(generation=True),
            lambda requests, records: records[-1]["turn"]["metadata"]["resource"].update(source="relative/" + RESOURCE_NAME + ".md"),
            lambda requests, records: records[-1]["turn"]["metadata"]["resource"].update(source_hash="0" * 64),
            lambda requests, records: records[-1]["turn"]["metadata"]["resource"].update(body="STAGE2_HOST_TEMPLATE_BODY"),
            lambda requests, records: records[3]["turn"].update(metadata=copy.deepcopy(records[-1]["turn"]["metadata"])),
            lambda requests, records: records.append(copy.deepcopy(records[-1])),
            lambda requests, records: requests[1].update(instructions="STAGE2_FILLER_BODY"),
        ):
            value = self.fixture(); change(value[0], value[1])
            self.assertFalse(self.verify(value))

    def test_selection_requires_actual_provider_values_durable_matching_receipts_and_unique_binding(self):
        self.assertTrue(selection_evidence(*self.fixture(),"project","device"))
        for change in (
            lambda value: value[0][0].update(model="gpt-4.1"),
            lambda value: value[0][1].update(model="gpt-5.2"),
            lambda value: value[0][1].update(reasoning={"effort":"high"}),
            lambda value: value[1][5]["event"].update(reasoning_effort="low"),
            lambda value: value[1][5].update(seq=True),
            lambda value: value[1][4].update(seq=8),
            lambda value: value[1][6].update(seq="11"),
            lambda value: value[1][5].update(sequence=value[1][5].pop("seq")),
            lambda value: value[1][4].update(kind="turn"),
            lambda value: value[1][6]["event"]["result"].update(selection_revision=11),
            lambda value: value[1][6]["event"]["result"].update(project_id="foreign"),
            lambda value: value[1][6]["event"]["result"].update(session_id="foreign"),
            lambda value: value[1][6]["event"].update(fingerprint="bad"),
            lambda value: value[1][6]["event"].update(state="unknown_outcome"),
            lambda value: value[1][6]["event"].update(result=None),
            lambda value: value[1][6]["event"].update(device_id="foreign"),
            lambda value: value[1][4]["event"].update(result={"selection":"premature"}),
            lambda value: value[1][8]["event"].update(reasoning_effort="none"),
            lambda value: value[1][6]["event"]["result"].update(extra="private"),
            lambda value: value[2]["remote_workspace"]["prompt_bindings"][2].update(operation="prompt"),
            lambda value: value[2]["remote_workspace"]["prompt_bindings"].append(copy.deepcopy(value[2]["remote_workspace"]["prompt_bindings"][2])),
            lambda value: value[1].append(copy.deepcopy(value[1][5])),
            lambda value: value[1].append(copy.deepcopy(value[1][6])),
            lambda value: value[1].__setitem__(slice(4,7),list(reversed(value[1][4:7]))),
            lambda value: value[1].append(value[1].pop(0)),
            lambda value: value[1].insert(4,value[1].pop(10)),
            lambda value: value[1].append(value[1].pop(2)),
        ):
            value=self.fixture();change(value)
            self.assertFalse(self.verify(value))


class AckEvidenceTests(unittest.TestCase):
    def fixture(self):
        _, records, checkpoint = EvidenceTests().fixture()
        checkpoint["remote_workspace"]["event_acks"] = [{"device_id":"device", "project_id":"project",
            "cursor":{"session_id":"session", "next_sequence":4}}]
        return records, checkpoint

    def verify(self, value):
        return ack_evidence(*value, "project", "device")

    def test_exact_durable_checkpoint_passes(self):
        self.assertTrue(self.verify(self.fixture()))

    def test_missing_duplicate_or_malformed_ack_fails(self):
        for acks in ([], [None], [{"cursor":None}], [{}, {}]):
            value = self.fixture(); value[1]["remote_workspace"]["event_acks"] = acks
            self.assertFalse(self.verify(value))
        value = self.fixture()
        value[1]["remote_workspace"]["event_acks"] *= 2
        self.assertFalse(self.verify(value))

    def test_wrong_device_project_or_session_fails(self):
        for field in ("device_id", "project_id", "session_id"):
            value = self.fixture(); ack = value[1]["remote_workspace"]["event_acks"][0]
            (ack["cursor"] if field == "session_id" else ack)[field] = "foreign"
            self.assertFalse(self.verify(value))

    def test_cursor_must_equal_all_durable_turns_and_be_an_integer(self):
        for next_sequence in (0, 1, 2, 3, 5, -1, "4", 4.0, True, None):
            value = self.fixture()
            value[1]["remote_workspace"]["event_acks"][0]["cursor"]["next_sequence"] = next_sequence
            self.assertFalse(self.verify(value))

    def test_ambiguous_missing_session_or_no_turns_fails(self):
        value = self.fixture(); value[0][10]["event"]["result"]["session_id"] = "foreign"
        self.assertFalse(self.verify(value))
        value = self.fixture(); value[0][0]["event"]["result"].pop("session_id")
        self.assertFalse(self.verify(value))
        value = self.fixture(); value[0][:] = [record for record in value[0] if record.get("kind") != "turn"]
        value[1]["remote_workspace"]["event_acks"][0]["cursor"]["next_sequence"] = 0
        self.assertFalse(self.verify(value))

    def test_extra_checkpoint_ack_fields_fail(self):
        for nested in (False, True):
            value = self.fixture(); ack = value[1]["remote_workspace"]["event_acks"][0]
            (ack["cursor"] if nested else ack)["extra"] = "unexpected"
            self.assertFalse(self.verify(value))


if __name__ == "__main__":
    unittest.main()
