package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import androidx.compose.runtime.mutableStateOf
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState

@RunWith(AndroidJUnit4::class)
class UnknownOutcomeScreenTest {
    @get:Rule val compose = createComposeRule()

    @Test fun unknownOutcomeIsVisibleWhileExactRetryRemainsPending() {
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, workspaceFailureCode = "unknown_outcome"))
        compose.setContent { PairingScreen(state.value, onScan = {}, onConfirm = {}, onCancel = {}) }
        compose.onNodeWithTag("workspace-unknown-outcome").assertIsDisplayed()
    }
}
