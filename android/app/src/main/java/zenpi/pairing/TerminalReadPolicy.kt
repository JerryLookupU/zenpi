package zenpi.pairing

import zenpi.transport.TerminalPageCodec

/** Reading host PTY history does not borrow input or resize authority. */
internal object TerminalReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED
        && !state.scanInProgress && !state.resetting && !state.resetRequired && !state.terminalLoading
        && state.pendingWorkspace == null && state.pendingRevoke == null
        && TerminalPageCodec.terminal(state.projection) != null

    fun navigation(state: PairingState): Boolean = browse(state) && state.terminalOpen && state.terminalError == null
        && state.terminalPage?.let {
            TerminalPageCodec.matchesSelection(it, state.projection) && TerminalPageCodec.validChain(state.terminalPrevious, it)
        } == true
}
