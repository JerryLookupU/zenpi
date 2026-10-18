package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.FrameCodec
import zenpi.transport.RemoteSchema
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream

class RemoteProtocolCorpusTest {
    @Test fun sharedMessageContractChecksFieldsChannelsAndRequiredMembers() {
        val contract = requireNotNull(javaClass.classLoader?.getResourceAsStream("schema-contract.json")).bufferedReader().use {
            JsonParser.parseReader(it).asJsonObject
        }
        val messages = contract["messages"].asJsonObject
        assertEquals(13, messages.size())
        assertEquals(messages.keySet(), RemoteSchema.MESSAGE_FIELDS.keys)
        assertEquals(messages.keySet(), RemoteSchema.MESSAGE_CHANNELS.keys)
        for ((name, value) in messages.entrySet()) {
            val message = value.asJsonObject
            val fields = message["fields"].asJsonArray.map { it.asString }
            assertEquals(fields.size, fields.toSet().size)
            assertEquals(fields.toSet(), message["payload"].asJsonObject.keySet())
            assertEquals(fields.toSet(), RemoteSchema.MESSAGE_FIELDS[name])
            assertEquals(message["channel"].asString, RemoteSchema.MESSAGE_CHANNELS[name])
            assertEquals(name, message["payload"].asJsonObject["type"].asString)
            val frame = message["envelope"].asJsonObject.deepCopy().apply {
                add("schema_version", contract["schema_version"])
                add("channel", message["channel"])
                add("payload", message["payload"].deepCopy())
            }
            assertEquals(name, FrameCodec.decode(frame.toString().toByteArray())["payload"].asJsonObject["type"].asString)
            for (field in fields) {
                val missing = frame.deepCopy().apply { getAsJsonObject("payload").remove(field) }
                assertTrue("$name missing $field", runCatching { FrameCodec.decode(missing.toString().toByteArray()) }.isFailure)
            }
            val extra = frame.deepCopy().apply { getAsJsonObject("payload").addProperty("unknown_field", true) }
            assertTrue("$name extra field", runCatching { FrameCodec.decode(extra.toString().toByteArray()) }.isFailure)
            for (channel in listOf("control", "event", "bulk")) {
                val changed = frame.deepCopy().apply { addProperty("channel", channel) }
                assertEquals("$name $channel", channel == message["channel"].asString,
                    runCatching { FrameCodec.decode(changed.toString().toByteArray()) }.isSuccess)
            }
        }
    }
    @Test fun sharedSchemaContractExercisesEveryOperationAndOutcome() {
        val contract = requireNotNull(javaClass.classLoader?.getResourceAsStream("schema-contract.json")).bufferedReader().use {
            JsonParser.parseReader(it).asJsonObject
        }
        assertEquals(1, contract["schema_version"].asInt)
        assertEquals(contract["schema_version"].asInt, RemoteSchema.VERSION)
        for ((field, values) in listOf("read_operations" to RemoteSchema.READ_OPERATIONS,
            "write_operations" to RemoteSchema.WRITE_OPERATIONS, "execution_states" to RemoteSchema.EXECUTION_STATES,
            "error_codes" to RemoteSchema.ERROR_CODES)) {
            assertEquals(field, contract[field].asJsonArray.map { it.asString }.toSet(), values)
        }
        val names = mutableSetOf<String>()
        for ((field, write) in listOf("read_operations" to false, "write_operations" to true)) {
            for (entry in contract[field].asJsonArray) {
                val name = entry.asString
                assertTrue(names.add(name))
                val frame = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"matrix","payload":{"type":"request","operation":"$name","arguments":{}}}""").asJsonObject
                fun accepted() = runCatching { FrameCodec.decode(frame.toString().toByteArray()) }.isSuccess
                assertEquals(name, !write, accepted())
                frame.addProperty("required_owner", "owner")
                assertEquals("$name owner only", !write, accepted())
                frame.addProperty("expected_revision", java.math.BigInteger("18446744073709551615"))
                assertTrue(name, accepted())
                assertEquals("18446744073709551615", FrameCodec.decode(frame.toString().toByteArray())["expected_revision"].toString())
                frame.remove("required_owner")
                assertEquals("$name revision only", !write, accepted())
                frame.addProperty("required_owner", "owner")
                for (channel in listOf("event", "bulk")) {
                    frame.addProperty("channel", channel)
                    assertFalse("$name $channel", accepted())
                }
            }
        }
        assertEquals(33, names.size)
        val states = mutableSetOf<String>()
        for (entry in contract["execution_states"].asJsonArray) {
            val state = entry.asString
            assertTrue(states.add(state))
            val frame = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"matrix","execution_state":"$state","payload":{"type":"response","result":{}}}""").asJsonObject
            fun accepted() = runCatching { FrameCodec.decode(frame.toString().toByteArray()) }.isSuccess
            assertEquals(state, state != "failed", accepted())
            if (state == "failed") {
                val codes = mutableSetOf<String>()
                for (code in contract["error_codes"].asJsonArray) {
                    assertTrue(codes.add(code.asString))
                    frame.add("code", code)
                    assertTrue(code.asString, accepted())
                }
                assertEquals(12, codes.size)
                frame.addProperty("code", "unknown_error")
                assertFalse(accepted())
            }
        }
        assertEquals(6, states.size)
        for (wire in listOf(
            """{"schema_version":1,"channel":"control","id":"matrix","payload":{"type":"request","operation":"unknown_operation","arguments":{}}}""",
            """{"schema_version":1,"channel":"control","id":"matrix","execution_state":"unknown_state","payload":{"type":"response","result":{}}}""")) {
            assertTrue(runCatching { FrameCodec.decode(wire.toByteArray()) }.isFailure)
        }
    }
    private fun corpus() = requireNotNull(javaClass.classLoader?.getResourceAsStream("corpus.jsonl")).bufferedReader().use { input ->
        input.lineSequence().map { JsonParser.parseString(it).asJsonObject }.toList()
    }
    @Test fun rawWireCorpusSharesRustAcceptanceWithoutPreParsingOrCoercion() {
        val names=mutableSetOf<String>();var accepted=0;var refused=0
        for(case in corpus()) {
            val name=case["name"].asString;assertTrue(names.add(name))
            val wire=case["wire"].asString.toByteArray(Charsets.UTF_8)
            val decoded=runCatching{FrameCodec.decode(wire)}
            assertEquals(name,case["accept"].asBoolean,decoded.isSuccess)
            if(decoded.isSuccess) {
                accepted++
                val value=decoded.getOrThrow();val output=ByteArrayOutputStream()
                FrameCodec.write(output,value)
                assertEquals(name,value,FrameCodec.read(ByteArrayInputStream(output.toByteArray())))
            } else refused++
        }
        assertTrue(accepted>=10);assertTrue(refused>=25)
    }
    @Test fun boundedSeededMutationFramesEitherRefuseOrRoundtripWithoutCursorInvention() {
        val random=java.util.Random(0x72656d6f746531L)
        val cases=corpus().map{it["wire"].asString.toByteArray(Charsets.UTF_8)}
        repeat(2048) { index ->
            val original=cases[random.nextInt(cases.size)]
            val at=random.nextInt(original.size)
            val bytes=when(index%5) {
                0 -> original.clone().apply{this[at]=(this[at].toInt() xor (1 shl random.nextInt(8))).toByte()}
                1 -> original.copyOfRange(0,at)+original.copyOfRange(at+1,original.size)
                2 -> original.copyOfRange(0,at)+byteArrayOf(random.nextInt(256).toByte())+original.copyOfRange(at,original.size)
                3 -> original.copyOf(at)
                else -> original+original.copyOfRange(at,original.size)
            }
            assertTrue(bytes.size<=4096)
            val decoded=runCatching{FrameCodec.decode(bytes)}
            val framed=java.nio.ByteBuffer.allocate(4).putInt(bytes.size).array()+bytes
            val read=runCatching{FrameCodec.read(ByteArrayInputStream(framed))}
            assertEquals("mutation $index",decoded.isSuccess,read.isSuccess)
            decoded.getOrNull()?.let { value ->
                assertEquals(value,read.getOrThrow())
                val output=ByteArrayOutputStream();FrameCodec.write(output,value)
                assertEquals(value,FrameCodec.read(ByteArrayInputStream(output.toByteArray())))
            }
        }
    }
    @Test fun exactPayloadNodeAndDepthBudgetsMatchRustBoundaries() {
        fun frame(arguments:String)="""{"schema_version":1,"channel":"control","id":"r","payload":{"type":"request","operation":"inspect","arguments":$arguments}}""".toByteArray()
        for((nodes,expected) in listOf(4096 to true,4097 to false)) {
            val arguments="{\"items\":["+List(nodes-2){"0"}.joinToString(",")+"]}"
            assertEquals("nodes $nodes",expected,runCatching{FrameCodec.decode(frame(arguments))}.isSuccess)
        }
        for((depth,expected) in listOf(32 to true,33 to false)) {
            val arguments="{\"nested\":"+"[".repeat(depth-1)+"null"+"]".repeat(depth-1)+"}"
            assertEquals("depth $depth",expected,runCatching{FrameCodec.decode(frame(arguments))}.isSuccess)
        }
    }
}
