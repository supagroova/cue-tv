package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject

/** One route for phone actions. The observed input wins over a phone's previous selection. */
object Platforms {
    @Volatile private var source = PlatformRules.Source(PlatformRules.Platform.UNKNOWN)
    @Volatile private var observedAt = 0L
    @Volatile private var homeRequest = 0L
    private val actions = Any()
    private val titles = AppleTitleQueue()

    fun observe(front: String?) {
        if (AppleLink.paired()) AppleDiscovery.start()
        val input = front in setOf("com.google.android.tv.inputplayer", "com.android.tv", "com.mediatek.tvinput")
        val overlay = front?.contains("settings") == true || front?.contains("systemui") == true
        val cfg = AppleLink.config()
        val family = cfg?.optString("input_id")?.let { Tv.inputFamily(it) }.orEmpty() +
            cfg?.optJSONArray("input_family")?.let { a -> (0 until a.length()).map { a.getString(it) }.toSet() }.orEmpty()
        val tvInput = if ((input || overlay) && Adb.connected) runCatching { Adb.shell("dumpsys tv_input", 6000) }.getOrNull() else null
        val hdmi = if (input && PlatformRules.mainInput(tvInput) == null && Adb.connected) runCatching { Adb.shell("dumpsys hdmi_control", 6000) }.getOrNull() else null
        source = PlatformRules.classify(front, hdmi, cfg?.optInt("input_port"), tvInput, family)
        observedAt = System.currentTimeMillis()
    }

    fun state(): JSONObject = JSONObject().put("active", source.platform.id).put("label", source.platform.label)
        .put("home_request", homeRequest)
        .put("open_request", titles.current()?.let { JSONObject().put("serial", it.serial).put("service", it.service).put("type", it.type).put("id", it.id) } ?: JSONObject.NULL)
        .put("port", source.port ?: JSONObject.NULL)
        .put("options", JSONArray().put(JSONObject().put("id", "apple").put("name", "Apple TV").put("ready", AppleLink.paired()))
            .put(JSONObject().put("id", "android").put("name", "Android TV").put("ready", true)))

    private fun active(fresh: Boolean = true): PlatformRules.Platform {
        // Preserve the single-platform installation's behaviour until the hybrid is configured.
        if (AppleLink.config() == null) return PlatformRules.Platform.ANDROID
        if (fresh || System.currentTimeMillis() - observedAt > 2500) observe(Tv.frontPackage())
        if (source.platform !in setOf(PlatformRules.Platform.ANDROID, PlatformRules.Platform.APPLE))
            throw ApiError(409, "Choose Apple TV or Android TV before opening this.")
        return source.platform
    }

    fun select(id: String): JSONObject = synchronized(actions) {
        when (id) {
            "android" -> { titles.cancel(); App.openHome() }
            "apple" -> {
                val cfg = AppleLink.config() ?: throw ApiError(503, "Apple TV needs to be connected once during setup.")
                // Authenticate and open the app before switching the panel. A failed connection
                // leaves the current input alone. Selecting an input also works when the box is awake.
                appleHome(wake = true)
                if (!Tv.selectInput(cfg.getString("input_id"))) throw ApiError(502, "The TV could not select Apple TV's HDMI input.")
            }
            else -> throw ApiError(400, "Choose Apple TV or Android TV.")
        }
        // Do not report success optimistically. Allow the real foreground/input to settle.
        repeat(8) {
            observe(Tv.frontPackage())
            if (source.platform.id == id) { Now.push(); return@synchronized state() }
            Thread.sleep(350)
        }
        throw ApiError(504, "The input switch has not been confirmed. Check the TV and try again.")
    }

    fun key(name: String, hold: Boolean): String = synchronized(actions) {
        val code = Keys.resolve(name) ?: throw ApiError(400, "Unknown remote control.")
        // Volume and panel power belong to the television, including when AirPods are connected
        // to the Apple TV. Navigation and media controls belong to the visible platform.
        if (code in setOf("KEYCODE_VOLUME_UP", "KEYCODE_VOLUME_DOWN", "KEYCODE_VOLUME_MUTE", "KEYCODE_POWER", "KEYCODE_TV_INPUT", "KEYCODE_SETTINGS")) return@synchronized Tv.key(name, hold)
        if (active(fresh = code == "KEYCODE_HOME") == PlatformRules.Platform.APPLE) {
            if (code == "KEYCODE_HOME") appleHome() else AppleLink.key(code, hold)
            code
        } else Tv.key(name, hold)
    }

    /** true means handled here; Android's existing app launcher handles the other case. */
    fun launch(target: String): Boolean = synchronized(actions) {
        if (active() != PlatformRules.Platform.APPLE) return@synchronized false
        titles.cancel()
        when (target) {
            "home", BuildConfig.APPLICATION_ID -> appleHome()
            "youtube", "com.google.android.youtube.tv" -> AppleLink.app("com.google.ios.youtube")
            "stremio", "com.stremio.one" -> AppleLink.stremio()
            else -> throw ApiError(409, "This shortcut is not available on Apple TV. Choose Android TV to use it there.")
        }
        true
    }

    fun openTitle(type: String, id: String) {
        Stremio.detailUri(type, id) // validate before opening or queuing anything
        openOnActivePlatform("stremio", type, id) { Stremio.open(type, id) }
    }

    fun openStremioPlayer(uri: String) {
        val ticket = StremioPlayback.ticket(uri)
        try {
            openOnActivePlatform("stremio-player", "stream", ticket) {
                try { Tv.startUri(uri, "com.stremio.one") }
                catch (_: Exception) { throw ApiError(502, "Stremio couldn’t open this source. Try another, or open the episode on the TV.") }
            }
        } finally { StremioPlayback.finish(ticket) }
    }

    fun openYouTube(id: String) {
        val uri = YouTube.watchUri(id)
        openOnActivePlatform("youtube", "video", id) {
            Tv.startUri(uri, "com.google.android.youtube.tv")
        }
    }

    private fun openOnActivePlatform(service: String, type: String, id: String, android: () -> Unit) {
        val name = if (service == "youtube") "YouTube" else "Stremio"
        val request = synchronized(actions) {
            if (active() == PlatformRules.Platform.APPLE) {
                AppleLink.home()
                titles.enqueue(type, id, service).also { Now.push() }
            } else { titles.cancel(); android(); null }
        } ?: return
        // Do not hold the action lock while waiting: Home and a later title can cancel this.
        try {
            if (!request.await(10_000)) throw ApiError(502, "$name could not open this on Apple TV.")
        } catch (e: java.util.concurrent.TimeoutException) {
            throw ApiError(504, "Apple TV did not receive the title. Open TV on Apple TV and try again.")
        } finally { titles.clear(request.serial) }
    }

    fun confirmTitle(serial: Long, opened: Boolean): Boolean = titles.confirm(serial, opened).also { Now.push() }

    fun launchAppleApp(target: String) = synchronized(actions) {
        if (target !in setOf("youtube", "stremio")) throw ApiError(400, "Choose YouTube or Stremio.")
        titles.cancel()
        if (target == "youtube") AppleLink.app("com.google.ios.youtube") else AppleLink.stremio()
    }

    private fun appleHome(wake: Boolean = false) {
        titles.cancel()
        AppleLink.home(wake)
        homeRequest = maxOf(homeRequest + 1, System.currentTimeMillis())
        Now.push()
    }

    fun requireAndroid() {
        if (active() != PlatformRules.Platform.ANDROID) throw ApiError(409, "Use Watch search for Stremio titles on Apple TV, or choose Android TV for this shortcut.")
    }

    fun text(text: String) = synchronized(actions) {
        if (active() == PlatformRules.Platform.APPLE) throw ApiError(409, "Use Watch search to open a title on Apple TV.")
        Tv.text(text)
    }
}
