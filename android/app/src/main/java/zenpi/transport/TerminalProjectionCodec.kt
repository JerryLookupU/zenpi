package zenpi.transport

data class RemoteTerminal(val projectId: String, val sessionId: String, val subtabId: String,
    val subtabRevision: String, val text: String, val truncated: Boolean, val terminalId: String? = null,
    val size: TerminalSize? = null, val running: Boolean = true)
data class TerminalSize(val rows: Int, val cols: Int)

internal object TerminalProjectionCodec {
    fun valid(terminal: RemoteTerminal): Boolean = terminal.subtabId.matches(Regex("[0-9a-f]{64}"))
        && terminal.subtabRevision.matches(Regex("0|[1-9][0-9]*"))
        && terminal.subtabRevision.toBigInteger() <= java.math.BigInteger("18446744073709551615")
        && terminal.text.toByteArray(Charsets.UTF_8).size <= 16_384
        && terminal.text.none { Character.isISOControl(it) && it != '\n' && it != '\t' }
        && (terminal.terminalId == null || terminal.terminalId.matches(Regex("[0-9a-f]{64}")))
        && (terminal.size == null || terminal.terminalId != null
            && terminal.size.rows in 1..512 && terminal.size.cols in 1..512)
        && (terminal.terminalId != null || terminal.running)

    fun decode(value: com.google.gson.JsonElement?, project: String, session: String): RemoteTerminal? {
        if (value == null || value.isJsonNull) return null
        if (!value.isJsonObject) throw WireException("Invalid terminal projection")
        val item = value.asJsonObject
        if (item.keySet() != setOf("project_id", "session_id", "subtab_id", "subtab_revision", "text", "truncated", "read_only") +
            (if (item.has("terminal_id")) setOf("terminal_id") else emptySet()) +
            (if (item.has("running")) setOf("running") else emptySet()) +
            (if (item.has("size")) setOf("size") else emptySet())
            || FrameCodec.string(item, "project_id") != project || FrameCodec.string(item, "session_id") != session)
            throw WireException("Terminal owner mismatch")
        val tab = FrameCodec.string(item, "subtab_id")
        if (!tab.matches(Regex("[0-9a-f]{64}"))) throw WireException("Terminal subtab mismatch")
        val revision = item["subtab_revision"]
        if (!revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || !revision.toString().matches(Regex("0|[1-9][0-9]*"))
            || revision.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid terminal revision")
        for (field in listOf("truncated", "read_only")) {
            if (!item[field].isJsonPrimitive || !item[field].asJsonPrimitive.isBoolean) throw WireException("Invalid terminal flag")
        }
        if (!item["read_only"].asBoolean) throw WireException("Terminal control unsupported")
        val text = FrameCodec.string(item, "text")
        if (text.toByteArray().size > 16_384 || text.any { Character.isISOControl(it) && it != '\n' && it != '\t' })
            throw WireException("Invalid terminal screen")
        val terminalId = if (item.has("terminal_id")) FrameCodec.string(item, "terminal_id").also {
            if (!it.matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid terminal identity")
        } else null
        val size = if (item.has("size")) {
            if (terminalId == null || !item["size"].isJsonObject) throw WireException("Invalid terminal size")
            val dimensions = item["size"].asJsonObject
            if (dimensions.keySet() != setOf("rows", "cols")) throw WireException("Invalid terminal size fields")
            fun dimension(field: String): Int {
                val number = dimensions[field]
                if (!number.isJsonPrimitive || !number.asJsonPrimitive.isNumber
                    || !number.toString().matches(Regex("[1-9][0-9]*"))) throw WireException("Invalid terminal dimension")
                return number.toString().toIntOrNull()?.takeIf { it in 1..512 } ?: throw WireException("Terminal dimension limit")
            }
            TerminalSize(dimension("rows"), dimension("cols"))
        } else null
        val running = if (item.has("running")) {
            val flag = item["running"]
            if (terminalId == null || !flag.isJsonPrimitive || !flag.asJsonPrimitive.isBoolean) throw WireException("Invalid terminal lifecycle")
            flag.asBoolean
        } else true
        return RemoteTerminal(project, session, tab, revision.toString(), text, item["truncated"].asBoolean, terminalId, size, running)
            .also { if (!valid(it)) throw WireException("Invalid retained terminal") }
    }
}
