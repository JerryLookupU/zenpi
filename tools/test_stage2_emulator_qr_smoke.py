#!/usr/bin/env python3
import struct
import subprocess
import unittest
from unittest.mock import patch
import zlib
from stage2_emulator_qr_smoke import camera_png, public_command
from stage2_emulator_qr_smoke import connect_emulator_wifi


class EmulatorWifiTests(unittest.TestCase):
    READY_ROUTE = "10.0.2.0/24 dev wlan0 proto kernel scope link src 10.0.2.16\n"
    def test_refuses_nonemulator_before_association(self):
        with patch("stage2_emulator_qr_smoke.subprocess.run",return_value=subprocess.CompletedProcess([],0,"0\n","")) as run:
            with self.assertRaises(ValueError): connect_emulator_wifi(["adb","-s","fixture"])
            self.assertEqual(1,run.call_count)

    def test_shell_association_never_uses_debug_root(self):
        with patch("stage2_emulator_qr_smoke.subprocess.run",side_effect=[subprocess.CompletedProcess([],0,"1\n",""),
                subprocess.CompletedProcess([],0,"",""), subprocess.CompletedProcess([],0,self.READY_ROUTE,"")]) as run:
            self.assertEqual("shell",connect_emulator_wifi(["adb"]))
            self.assertIn(["adb","shell","cmd","wifi","connect-network","AndroidWifi","open"],
                [call.args[0] for call in run.call_args_list])

    def test_only_expected_permission_refusal_allows_emulator_debug_association(self):
        denied=subprocess.CompletedProcess([],255,"","SecurityException: Uid 2000 does not have access")
        with patch("stage2_emulator_qr_smoke.subprocess.run",side_effect=[subprocess.CompletedProcess([],0,"1","") ,denied,
                subprocess.CompletedProcess([],0,"",""), subprocess.CompletedProcess([],0,self.READY_ROUTE,"")]) as run:
            self.assertEqual("owned_emulator_debug_su",connect_emulator_wifi(["adb"]))
            self.assertIn(["adb","shell","su","0","cmd","wifi","connect-network","AndroidWifi","open"],
                [call.args[0] for call in run.call_args_list])
        with patch("stage2_emulator_qr_smoke.subprocess.run",side_effect=[subprocess.CompletedProcess([],0,"1",""),
                subprocess.CompletedProcess([],1,"","unknown command")]) as run:
            with self.assertRaises(subprocess.CalledProcessError): connect_emulator_wifi(["adb"])
            self.assertEqual(2,run.call_count)


class EmulatorCameraFixtureTests(unittest.TestCase):
    def test_terminal_pixels_are_preserved_at_declared_position_with_quiet_zone(self):
        pixels = bytearray(b"\xff" * 100)
        for y in range(2,6):
            pixels[y*10+3:y*10+7] = b"\0\0\0\0"
        png = camera_png(b"P5\n10 10\n255\n"+pixels)
        self.assertEqual(b"\x89PNG\r\n\x1a\n",png[:8])
        offset=8; chunks={}
        while offset<len(png):
            size=struct.unpack(">I",png[offset:offset+4])[0]
            kind=png[offset+4:offset+8]; data=png[offset+8:offset+8+size]
            crc=struct.unpack(">I",png[offset+8+size:offset+12+size])[0]
            self.assertEqual(zlib.crc32(kind+data),crc)
            chunks[kind]=data;offset+=12+size
        self.assertEqual((1280,960,8,0,0,0,0),struct.unpack(">IIBBBBB",chunks[b"IHDR"]))
        raw=zlib.decompress(chunks[b"IDAT"])
        self.assertEqual(1281*960,len(raw))
        for y in range(960):
            self.assertEqual(0,raw[y*1281])
            for x in range(1280):
                self.assertEqual(0 if 338<=x<342 and 478<=y<482 else 255,raw[y*1281+x+1])

    def test_refuses_missing_rectangular_or_nonbinary_symbols(self):
        for pgm in (b"P5\n2 2\n255\n\xff\xff\xff\xff", b"P5\n2 2\n255\n\0\0\xff\xff",
                    b"P5\n2 2\n255\n\0\0\0\xfe",b"P5\n2 2\n255\n\0"):
            with self.assertRaises(ValueError): camera_png(pgm)

    def test_only_emulator_sas_and_public_expectations_are_supplied(self):
        command=public_command("127.0.0.1:1234","a"*64,"b"*64,"012345")
        extras={command[index+1]:command[index+2] for index,value in enumerate(command) if value=="-e"}
        self.assertEqual({"class","emulator_camera_run","expected_address","expected_fingerprint","expected_project","safety_code"},set(extras))
        self.assertEqual("true",extras["emulator_camera_run"])
        sync=public_command("10.0.2.2:1234","a"*64,"b"*64,"012345",True)
        sync_extras={sync[index+1]:sync[index+2] for index,value in enumerate(sync) if value=="-e"}
        self.assertEqual(extras.keys()|{"bidirectional"},sync_extras.keys())
        self.assertEqual("true",sync_extras["bidirectional"])
        self.assertFalse({"invite","uri","token"}&sync_extras.keys())
        doze=public_command("10.0.2.2:1234","a"*64,"b"*64,"012345",True,True)
        self.assertIn("with_doze",doze)
        with self.assertRaises(ValueError): public_command("10.0.2.2:1234","a"*64,"b"*64,"012345",False,True)
        for invalid in ("12345","1234567","abcdef"):
            with self.assertRaises(ValueError): public_command("host","pin","project",invalid)


if __name__ == "__main__": unittest.main()
