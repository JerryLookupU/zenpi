#!/usr/bin/env python3
"""Production terminal QR -> emulator imagefile camera -> pinned TLS transcript.

Owns one dedicated AVD. This is neither a physical-camera nor direct LAN gate.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import struct
import subprocess
import tempfile
import threading
import time
import zlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from stage2_android_build import require_current_build
from stage2_android_pair_smoke import ROOT, seed_session, run_instrumentation, serialized_device_validation, InstrumentationFailure, connect_emulator_wifi
from stage2_desktop_android_smoke import Desktop, terminal_pixels, require_no_reverse
from stage2_host_build import require_current_host
from stage2_host_smoke import validate_invite
from stage2_instrumentation import require_test_results
from tui_user_shell_smoke import drain
from tui_project_workspace_smoke import screen_text
from stage2_physical_qr_android_smoke import (provider_prompt, require_prompt_evidence, foreground_replay_evidence,
    LOCAL_PROMPT, LOCAL_ANSWER, BACKGROUND_PROMPT, BACKGROUND_ANSWER, PHONE_PROMPT, DESKTOP_ANSWER)

CLASSES = ("zenpi.EmulatorQrHostTest",)
PHASES = ("emulator_qr_production_camera_requested", "emulator_qr_camera_confirmation_verified",
          "ungranted_projection_verified", "emulator_qr_authenticated_desktop_transcript_verified",
          "emulator_qr_local_cleanup_verified")
SYNC_PHASES = ("desktop_update_requested","emulator_qr_desktop_live_update_verified","background_update_requested",
               "emulator_qr_background_cache_fenced_verified","emulator_qr_foreground_replay_verified",
               "emulator_qr_android_prompt_answer_verified")
DOZE_PHASES = ("emulator_qr_deep_idle_verified","emulator_qr_doze_cache_fenced_verified",
               "emulator_qr_doze_foreground_resume_verified")


def camera_png(pgm):
    header, width_height, depth, pixels = pgm.split(b"\n", 3)
    width, height = map(int, width_height.split())
    if header != b"P5" or depth != b"255" or len(pixels) != width * height:
        raise ValueError("Invalid terminal pixel dimensions")
    if not 1<=width<=4096 or not 1<=height<=4096: raise ValueError("Terminal image exceeds bounds")
    black = [index for index, value in enumerate(pixels) if value == 0]
    if not black or any(value not in (0,255) for value in pixels):
        raise ValueError("Missing binary terminal QR pixels")
    left, right = min(index % width for index in black), max(index % width for index in black)
    top, bottom = min(index // width for index in black), max(index // width for index in black)
    qr_width, qr_height = right - left + 1, bottom - top + 1
    if qr_width != qr_height or qr_width > 440:
        raise ValueError("Terminal QR geometry exceeds camera bounds")
    # The imagefile backend's observed landscape viewport crops the sensor center.
    # Its rotated image coordinates require an offset, with a complete quiet zone.
    camera_width, camera_height = 1280, 960
    dx, dy = 340 - qr_width // 2, (camera_height - qr_height) // 2
    if dx<16 or dx+qr_width>camera_width-16 or dy<16 or dy+qr_height>camera_height-16:
        raise ValueError("QR does not fit camera quiet zone")
    rows = [bytearray(b"\xff" * camera_width) for _ in range(camera_height)]
    for y in range(qr_height):
        rows[dy+y][dx:dx+qr_width] = pixels[(top+y)*width+left:(top+y)*width+right+1]
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind+data))
    data = b"".join(b"\0"+row for row in rows)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB",camera_width,camera_height,8,0,0,0,0)) \
        + chunk(b"IDAT",zlib.compress(data)) + chunk(b"IEND",b"")


def public_command(address, fingerprint, project, safety_code, bidirectional=False, with_doze=False):
    if not re.fullmatch(r"[0-9]{6}", safety_code): raise ValueError("Invalid emulator safety code")
    if with_doze and not bidirectional: raise ValueError("Doze requires bidirectional workflow")
    return ["am","instrument","-w","-r","-e","class",CLASSES[0],"-e","emulator_camera_run","true",
            "-e","expected_address",address,"-e","expected_fingerprint",fingerprint,"-e","expected_project",project,
            "-e","safety_code",safety_code,
            *(["-e","bidirectional","true"] if bidirectional else []),
            *(["-e","with_doze","true"] if with_doze else []),
            "app.zenpi.android.test/androidx.test.runner.AndroidJUnitRunner"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--avd", required=True)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--port",type=int,default=5584,choices=range(5554,5683,2))
    parser.add_argument("--expected-api",type=int,required=True)
    parser.add_argument("--receipt",type=Path,required=True)
    parser.add_argument("--emulator-host-network",action="store_true",
                        help="Use the emulator's host alias without adb reverse; not physical LAN evidence")
    parser.add_argument("--bidirectional",action="store_true")
    parser.add_argument("--with-doze",action="store_true")
    args = parser.parse_args()
    if args.with_doze and not args.bidirectional: parser.error("--with-doze requires --bidirectional")
    if args.avd not in ("ZenpiStage2Api31","ZenpiStage2Api34","ZenpiStage2Api35"):
        parser.error("A dedicated Stage 2 AVD is required")
    if args.receipt.exists(): raise ValueError("Refusing to overwrite emulator QR evidence")
    provenance = require_current_build()
    binary = ROOT / "target/debug/zenpi"
    host_provenance = require_current_host(binary)
    sdk = Path(os.environ.get("ANDROID_HOME",Path.home()/"Library/Android/sdk"))
    emulator = sdk / "emulator/emulator"
    serial = f"emulator-{args.port}"
    if args.serial != serial: parser.error("--serial must match the owned emulator port")
    adb = [str(sdk/"platform-tools/adb"),"-s",serial]
    devices = subprocess.run([adb[0],"devices"],check=True,capture_output=True,text=True,timeout=10).stdout
    if re.search(rf"(?m)^{re.escape(serial)}\s",devices): raise ValueError("Requested emulator already exists; it will not be replaced")
    started = time.time()
    passed = False; failure = None; output = ""; result = None; api = None
    terminal = process = None; reverse = None; endpoint = None; screenshot = None
    server = provider_thread = None; provider_requests = []; provider_answers = []; device_id = None
    sync_evidence = replay_evidence = None
    callback_steps = []; journal_turns = []
    stage = "seed"; prerequisite_diagnostics = None
    wifi_setup = None
    camera_hash = None
    inputs = ("tools/stage2_emulator_qr_smoke.py","tools/stage2_desktop_android_smoke.py","tools/stage2_android_pair_smoke.py",
              "tools/stage2_instrumentation.py","tools/decode_terminal_qr.swift","tools/stage2_host_smoke.py",
              "tools/stage2_physical_qr_android_smoke.py")
    harness_hashes = {path:hashlib.sha256((ROOT/path).read_bytes()).hexdigest() for path in inputs}
    build_hashes = {path:hashlib.sha256((ROOT/path).read_bytes()).hexdigest() for path in provenance["artifacts"]}
    with tempfile.TemporaryDirectory(prefix="zenpi-emulator-qr-") as raw:
        root = Path(raw)
        try:
            workspace, session, env, project, _, seed = seed_session(binary, root)
            if args.bidirectional:
                class Handler(BaseHTTPRequestHandler):
                    def log_message(self,*args): pass
                    def do_POST(self):
                        try:
                            length = int(self.headers.get("Content-Length","0"))
                            if not 0<length<=1024*1024: raise ValueError("Invalid body size")
                            body = json.loads(self.rfile.read(length))
                            prompt = provider_prompt(body)
                            answer = {LOCAL_PROMPT:LOCAL_ANSWER,BACKGROUND_PROMPT:BACKGROUND_ANSWER,PHONE_PROMPT:DESKTOP_ANSWER}[prompt]
                        except (ValueError,KeyError,TypeError): self.send_error(400);return
                        provider_requests.append(body);provider_answers.append(answer)
                        self.send_response(200);self.send_header("Content-Type","text/event-stream");self.end_headers()
                        for event in ({"type":"response.output_text.delta","delta":answer},
                                      {"type":"response.completed","response":{"id":"emulator-sync","status":"completed","model":body["model"]}}):
                            self.wfile.write(("data: "+json.dumps(event)+"\n\n").encode());self.wfile.flush()
                server = ThreadingHTTPServer(("127.0.0.1",0),Handler)
                provider_thread = threading.Thread(target=server.serve_forever,daemon=True);provider_thread.start()
                (root/"config/config.toml").write_text('backend="openai"\nmodel="gpt-4.1"\n'
                    f'base_url="http://127.0.0.1:{server.server_port}/v1"\nwire_api="responses"\nrequires_openai_auth=false\nmax_retries=0\n')
            env["TERM"] = "xterm-256color"
            stage = "desktop_start"
            terminal = Desktop(binary,workspace,session,root/"identity.json",env,duration=300,
                bind_ip="0.0.0.0" if args.emulator_host_network else "127.0.0.1",
                advertise="10.0.2.2" if args.emulator_host_network else None)
            pgm = root/"terminal.pgm"; pgm.write_bytes(terminal_pixels(bytes(terminal.output)))
            invite = subprocess.run(["swift",str(ROOT/"tools/decode_terminal_qr.swift"),str(pgm)],
                capture_output=True,text=True,check=True,timeout=60).stdout.strip()
            values = validate_invite(invite)
            certificate = json.loads((root/"identity.json").read_text())["certificate"]
            der = base64.b64decode(certificate+"="*(-len(certificate)%4),altchars=b"-_",validate=True)
            if hashlib.sha256(der).hexdigest()!=values["fingerprint"]: raise ValueError("Terminal QR pin mismatch")
            endpoint = {key:values[key] for key in ("address","fingerprint")}
            safety = re.search(rb"Safety code:\s*([0-9]{6})", screen_text(bytes(terminal.output),height=100))
            if safety is None: raise ValueError("Production safety code missing")
            image = root/"camera.png"; image.write_bytes(camera_png(pgm.read_bytes()))
            camera_hash = hashlib.sha256(image.read_bytes()).hexdigest()
            before_bytes = session.read_bytes()
            log = (root/"emulator.log").open("wb")
            stage = "emulator_boot"
            process = subprocess.Popen([str(emulator),"-avd",args.avd,"-port",str(args.port),"-no-window","-no-audio",
                "-no-boot-anim","-no-snapshot-save","-camera-back",f"imagefile:{image}"],stdout=log,stderr=subprocess.STDOUT)
            until = time.monotonic()+90
            while subprocess.run(adb+["shell","getprop","sys.boot_completed"],capture_output=True,text=True,timeout=10).stdout.strip() != "1":
                if process.poll() is not None or time.monotonic()>until: raise ValueError("Owned emulator did not boot")
                drain(terminal.fd,terminal.output,.05); time.sleep(1)
            api = int(subprocess.run(adb+["shell","getprop","ro.build.version.sdk"],check=True,capture_output=True,text=True,timeout=10).stdout)
            if api != args.expected_api: raise ValueError("Android API mismatch")
            stage = "wifi_connect"
            wifi_setup = connect_emulator_wifi(adb)
            stage = "apk_install"
            for path in build_hashes: subprocess.run(adb+["install","-r",str(ROOT/path)],check=True,capture_output=True,timeout=60)
            stage = "camera_permission"
            subprocess.run(adb+["shell","pm","grant","app.zenpi.android","android.permission.CAMERA"],check=True,capture_output=True,timeout=10)
            if args.emulator_host_network:
                require_no_reverse(adb)
                if values["address"].split(":",1)[0]!="10.0.2.2": raise ValueError("Unexpected emulator host alias")
            else:
                reverse = f"tcp:{values['address'].rsplit(':',1)[1]}"
                subprocess.run(adb+["reverse",reverse,reverse],check=True,capture_output=True,timeout=10)
            def grant():
                nonlocal device_id
                identity = root/"identity.devices.json"
                records = json.loads(identity.read_text())["devices"]
                if len(records)!=1 or records[0]["projects"]: raise ValueError("Unexpected device registration")
                device_id = records[0]["id"]
                grant_flag = "--grant-device-control" if args.bidirectional else "--grant-device-read"
                subprocess.run([str(binary),"host","pair","--identity",str(root/"identity.json"),grant_flag,records[0]["id"],"--project",project],
                    cwd=ROOT,env=env,check=True,capture_output=True,timeout=10)
                if args.bidirectional:
                    terminal.write(b"\x1b");terminal.expect(b"Prompt")
                callback_steps.append("read_control_grant_completed" if args.bidirectional else "read_grant_completed")
            def update():
                callback_steps.append("desktop_update_started")
                if session.read_bytes()!=before_bytes: raise ValueError("Read grant modified journal")
                terminal.command(LOCAL_PROMPT,LOCAL_ANSWER.encode());terminal.wait(lambda:LOCAL_ANSWER in session.read_text())
                callback_steps.append("desktop_update_committed")
            def background():
                terminal.command(BACKGROUND_PROMPT,BACKGROUND_ANSWER.encode());terminal.wait(lambda:BACKGROUND_ANSWER in session.read_text())
            command = public_command(values["address"],values["fingerprint"],project,safety[1].decode(),args.bidirectional,args.with_doze)
            stage = "instrumentation"
            try:
                result = run_instrumentation(adb,shlex.join(command),grant,lambda:None,
                    update=update if args.bidirectional else None,on_background_update=background if args.bidirectional else None,
                    tick=lambda:drain(terminal.fd,terminal.output,.01),timeout=180 if args.bidirectional else 120)
            except InstrumentationFailure as error:
                result = error.result
                raise
            output = result.stdout.replace(invite,"<redacted-invite>").replace(values["token"],"<redacted-token>")
            completed = require_test_results(output,CLASSES,1)
            phases = (*PHASES[:-1],*SYNC_PHASES,PHASES[-1]) if args.bidirectional else PHASES
            if args.with_doze: phases = (*phases[:-1],*DOZE_PHASES,phases[-1])
            if re.findall(r"(?m)^INSTRUMENTATION_STATUS: zenpi_phase=(.*)$",output) != list(phases):
                raise ValueError("Camera pairing phases incomplete")
            if result.returncode != 0: raise ValueError("Camera instrumentation failed")
            if args.bidirectional:
                after_bytes = session.read_bytes()
                if not after_bytes.startswith(before_bytes): raise ValueError("Session prefix changed")
                appended = [json.loads(line) for line in after_bytes[len(before_bytes):].splitlines()]
                checkpoint = json.loads((session.parent/"project-workspace.json").read_text())
                sync_evidence = {"session_id":json.loads(before_bytes.splitlines()[0])["session_id"],
                    "provider_requests":len(provider_requests),"provider_prompt":provider_prompt(provider_requests[-1]) if provider_requests else None,
                    "provider_answer":DESKTOP_ANSWER,"provider_prompts":[provider_prompt(body) for body in provider_requests],"provider_answers":provider_answers,
                    "original_prefix_sha256":hashlib.sha256(before_bytes).hexdigest(),
                    "preserved_prefix_sha256":hashlib.sha256(after_bytes[:len(before_bytes)]).hexdigest(),
                    "appended_turns":[{field:record["turn"][field] for field in ("id","role","content")} for record in appended if record.get("kind")=="turn"],
                    "admissions":[record["event"] for record in appended if record.get("event",{}).get("type")=="remote_prompt_admission_v1" and record["event"].get("state")=="accepted"],
                    "bindings":checkpoint["remote_workspace"]["prompt_bindings"],
                    "progress":[record["event"] for record in appended if record.get("event",{}).get("type")=="remote_prompt_progress_v1"]}
                require_prompt_evidence(sync_evidence,device_id,project)
                replay_evidence = foreground_replay_evidence(output,sync_evidence["session_id"])
                terminal.expect(DESKTOP_ANSWER.encode(),timeout=15)
            png = subprocess.run(adb+["exec-out","run-as","app.zenpi.android","cat","files/emulator-qr-host.png"],check=True,capture_output=True,timeout=10).stdout
            if not png.startswith(b"\x89PNG\r\n\x1a\n"): raise ValueError("Missing transcript PNG")
            path = args.receipt.with_suffix(".png"); path.parent.mkdir(parents=True,exist_ok=True); path.write_bytes(png)
            screenshot = {"path":str(path),"sha256":hashlib.sha256(png).hexdigest()}
            if provenance != require_current_build() or host_provenance != require_current_host(binary): raise ValueError("Build provenance changed")
            if harness_hashes != {path:hashlib.sha256((ROOT/path).read_bytes()).hexdigest() for path in inputs}:
                raise ValueError("Harness inputs changed")
            if args.emulator_host_network: require_no_reverse(adb)
            passed = True
        except Exception as error:
            failure = type(error).__name__ + ": emulator camera workflow incomplete"
            if isinstance(error,subprocess.CalledProcessError) and stage in ("wifi_connect","apk_install","camera_permission"):
                def bounded(value):
                    return (value.decode(errors="replace") if isinstance(value,bytes) else value or "")[:4096]
                prerequisite_diagnostics = {"stage":stage,"exit_code":error.returncode,
                                            "stdout":bounded(error.stdout),"stderr":bounded(error.stderr)}
            if result is not None:
                output = result.stdout
                for private in (locals().get("invite"),locals().get("values",{}).get("token")):
                    if private: output=output.replace(private,"<redacted-invite>")
        finally:
            if args.bidirectional and locals().get("session") is not None and session.exists():
                records = [json.loads(line) for line in session.read_bytes().splitlines()]
                journal_turns = [{field:record["turn"][field] for field in ("role","content")}
                                 for record in records if record.get("kind")=="turn"]
            if reverse: subprocess.run(adb+["reverse","--remove",reverse],capture_output=True,timeout=10)
            if process is not None and process.poll() is None:
                subprocess.run(adb+["shell","am","force-stop","app.zenpi.android"],capture_output=True,timeout=10)
                subprocess.run(adb+["emu","kill"],capture_output=True,timeout=10)
                try: process.wait(timeout=30)
                except subprocess.TimeoutExpired: process.terminate(); process.wait(timeout=10)
                log.close()
            elif process is not None:
                log.close()
            if terminal is not None: terminal.cleanup()
            if server is not None:
                server.shutdown();server.server_close();provider_thread.join(timeout=5)
                if provider_thread.is_alive(): passed=False;failure="Provider cleanup incomplete"
    args.receipt.parent.mkdir(parents=True,exist_ok=True)
    path=args.receipt.with_suffix(".instrumentation.txt");path.write_text(output)
    receipt={"schema_version":"stage2-emulator-camera/v1","passed":passed,"failure":failure,"whole_item_accepted":False,
        "emulator_imagefile_camera_verified":passed,"production_scanner_to_pinned_tls_verified":passed,
        "physical_camera_scan_verified":False,"physical_phone_lan_verified":False,"direct_lan_verified":False,
        "adb_reverse_used":not args.emulator_host_network,
        "emulator_host_network_verified":passed and args.emulator_host_network,
        "transport":"emulator_host_alias" if args.emulator_host_network else "adb_reverse",
        "bidirectional_sync_verified":passed and args.bidirectional,"foreground_replay_verified":passed and args.bidirectional,
        "deep_idle_cache_foreground_resume_verified":passed and args.with_doze,
        "sync_evidence":sync_evidence,"replay_evidence":replay_evidence,"device_id":device_id,"project_id":locals().get("project"),
        "callback_steps":callback_steps,"provider_requests":len(provider_requests),
        "failure_stage":stage if not passed else None,"prerequisite_diagnostics":prerequisite_diagnostics,
        "wifi_association_setup":wifi_setup,
        "provider_prompts":[provider_prompt(body) for body in provider_requests],"provider_answers":provider_answers,
        "journal_turns":journal_turns,
        "serial":serial,"avd":args.avd,"android_api":api,"expected_android_api":args.expected_api,"endpoint":endpoint,
        "safety_code_input":"automated emulator-only input; not manual operator confirmation",
        "camera_fixture_orientation":"landscape imagefile input on owned emulator",
        "camera_fixture_placement":{"width":1280,"height":960,"qr_center_x":340,"qr_center_y":480},
        "camera_fixture_sha256":camera_hash,"harness_inputs":harness_hashes,
        "completed_tests":locals().get("completed",[]),"android_build_provenance":provenance,"host_build_provenance":host_provenance,
        "screenshot":screenshot,"instrumentation":{"path":str(path),"sha256":hashlib.sha256(path.read_bytes()).hexdigest()},
        "started_at_unix":started,"finished_at_unix":time.time()}
    args.receipt.write_text(json.dumps(receipt,indent=2)+"\n")
    print(json.dumps(receipt,indent=2))
    if not passed: raise SystemExit(1)


if __name__ == "__main__": serialized_device_validation(main)
