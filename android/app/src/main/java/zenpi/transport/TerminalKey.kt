package zenpi.transport

/** Fixed terminal bytes only; dispatch still requires the current host-owned instance. */
internal enum class TerminalKey(val label: String, val bytes: String) {
    BACKSPACE("Backspace", "\u007f"),
    DELETE("Delete", "\u001b[3~"),
    HOME("Home", "\u001b[H"),
    END("End", "\u001b[F"),
    PAGE_UP("Page Up", "\u001b[5~"),
    PAGE_DOWN("Page Down", "\u001b[6~"),
    INSERT("Insert", "\u001b[2~"),
    EOF("EOF (Ctrl+D)", "\u0004"),
    SUSPEND("Suspend (Ctrl+Z)", "\u001a"),
    LINE_START("Line start (Ctrl+A)", "\u0001"),
    LINE_END("Line end (Ctrl+E)", "\u0005"),
    REDRAW("Redraw (Ctrl+L)", "\u000c"),
}
