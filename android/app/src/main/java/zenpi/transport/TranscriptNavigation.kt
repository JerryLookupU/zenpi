package zenpi.transport

internal object TranscriptNavigation {
    const val MAX_PREVIOUS_PAGES = 7
    private fun validPage(page: TranscriptPage): Boolean {
        fun identifier(value: String, limit: Int = 128) = value.isNotBlank()
            && value.toByteArray(Charsets.UTF_8).size <= limit && value.none(Char::isISOControl)
        if (!identifier(page.projectId) || !identifier(page.sessionId) || !identifier(page.turnId)
            || page.role !in setOf("system", "user", "assistant", "tool")
            || (page.workerRequestId == null) != (page.subtabId == null)
            || page.workerRequestId?.let { !identifier(it) } == true
            || page.subtabId?.matches(Regex("[0-9a-f]{64}")) == false
            || (page.toolCallId == null) != (page.toolName == null)
            || page.toolCallId?.let { page.role != "assistant" || !identifier(it) } == true
            || page.toolName?.let { !identifier(it, 64) } == true
            || page.journalRevision?.let { page.workerRequestId != null || !it.matches(Regex("0|[1-9][0-9]*"))
                || it.toBigInteger() > java.math.BigInteger("18446744073709551615") } == true) return false
        val bytes = page.text.toByteArray(Charsets.UTF_8).size
        val limit = if (page.toolCallId != null) 65536 else 4 * 1024 * 1024
        return page.digest.matches(Regex("[0-9a-f]{64}")) && page.offset in 0..limit
            && page.nextOffset in page.offset..limit && page.totalBytes in page.nextOffset..limit
            && bytes <= 8192 && page.nextOffset - page.offset == bytes
            && page.hasMore == (page.nextOffset < page.totalBytes) && (!page.hasMore || bytes > 0)
            && !page.text.contains('\u0000')
    }
    private fun consecutive(first: TranscriptPage, second: TranscriptPage): Boolean =
        first.copy(offset = second.offset, nextOffset = second.nextOffset, text = second.text, hasMore = second.hasMore) == second
            && first.hasMore && first.nextOffset == second.offset
    fun validChain(pages: List<TranscriptPage>, current: TranscriptPage): Boolean =
        validPage(current) && pages.size <= MAX_PREVIOUS_PAGES && pages.all(::validPage)
            && pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
            && pages.lastOrNull()?.let { consecutive(it, current) } != false
    fun remember(pages: List<TranscriptPage>, current: TranscriptPage?, next: TranscriptPage): List<TranscriptPage> {
        if (current == null) { require(next.offset == 0 && validPage(next)); return emptyList() }
        require(validPage(next) && consecutive(current, next) && validChain(pages, current))
        return (pages + current).takeLast(MAX_PREVIOUS_PAGES)
    }
    fun previous(pages: List<TranscriptPage>, current: TranscriptPage): TranscriptPage? {
        val previous = pages.lastOrNull() ?: return null
        require(validChain(pages, current))
        return previous
    }
}
