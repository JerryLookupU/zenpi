package zenpi

import android.content.Intent
import android.net.Uri
import androidx.compose.ui.test.junit4.AndroidComposeTestRule
import androidx.test.ext.junit.rules.ActivityScenarioRule
import androidx.lifecycle.ViewModelProvider
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.runner.lifecycle.ActivityLifecycleMonitorRegistry
import androidx.test.runner.lifecycle.Stage
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import java.util.Base64

class PairingActivityIntentTest {
    // ActivityScenario filters lifecycle callbacks by its launch intent. Start
    // with the sanitized external-link shape so later VIEW deliveries remain tracked.
    @get:Rule val compose = AndroidComposeTestRule(
        ActivityScenarioRule<MainActivity>(Intent(Intent.ACTION_VIEW).apply {
            setClass(InstrumentationRegistry.getInstrumentation().targetContext, MainActivity::class.java)
            addCategory(Intent.CATEGORY_BROWSABLE)
        })
    ) { rule ->
        lateinit var activity: MainActivity
        rule.scenario.onActivity { activity = it }
        activity
    }

    private fun invitation(value: Byte): String {
        val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32) { value })
        return "zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${"a".repeat(64)}&token=$token&expires=${System.currentTimeMillis()/1000 + 120}"
    }

    @Test fun externalPairingIntentsReuseTheActivityAndCannotBypassAnInFlightRegistration() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val original = compose.activity
        val model = ViewModelProvider(original)[PairingViewModel::class.java]
        val metadata = PairingViewModel::class.java.getDeclaredField("metadataLoaded").apply { isAccessible = true }
        compose.waitUntil(5000) { metadata.getBoolean(model) }
        @Suppress("UNCHECKED_CAST")
        val state = PairingViewModel::class.java.getDeclaredField("mutable").apply { isAccessible = true }
            .get(model) as MutableStateFlow<PairingState>
        val generation = PairingViewModel::class.java.getDeclaredField("generation").apply { isAccessible = true }
        val retained = model.state.value
        val first = invitation(1)
        val replacement = invitation(2)
        fun resumed(): MainActivity? {
            var result: MainActivity? = null
            instrumentation.runOnMainSync {
                result = ActivityLifecycleMonitorRegistry.getInstance().getActivitiesInStage(Stage.RESUMED)
                    .filterIsInstance<MainActivity>().singleOrNull()
            }
            return result
        }
        fun deliver(uri: String, flags: Int = 0) {
            val delivery = System.nanoTime()
            // No CLEAR_TOP/SINGLE_TOP flag: the manifest itself must enforce ownership.
            instrumentation.runOnMainSync {
                original.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(uri))
                    .addCategory(Intent.CATEGORY_BROWSABLE).addFlags(flags)
                    .putExtra("zenpi.fixture.delivery", delivery))
            }
            instrumentation.waitForIdleSync()
            compose.waitUntil(5000) {
                val active = resumed()
                active != null && (active !== original || original.intent.getLongExtra("zenpi.fixture.delivery", 0) == delivery)
            }
            instrumentation.runOnMainSync {
                val monitor = ActivityLifecycleMonitorRegistry.getInstance()
                val instances = listOf(Stage.CREATED, Stage.STARTED, Stage.RESUMED, Stage.PAUSED, Stage.STOPPED)
                    .flatMap { monitor.getActivitiesInStage(it) }.filterIsInstance<MainActivity>().distinct()
                assertEquals("External QR links must not create a parallel pairing owner", listOf(original), instances)
                assertSame(original, monitor.getActivitiesInStage(Stage.RESUMED).filterIsInstance<MainActivity>().single())
                assertSame(model, ViewModelProvider(original)[PairingViewModel::class.java])
                assertNull("The task intent must not retain the one-use URI", original.intent.data)
                assertEquals(Intent.ACTION_VIEW, original.intent.action)
            }
        }
        try {
            compose.runOnUiThread {
                assertNull("Entry fixture requires an unpaired owned emulator", retained.host)
                assertNull(retained.pendingWorkspace); assertNull(retained.pendingRevoke)
                state.value = PairingState()
            }
            deliver(first)
            compose.runOnUiThread {
                assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
                assertEquals(PairingInvite.parse(first).safetyCode, model.state.value.invite?.safetyCode)
                // Hold only the state admission boundary, not a fabricated network success.
                state.value = state.value.copy(phase = PairingPhase.CONNECTING)
            }
            val pending = model.state.value
            val beforeGeneration = generation.getInt(model)
            deliver(replacement)
            compose.runOnUiThread {
                assertEquals(pending, model.state.value)
                assertEquals(beforeGeneration, generation.getInt(model))
            }
            deliver(first, Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_NEW_DOCUMENT or Intent.FLAG_ACTIVITY_MULTIPLE_TASK)
            compose.runOnUiThread {
                assertEquals(pending, model.state.value)
                assertEquals(beforeGeneration, generation.getInt(model))
                state.value = pending.copy(phase = PairingPhase.CONFIRMING)
            }
            deliver(replacement)
            compose.runOnUiThread {
                assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
                assertEquals(PairingInvite.parse(replacement).safetyCode, model.state.value.invite?.safetyCode)
                assertTrue(generation.getInt(model) > beforeGeneration)
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "external_pair_intent_single_owner_verified")
            })
            compose.runOnUiThread { model.cancel() }
            compose.waitUntil(5000) { metadata.getBoolean(model) && model.state.value.invite == null }
            val cancelledGeneration = generation.getInt(model)
            compose.activityRule.scenario.recreate()
            compose.activityRule.scenario.onActivity { recreated ->
                assertNotSame(original, recreated)
                assertNull(recreated.intent.data)
                assertSame(model, ViewModelProvider(recreated)[PairingViewModel::class.java])
                assertNull(model.state.value.invite)
                assertEquals(cancelledGeneration, generation.getInt(model))
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "external_pair_intent_consumed_recreation_verified")
            })
        } finally {
            compose.runOnUiThread { model.setForeground(false); state.value = retained }
        }
    }
}
