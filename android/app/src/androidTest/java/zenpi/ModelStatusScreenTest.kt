package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class ModelStatusScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun pushedHostSelectionUpdatesOpenPanelAndResetsStaleDraftWithoutSending() {
        val projection = HostProjection(listOf(RemoteProject("project","Project","session",modelControl=true,reasoningControl=true)),
            "project","session","idle",emptyList(),"1",false,"0")
        val original = RemoteModelStatus("project","session","gpt-current","high",true,"7",null,"128000","8192",listOf(null,"low","high"))
        val state = mutableStateOf(PairingState(phase=PairingPhase.CONNECTED,projection=projection,modelOpen=true,modelStatus=original))
        var writes = 0
        compose.setContent { MaterialTheme { PairingScreen(state.value,{},{},{},onSelectModel={writes++},onSelectReasoning={writes++}) } }
        compose.onNodeWithText("Model ID").performTextReplacement("unsent-draft")
        compose.runOnIdle { state.value=state.value.copy(modelStatus=original.copy(model="gpt-host-selected",reasoningEffort="low",selectionRevision="9")) }
        compose.onAllNodesWithText("gpt-host-selected",useUnmergedTree=true).assertCountEquals(2)
        compose.onNodeWithText("Reasoning: low").assertIsDisplayed()
        compose.onNodeWithText("Apply model").assertIsNotEnabled()
        compose.onNodeWithText("Apply reasoning").assertIsNotEnabled()
        assertEquals(0,writes)
    }
    @Test fun catalogSelectionOnlyChangesDraftUntilApplyAndInvalidModelsStayDisabled() {
        val projection=HostProjection(listOf(RemoteProject("project","Project","session",modelControl=true)),"project","session","idle",emptyList(),"1",false,"0")
        val status=RemoteModelStatus("project","session","gpt-current",null,true,"7",null,"128000","8192",catalog=RemoteModelCatalog("a".repeat(64),0,16,
            listOf(RemoteModelChoice("gpt-next","active",true),RemoteModelChoice("unsupported","beta",false)) +
                (0..13).map { RemoteModelChoice("z-model-%02d".format(it), "active", true) }))
        val state=mutableStateOf(PairingState(phase=PairingPhase.CONNECTED,projection=projection,modelOpen=true,modelStatus=status))
        var selection:String?=null;var page:Int?=null
        compose.setContent {MaterialTheme {PairingScreen(state.value,{},{},{},onSelectModel={selection=it},onModelPage={page=it})}}
        compose.onNodeWithContentDescription("Select host model from catalog").performClick()
        compose.onNodeWithText("unsupported").assertIsNotEnabled()
        compose.onNodeWithText("gpt-next").performClick();assertNull(selection)
        compose.onNodeWithText("Apply model").performClick();assertEquals("gpt-next",selection)
        compose.onNodeWithContentDescription("Next model page").performClick();assertEquals(16,page)
        for (blocked in listOf(
            state.value.copy(modelStatus = status.copy(projectId = "foreign")),
            state.value.copy(modelStatus = status.copy(sessionId = "foreign")),
            state.value.copy(modelStatus = status.copy(selectionRevision = "01")),
            state.value.copy(modelStatus = status.copy(catalog = status.catalog?.copy(nextOffset = 32))),
            state.value.copy(projection = projection.copy(modelStatus = status.copy(selectionRevision = "8"))),
            state.value.copy(projection = projection.copy(modelStatus = status.copy(registryBound = false))),
            state.value.copy(modelError = "Model status unavailable"),
            state.value.copy(scanInProgress = true),
            state.value.copy(resetRequired = true), state.value.copy(resetting = true))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithContentDescription("Next model page").assertIsNotEnabled()
            compose.onNodeWithText("Model ID").assertIsNotEnabled()
            compose.onNodeWithText("Apply model").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = state.value.copy(modelStatus = status, modelError = null, resetRequired = false, resetting = false) }
        compose.runOnIdle{state.value=state.value.copy(phase=PairingPhase.OFFLINE)}
        compose.onNodeWithContentDescription("Next model page").assertIsNotEnabled()
    }
    @Test fun reasoningMenuContainsOnlyHostChoicesAndDefaultAndNoneAreDistinct() {
        val projection = HostProjection(listOf(RemoteProject("project","Project","session",reasoningControl=true)),"project","session","idle",emptyList(),"1",false,"0")
        val state = mutableStateOf(PairingState(phase=PairingPhase.CONNECTED,projection=projection,modelOpen=true,
            modelStatus=RemoteModelStatus("project","session","gpt-fixture","high",true,"7",null,"128000","8192",listOf(null,"none","low","high"))))
        var calls = 0; var effort:String? = "unchanged"
        compose.setContent {MaterialTheme {PairingScreen(state.value,{},{},{},onSelectReasoning={ calls++;effort=it })}}
        compose.onNodeWithText("Apply reasoning").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Select reasoning level").performClick()
        compose.onNodeWithText("ultra").assertDoesNotExist()
        compose.onNodeWithText("Provider default").performClick()
        compose.onNodeWithText("Apply reasoning").performClick();assertEquals(1,calls);assertNull(effort)
        compose.onNodeWithContentDescription("Select reasoning level").performClick()
        compose.onNodeWithText("none").performClick()
        compose.onNodeWithText("Apply reasoning").performClick();assertEquals(2,calls);assertEquals("none",effort)
        val live = state.value
        // Registry choices may change without replacing the displayed revision.
        compose.runOnIdle { state.value = live.copy(projection = projection.copy(modelStatus =
            requireNotNull(live.modelStatus).copy(reasoningChoices = listOf("low", "high")))) }
        compose.onNodeWithText("Apply reasoning").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Select reasoning level").performClick()
        compose.onAllNodesWithText("none").onLast().assertIsNotEnabled()
        compose.onNodeWithText("Provider default").assertIsNotEnabled()
        compose.onNodeWithText("low").performClick()
        compose.onNodeWithText("Apply reasoning").performClick(); assertEquals(3, calls); assertEquals("low", effort)
        for (blocked in listOf(live.copy(modelStatus = live.modelStatus?.copy(sessionId = "foreign")),
            live.copy(modelError = "Model selection unavailable"), live.copy(resetRequired = true),
            live.copy(projection = projection.copy(modelStatus = live.modelStatus?.copy(selectionRevision = "8"))),
            live.copy(projection = projection.copy(modelStatus = live.modelStatus?.copy(registryBound = false))))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithContentDescription("Select reasoning level").assertIsNotEnabled()
            compose.onNodeWithText("Apply reasoning").assertIsNotEnabled()
        }
        compose.runOnIdle {state.value=state.value.copy(phase=PairingPhase.OFFLINE)}
        compose.onNodeWithText("Apply reasoning").assertIsNotEnabled()
    }
    @Test fun modelMutationRequiresLiveControlAndNeverAppliesOffline() {
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", modelControl = true)), "project", "session", "idle", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection, modelOpen = true,
            modelStatus = RemoteModelStatus("project", "session", "gpt-fixture", "high", true, "7", null, "128000", "8192")))
        var selected: String? = null
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onSelectModel = { selected = it }) } }
        compose.onNodeWithText("Apply model").assertIsNotEnabled()
        compose.onNodeWithText("Model ID").performTextReplacement("gpt-next")
        compose.onNodeWithText("Apply model").performClick(); assertEquals("gpt-next", selected)
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithText("Apply model").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.CONNECTED,
            projection = projection.copy(projects = listOf(projection.projects.single().copy(modelControl = false)))) }
        compose.onNodeWithText("Apply model").assertDoesNotExist()
    }
    @Test fun readOnlyModelViewIsAvailableWithoutPromptControlAndOfflineRefreshIsDisabled() {
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session")), "project", "session", "idle", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection, modelOpen = true,
            modelStatus = RemoteModelStatus("project", "session", "gpt-fixture", "high", true, "7", null, "128000", "8192",
                catalog = RemoteModelCatalog("a".repeat(64), 0, 16, listOf(RemoteModelChoice("gpt-next", "active", true)) +
                    (0..14).map { RemoteModelChoice("z-model-%02d".format(it), "active", true) }))))
        var reads = 0; var closed = false
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onModel = { reads++ }, onCloseModel = { closed = true }) } }
        compose.onNodeWithText("gpt-fixture").assertIsDisplayed()
        compose.onNodeWithText("Apply model").assertDoesNotExist()
        compose.onNodeWithText("Context budget: 128000").assertIsDisplayed()
        val oldStatus = requireNotNull(state.value.modelStatus)
        val updated = oldStatus.copy(maxTokens = "32000", reservedOutputTokens = "2048")
        compose.runOnIdle { state.value = state.value.copy(
            modelStatus = ModelStatusCodec.newest(oldStatus, updated, projection)) }
        compose.onNodeWithText("Context budget: 32000").assertIsDisplayed()
        compose.onNodeWithText("Reserved output: 2048").assertIsDisplayed()
        compose.onNodeWithText("Context budget: 128000").assertDoesNotExist()
        assertEquals(oldStatus.catalog, state.value.modelStatus?.catalog)
        compose.onNodeWithContentDescription("Next model page").assertIsEnabled()
        compose.onNodeWithContentDescription("Select host model from catalog").assertIsEnabled()
        compose.onNodeWithContentDescription("Refresh host model").performClick(); assertEquals(1, reads)
        val ready = state.value
        for (changed in listOf(ready.copy(scanInProgress = true), ready.copy(resetting = true), ready.copy(resetRequired = true),
            ready.copy(projection = projection.copy(projects = listOf(projection.projects.single(), projection.projects.single().copy(sessionId = "foreign")))))) {
            compose.runOnIdle { state.value = changed }
            compose.onNodeWithContentDescription("Refresh host model").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next model page").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = ready.copy(modelError = "changed") }
        compose.onNodeWithContentDescription("Refresh host model").assertIsEnabled()
        compose.onNodeWithContentDescription("Next model page").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Refresh host model").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Close host model").performClick(); assertTrue(closed)
    }
}
