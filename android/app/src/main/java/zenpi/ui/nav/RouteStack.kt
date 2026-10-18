package zenpi.ui.nav

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable

/** Full-screen secondary pages above the conversation. */
enum class Route { SESSIONS, DIFF, TERMINAL, TASKS }

/**
 * The secondary-page back stack. Sessions and Diff are readers owned by the view model (their open
 * flags may flip without a tap here, e.g. a reader is closed when another one opens), so [sync]
 * mirrors those flags: an opened reader is pushed, a closed one is removed wherever it sits. Terminal
 * and Tasks are pure navigation. [pop] returns the page left, so the caller can close its reader.
 */
@Stable
class RouteStack internal constructor(initial: List<Route>) {
    private val routes = mutableStateListOf<Route>().apply { addAll(initial) }

    val top: Route? get() = routes.lastOrNull()
    val entries: List<Route> get() = routes.toList()

    fun push(route: Route) { routes.remove(route); routes.add(route) }

    fun pop(): Route? = routes.removeLastOrNull()

    /** Mirrors the view model's reader flags; navigation-only pages are left untouched. */
    fun sync(sessionsOpen: Boolean, diffOpen: Boolean) {
        for ((route, open) in listOf(Route.SESSIONS to sessionsOpen, Route.DIFF to diffOpen)) {
            if (!open) routes.remove(route) else if (route !in routes) routes.add(route)
        }
    }

    companion object {
        val saver: Saver<RouteStack, List<String>> = Saver(save = { stack -> stack.routes.map { it.name } },
            restore = { names -> RouteStack(names.map(Route::valueOf)) })
    }
}

/** A back stack scoped to [keys] (host and session): switching either returns to the conversation. */
@Composable
fun rememberRouteStack(vararg keys: Any?): RouteStack = rememberSaveable(*keys, saver = RouteStack.saver) { RouteStack(emptyList()) }
