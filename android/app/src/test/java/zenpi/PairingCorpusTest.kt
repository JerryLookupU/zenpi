package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.PairingInvite

class PairingCorpusTest {
    @Test fun productionRustTerminalPixelsDecodeWithAndroidScannerEngineAndMatchingSafetyCode() {
        val current = System.getProperty("zenpi.qr.fixture")?.let { java.io.File(it) }
        val names = if (current == null) listOf("ipv4", "ipv6") else {
            val manifest = JsonParser.parseString(java.io.File(current, "terminal-qr-manifest.json").readText()).asJsonObject
            assertEquals("stage2-terminal-qr-fixture/v1", manifest["schema_version"].asString)
            assertTrue(manifest["fixture_only"].asBoolean)
            manifest.getAsJsonArray("cases").map { it.asString }.also {
                assertEquals(listOf("ipv4", "ipv6", "ipv4-port-min", "ipv6-long", "ipv4-loopback", "ipv6-loopback") +
                    (0 until 48).map { index -> "payload-%02d".format(index) }, it)
            }
        }
        for(name in names) {
            fun resource(suffix:String)= if (current != null) java.io.File(current, "terminal-qr-$name.$suffix").inputStream()
                else requireNotNull(javaClass.classLoader?.getResourceAsStream("terminal-qr-$name.$suffix"))
            val fixture=resource("json").bufferedReader().use{JsonParser.parseReader(it).asJsonObject}
            assertTrue(fixture["fixture_only"].asBoolean)
            val bytes=resource("pgm").use{it.readBytes()}
            val separators=bytes.indices.filter{bytes[it]==10.toByte()}.take(3)
            assertEquals(3,separators.size)
            val end=separators.last()+1
            val header=String(bytes.copyOfRange(0,end),Charsets.US_ASCII).trim().split(Regex("\\s+"))
            assertEquals(listOf("P5",fixture["width"].asString,fixture["height"].asString,"255"),header)
            val width=fixture["width"].asInt;val height=fixture["height"].asInt
            assertEquals(width*height,bytes.size-end)
            val source=com.google.zxing.PlanarYUVLuminanceSource(bytes.copyOfRange(end,bytes.size),width,height,0,0,width,height,false)
            val bitmap=com.google.zxing.BinaryBitmap(com.google.zxing.common.HybridBinarizer(source))
            val decoded=try { zenpi.pairing.PairingQrReader().decode(bitmap).text }
                catch (error: com.google.zxing.ReaderException) { throw AssertionError("Production terminal QR failed to decode: $name", error) }
            assertEquals(fixture["uri"].asString,decoded)
            val invite=PairingInvite.parse(decoded,fixture["now"].asLong) { 0 }
            assertEquals(fixture["address"].asString,invite.address)
            assertEquals(fixture["safety_code"].asString,invite.safetyCode)
            if (current != null) {
                val now = fixture["now"].asLong
                assertEquals(300L, invite.remainingSeconds(now))
                assertEquals(1L, invite.remainingSeconds(now + 299))
                assertTrue(runCatching { invite.requireFresh(now + 300) }.isFailure)
                assertTrue(runCatching { invite.requireFresh(now) }.isFailure)
                val beforeWindow = runCatching { PairingInvite.parse(decoded, now - 1) { 0 } }
                assertTrue(beforeWindow.isFailure)
                assertFalse(invite.toString().contains(invite.token))
            }
        }
    }
    @Test fun sharedRustAndroidInvitesAgreeOnAcceptanceAddressAndSafetyCode() {
        val input=requireNotNull(javaClass.classLoader?.getResourceAsStream("pairing-corpus.jsonl"))
        val names=mutableSetOf<String>()
        input.bufferedReader().useLines { lines -> lines.forEach { row ->
            val case=JsonParser.parseString(row).asJsonObject
            val name=case["name"].asString;assertTrue(names.add(name))
            val invite=runCatching{PairingInvite.parse(case["uri"].asString,case["now"].asLong) { 0 }}
            assertEquals(name,case["accept"].asBoolean,invite.isSuccess)
            invite.getOrNull()?.let {
                assertEquals(name,case["address"].asString,it.address)
                assertEquals(name,case["safety_code"].asString,it.safetyCode)
                assertFalse(it.toString().contains(it.token))
            }
        } }
        assertTrue(names.size>=40)
    }
}
