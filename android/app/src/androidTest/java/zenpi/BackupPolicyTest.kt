package zenpi

import android.content.pm.ApplicationInfo
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import org.xmlpull.v1.XmlPullParser

class BackupPolicyTest {
    private val domains=setOf("root","file","database","sharedpref","external","device_root","device_file","device_database","device_sharedpref")
    @Test fun packagedApplicationDisablesBackupAndExcludesEveryDataDomainFromTransfer() {
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        assertEquals(0,context.applicationInfo.flags and ApplicationInfo.FLAG_ALLOW_BACKUP)
        context.assets.openXmlResourceParser("AndroidManifest.xml").use { parser ->
            var applicationSeen=false
            while(parser.eventType!=XmlPullParser.END_DOCUMENT) {
                if(parser.eventType==XmlPullParser.START_TAG && parser.name=="application") {
                    assertFalse(applicationSeen);applicationSeen=true
                    val android="http://schemas.android.com/apk/res/android"
                    assertFalse(parser.getAttributeBooleanValue(android,"allowBackup",true))
                    assertEquals(context.resources.getIdentifier("backup_rules","xml",context.packageName),
                        parser.getAttributeResourceValue(android,"fullBackupContent",0))
                    assertEquals(context.resources.getIdentifier("data_extraction_rules","xml",context.packageName),
                        parser.getAttributeResourceValue(android,"dataExtractionRules",0))
                }
                parser.next()
            }
            assertTrue(applicationSeen)
        }
        for((resource,expectedRoot,branches) in listOf(
            Triple("backup_rules","full-backup-content",setOf("full-backup-content")),
            Triple("data_extraction_rules","data-extraction-rules",setOf("cloud-backup","device-transfer")))) {
            val id=context.resources.getIdentifier(resource,"xml",context.packageName)
            assertNotEquals(0,id)
            val seen=mutableMapOf<String,MutableSet<String>>()
            context.resources.getXml(id).use { parser ->
                var branch:String?=null;var root:String?=null
                while(parser.eventType!=XmlPullParser.END_DOCUMENT) {
                    if(parser.eventType==XmlPullParser.START_TAG) {
                        val tag=parser.name
                        if(parser.depth==1){root=tag;assertEquals(expectedRoot,tag)}
                        if(tag in branches){branch=tag;assertNull(seen.put(tag,mutableSetOf()))}
                        else if(tag=="exclude") {
                            assertEquals(".",parser.getAttributeValue(null,"path"))
                            assertEquals(2,parser.attributeCount)
                            val domain=parser.getAttributeValue(null,"domain")
                            assertTrue(domain in domains);assertTrue(requireNotNull(seen[branch]).add(domain))
                        } else assertEquals(expectedRoot,tag)
                    }
                    if(parser.eventType==XmlPullParser.END_TAG && parser.name==branch)branch=null
                    parser.next()
                }
                assertEquals(expectedRoot,root)
            }
            assertEquals(branches,seen.keys)
            seen.values.forEach{assertEquals(domains,it)}
        }
    }
}
