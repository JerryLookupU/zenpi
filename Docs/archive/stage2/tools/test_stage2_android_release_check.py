#!/usr/bin/env python3
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import stage2_android_release_check as checker

XML = '''<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="app.zenpi.android" android:versionCode="1">
<uses-sdk android:minSdkVersion="31" android:targetSdkVersion="35"/>
<uses-permission android:name="android.permission.INTERNET"/><uses-permission android:name="android.permission.ACCESS_NETWORK_STATE"/><uses-permission android:name="android.permission.CAMERA"/>
<application android:allowBackup="false" android:usesCleartextTraffic="false" android:fullBackupContent="@ref/0x7f010000" android:dataExtractionRules="@ref/0x7f010001">
<activity android:name="zenpi.MainActivity" android:exported="true" android:launchMode="singleTask" android:documentLaunchMode="never">
<intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter>
<intent-filter><action android:name="android.intent.action.VIEW"/><category android:name="android.intent.category.DEFAULT"/><category android:name="android.intent.category.BROWSABLE"/><data android:scheme="zenpi" android:host="pair"/></intent-filter>
</activity></application></manifest>'''
PIN = "a"*64
CERT = f"Number of signers: 1\nSigner #1 certificate SHA-256 digest: {PIN}\n"
TABLE = "Binary APK\nPackage name=app.zenpi.android id=7f\n  type xml id=01 entryCount=2\n    resource 0x7f010000 xml/backup_rules\n    resource 0x7f010001 xml/data_extraction_rules\n"
EXCLUDES = ''.join(f'<exclude domain="{name}" path="."/>' for name in sorted(checker.DOMAINS))
BACKUP = f'<full-backup-content>{EXCLUDES}</full-backup-content>'
EXTRACTION = f'<data-extraction-rules><cloud-backup>{EXCLUDES}</cloud-backup><device-transfer>{EXCLUDES}</device-transfer></data-extraction-rules>'


def inspection_outputs():
    return [CERT.encode(), b'align', XML.encode(), TABLE.encode(), b'default\n',
            b'res/xml/backup_rules.xml\n', BACKUP.encode(), b'res/xml/data_extraction_rules.xml\n', EXTRACTION.encode()]


class ReleaseInspectionTests(unittest.TestCase):
    def test_valid_manifest_is_non_debuggable_with_qr_and_bounded_permissions(self):
        result=checker.manifest(XML)
        self.assertEqual(result["package"],"app.zenpi.android")
        self.assertFalse(result["debuggable"])

    def test_debug_test_backup_cleartext_sdk_and_permission_regressions_refused(self):
        for changed in (XML.replace('<application ','<application android:debuggable="true" '),
            XML.replace('<application ','<application android:testOnly="true" '),
            XML.replace('allowBackup="false"','allowBackup="true"'),
            XML.replace('usesCleartextTraffic="false"','usesCleartextTraffic="true"'),
            XML.replace('minSdkVersion="31"','minSdkVersion="30"'),
            XML.replace('android:fullBackupContent="@ref/0x7f010000"',''),
            XML.replace('android.permission.CAMERA','android.permission.FOREGROUND_SERVICE'),
            XML.replace('package="app.zenpi.android"','package="other"'),
            XML.replace('versionCode="1"','versionCode="0"')):
            with self.subTest(changed=changed),self.assertRaises(ValueError):checker.manifest(changed)

    def test_qr_entrypoint_and_foreground_service_regressions_refused(self):
        for changed in (XML.replace('android:host="pair"','android:host="other"'),
            XML.replace('android:exported="true"','android:exported="false"'),
            XML.replace('android:scheme="zenpi"','android:scheme="http"'),
            XML.replace('</application>','<service android:name="androidx.work.impl.foreground.SystemForegroundService"/></application>')):
            with self.assertRaises(ValueError):checker.manifest(changed)

    def test_exported_components_require_known_system_permission_and_exact_deep_link_categories(self):
        protected='<service android:name="androidx.work.impl.background.systemjob.SystemJobService" android:exported="true" android:permission="android.permission.BIND_JOB_SERVICE"/>'
        checker.manifest(XML.replace('</application>',protected+'</application>'))
        for component in ('<service android:name="foreign" android:exported="true"/>',
            '<provider android:name="foreign"/>',
            protected.replace('android.permission.BIND_JOB_SERVICE','android.permission.INTERNET'),
            protected.replace('<service ', '<activity '),
            '<service android:name="zenpi.MainActivity" android:exported="true"/>',
            '<receiver android:exported="false"/>',
            '<receiver android:name="duplicate" android:exported="false"/><receiver android:name="duplicate" android:exported="false"/>',
            '<receiver android:name="foreign"><intent-filter><action android:name="arbitrary"/></intent-filter></receiver>'):
            with self.assertRaises(ValueError):checker.manifest(XML.replace('</application>',component+'</application>'))
        with self.assertRaises(ValueError):checker.manifest(XML.replace('<category android:name="android.intent.category.BROWSABLE"/>',''))

    def test_only_single_exact_expected_certificate_passes(self):
        self.assertEqual(checker.certificate(CERT,PIN),PIN)
        for value in ("",CERT.replace(PIN,"b"*64),CERT+CERT,CERT.replace('signers: 1','signers: 2')):
            with self.assertRaises(ValueError):checker.certificate(value,PIN)

    def test_packaged_pairing_entry_cannot_create_a_parallel_task_owner(self):
        for changed in (XML.replace(' android:launchMode="singleTask"', ''),
            XML.replace('launchMode="singleTask"', 'launchMode="standard"'),
            XML.replace('launchMode="singleTask"', 'launchMode="singleTop"'),
            XML.replace('documentLaunchMode="never"', 'documentLaunchMode="always"'),
            XML.replace(' android:documentLaunchMode="never"', ''),
            XML.replace('<activity ', '<activity android:taskAffinity="other" '),
            XML.replace('<activity ', '<activity android:process=":other" '),
            XML.replace('<application ', '<application android:allowTaskReparenting="true" ')):
            with self.subTest(changed=changed), self.assertRaises(ValueError): checker.manifest(changed)
        checker.manifest(XML.replace('launchMode="singleTask"', 'launchMode="2"').replace('documentLaunchMode="never"', 'documentLaunchMode="3"'))

    def test_release_rejects_shared_uid_and_instrumentation_even_without_test_only(self):
        for changed in (XML.replace('<manifest ', '<manifest android:sharedUserId="android.uid.system" '),
            XML.replace('<manifest ', '<manifest android:sharedUserMaxSdkVersion="32" '),
            XML.replace('</manifest>', '<instrumentation android:name="test.Runner" android:targetPackage="app.zenpi.android"/></manifest>')):
            with self.subTest(changed=changed),self.assertRaises(ValueError):checker.manifest(changed)

    def test_dynamic_receiver_permission_requires_one_exact_signature_declaration(self):
        name = 'app.zenpi.android.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION'
        use = f'<uses-permission android:name="{name}"/>'
        declaration = f'<permission android:name="{name}" android:protectionLevel="signature"/>'
        valid = XML.replace('<application ', use + declaration + '<application ')
        checker.manifest(valid)
        checker.manifest(valid.replace('protectionLevel="signature"', 'protectionLevel="0x2"'))
        for changed in (valid.replace(declaration, ''), valid.replace(use, ''),
            valid.replace(declaration, declaration + declaration),
            valid.replace('protectionLevel="signature"', 'protectionLevel="normal"'),
            valid.replace('protectionLevel="signature"', 'protectionLevel="signature|privileged"'),
            valid.replace('<permission ', '<permission android:knownCerts="@array/keys" '),
            valid.replace('<permission ', '<permission android:permissionFlags="0x10" '),
            valid.replace('<permission ', '<permission-tree '),
            XML.replace('<application ', '<permission-group android:name="foreign"/><application '),
            XML.replace('<application ', '<permission android:name="foreign" android:protectionLevel="signature"/><application ')):
            with self.subTest(changed=changed),self.assertRaises(ValueError):checker.manifest(changed)

    def fixture(self):
        temporary=tempfile.TemporaryDirectory();self.addCleanup(temporary.cleanup)
        root=Path(temporary.name);apk=root/'release.apk';apk.write_bytes(b'fixture APK')
        return root,apk,root/'receipt.json'

    def test_inspection_never_claims_source_install_upgrade_or_reproducibility(self):
        root,apk,receipt=self.fixture()
        outputs=inspection_outputs()
        with patch.object(checker.subprocess,'run',side_effect=[subprocess.CompletedProcess([],0,value,b'') for value in outputs]) as run:
            result=checker.inspect(apk,PIN,root,receipt)
        self.assertTrue(result['passed']);self.assertEqual(9,run.call_count)
        self.assertTrue(result['backup_rule_contents_verified'])
        self.assertEqual(result['summary']['backup_policy']['domains'],9)
        self.assertEqual(result['schema_version'],'stage2-release-inspection/v2')
        for name in ('whole_item_accepted','source_attribution_verified','install_verified','upgrade_verified','reproducible_build_verified'):
            self.assertFalse(result[name])
        self.assertEqual(result,json.loads(receipt.read_text()))

    def test_unsigned_wrong_signer_and_failed_alignment_leave_failed_receipts(self):
        for outputs in ([subprocess.CompletedProcess([],1,b'',b'unsigned')],
            [subprocess.CompletedProcess([],0,CERT.replace(PIN,'b'*64).encode(),b''),subprocess.CompletedProcess([],0,b'',b''),subprocess.CompletedProcess([],0,XML.encode(),b'')],
            [subprocess.CompletedProcess([],0,CERT.encode(),b''),subprocess.CompletedProcess([],1,b'',b'bad align')]):
            root,apk,receipt=self.fixture()
            with patch.object(checker.subprocess,'run',side_effect=outputs):result=checker.inspect(apk,PIN,root,receipt)
            self.assertFalse(result['passed']);self.assertIsNotNone(result['failure'])

    def test_changed_apk_never_passes_and_existing_receipt_is_not_overwritten(self):
        root,apk,receipt=self.fixture();responses=iter(inspection_outputs())
        def run(*args,**kwargs):
            apk.write_bytes(b'changed');return subprocess.CompletedProcess([],0,next(responses),b'')
        with patch.object(checker.subprocess,'run',side_effect=run):result=checker.inspect(apk,PIN,root,receipt)
        self.assertFalse(result['passed']);before=receipt.read_bytes()
        with self.assertRaises(ValueError):checker.inspect(apk,PIN,root,receipt)
        self.assertEqual(before,receipt.read_bytes())

    def test_invalid_expected_pin_or_symlink_refuses_before_tools(self):
        root,apk,receipt=self.fixture();alias=root/'alias.apk';alias.symlink_to(apk)
        with patch.object(checker.subprocess,'run') as run:
            with self.assertRaises(ValueError):checker.inspect(apk,'bad',root,receipt)
            with self.assertRaises(ValueError):checker.inspect(alias,PIN,root,receipt)
            run.assert_not_called()

    def test_packaged_reference_must_match_the_exact_single_resource_identity(self):
        checker.backup_references(XML,TABLE)
        checker.backup_references(XML.replace('@ref/0x7f010000','@xml/backup_rules'),TABLE)
        for text,table in ((XML.replace('@ref/0x7f010000','@ref/0x7f010001'),TABLE),
            (XML.replace('@ref/0x7f010000','@xml/uninspected'),TABLE),
            (XML,TABLE.replace('xml/backup_rules','xml/other')),
            (XML,TABLE+'    resource 0x7f010002 xml/backup_rules\n'),
            (XML,TABLE+'    resource 0x7f010000 xml/other\n')):
            with self.subTest(text=text,table=table),self.assertRaises(ValueError):checker.backup_references(text,table)

    def test_packaged_rules_reject_missing_partial_duplicate_included_and_overridden_domains(self):
        for text,extraction in ((BACKUP,False),(EXTRACTION,True)):
            checker.backup_xml(text,extraction)
            for changed in (text.replace('path="."','path="paired-host.v1"',1),
                text.replace('domain="database"','domain="file"',1),
                text.replace('<exclude domain="database" path="."/>','',1),
                text.replace('<exclude ','<include ',1),
                text.replace('domain="database"','domain="unknown"',1),
                text.replace('domain="database"','requireFlags="clientSideEncryption" domain="database"',1)):
                with self.subTest(changed=changed),self.assertRaises(ValueError):checker.backup_xml(changed,extraction)
        for changed in (EXTRACTION.replace('device-transfer','cloud-backup'),
            EXTRACTION.replace('<device-transfer>','<device-transfer enabled="true">'),
            EXTRACTION.replace('<data-extraction-rules>','<data-extraction-rules override="true">')):
            with self.assertRaises(ValueError):checker.backup_xml(changed,True)

    def test_packaged_configs_paths_xml_tool_errors_and_oversized_outputs_leave_failed_receipts(self):
        for index,replacement in ((3,TABLE.replace('backup_rules','other').encode()),
            (4,b'default\nv31\n'),(4,b'default\ndefault\n'),(4,b''),
            (5,b'res/xml/uninspected.xml'),(5,b'../backup_rules.xml'),
            (6,BACKUP.replace('<exclude ','<include ',1).encode()),
            (8,EXTRACTION.replace('device-transfer','cloud-backup').encode()),
            (8,b'<invalid'),(8,b'\xff'),(8,b'x'*(4*1024*1024+1))):
            root,apk,receipt=self.fixture();outputs=inspection_outputs();outputs[index]=replacement
            with patch.object(checker.subprocess,'run',side_effect=[subprocess.CompletedProcess([],0,value,b'') for value in outputs]):
                result=checker.inspect(apk,PIN,root,receipt)
            self.assertFalse(result['passed']);self.assertFalse(result['backup_rule_contents_verified'])
            self.assertIsNotNone(result['failure'])
        for failure in (subprocess.TimeoutExpired(['resources'],60), OSError('tool unavailable')):
            root,apk,receipt=self.fixture()
            outcomes=[subprocess.CompletedProcess([],0,value,b'') for value in inspection_outputs()[:5]]+[failure]
            with patch.object(checker.subprocess,'run',side_effect=outcomes):result=checker.inspect(apk,PIN,root,receipt)
            self.assertFalse(result['passed']);self.assertFalse(result['backup_rule_contents_verified'])


if __name__=='__main__':unittest.main()
