package dev.couchtv.ontv

import org.json.JSONObject

/** The Apple TV remote identity is private to this Android app, never returned to a phone. */
object AppleLink {
    private const val STORE = "apple_tv_link"
    private var session: CompanionSession? = null
    private var lastUse = 0L
    internal fun config(): JSONObject? = Store.str(STORE)?.let { runCatching { JSONObject(it) }.getOrNull() }
    fun paired() = config()?.has("credentials") == true
    fun status(): JSONObject = JSONObject().put("paired", paired()).put("name", config()?.optString("name") ?: "Apple TV")

    fun provision(input: JSONObject) {
        val host = input.getString("host")
        val ip = host.split('.').map { it.toIntOrNull() }
        requireApi(ip.size == 4 && ip.all { it != null && it in 0..255 } &&
            (ip[0] == 10 || (ip[0] == 192 && ip[1] == 168) || (ip[0] == 172 && ip[1]!! in 16..31)), "Apple TV needs a home network address.")
        val port = input.optInt("port", 49153); requireApi(port in 1..65535, "Invalid Apple TV port.")
        val inputId = input.getString("input_id"); requireApi(Tv.hasInput(inputId), "Apple TV's input is not available.")
        val inputPort = input.getInt("input_port"); requireApi(inputPort in 1..15, "Invalid HDMI port.")
        val bundle = input.getString("home_bundle"); requireApi(bundle.matches(Regex("[A-Za-z0-9][A-Za-z0-9.-]{2,199}")), "Invalid Apple TV app.")
        val stremio = input.optString("stremio_bundle", "com.stremio.pal")
        requireApi(stremio.matches(Regex("[A-Za-z0-9][A-Za-z0-9.-]{2,199}")), "Invalid Stremio app.")
        val credentials = input.getJSONObject("credentials")
        for (key in listOf("ltsk", "atv_ltpk")) requireApi(credentials.optString(key).matches(Regex("[0-9a-fA-F]{64}")), "Invalid Apple TV pairing.")
        for (key in listOf("client_id", "atv_id")) requireApi(credentials.optString(key).length in 1..128, "Invalid Apple TV identity.")
        val cleanCredentials = JSONObject().also { obj -> listOf("ltsk", "atv_ltpk", "client_id", "atv_id").forEach { obj.put(it, credentials.getString(it)) } }
        val clean = JSONObject().put("host", host).put("port", port).put("input_id", inputId).put("input_port", inputPort)
            .put("input_family", org.json.JSONArray(Tv.inputFamily(inputId).toList()))
            .put("home_bundle", bundle).put("stremio_bundle", stremio).put("name", input.optString("name", "Apple TV").take(50)).put("credentials", cleanCredentials)
        // Verify the provisioned identity before replacing an existing working one. No key, wake,
        // app launch or pairing prompt is sent here.
        connection(clean) { }
        synchronized(this) { session?.close(); session = null; Store.putStr(STORE, clean.toString()) }
    }
    private fun requireApi(ok: Boolean, message: String) { if (!ok) throw ApiError(400, message) }
    private fun connection(cfg: JSONObject = config() ?: throw ApiError(503, "Apple TV needs to be connected once during setup."), action: (CompanionSession) -> Unit) {
        try {
            val c = cfg.getJSONObject("credentials")
            val cred = CompanionSession.Credentials(c.getString("client_id"), CompanionWire.unhex(c.getString("ltsk")), c.getString("atv_id"), CompanionWire.unhex(c.getString("atv_ltpk")))
            try { CompanionSession(cfg.getString("host"), cfg.getInt("port")).use { it.verify(cred); action(it) } }
            finally { cred.privateKey.fill(0) }
        } catch (e: ApiError) { throw e }
        catch (e: Exception) { throw ApiError(503, "Apple TV did not complete the action. Check it is on the home network and paired.") }
    }
    @Synchronized private fun action(block: (CompanionSession) -> Unit) {
        val cfg = config() ?: throw ApiError(503, "Apple TV needs to be connected once during setup.")
        if (System.currentTimeMillis() - lastUse > 25_000) { session?.close(); session = null }
        try {
            if (session == null) {
                val c = cfg.getJSONObject("credentials")
                val cred = CompanionSession.Credentials(c.getString("client_id"), CompanionWire.unhex(c.getString("ltsk")), c.getString("atv_id"), CompanionWire.unhex(c.getString("atv_ltpk")))
                val endpoint = AppleDiscovery.endpoint(cfg.optString("name"), cfg.getString("host"), cfg.getInt("port"))
                val link = CompanionSession(endpoint.host, endpoint.port)
                try { link.verify(cred); link.start(); session = link }
                catch (e: Exception) { link.close(); throw e }
                finally { cred.privateKey.fill(0) }
            }
            block(session!!); lastUse = System.currentTimeMillis()
        } catch (e: Exception) {
            session?.close(); session = null
            // Never retry a key or launch automatically: it might already have been received.
            throw ApiError(503, "Apple TV did not complete the action. Check it is on the home network and paired.")
        }
    }
    fun home(wake: Boolean = false) {
        val bundle = config()?.getString("home_bundle") ?: throw ApiError(503, "Apple TV needs setup first.")
        action { it.prepareForApp(wake); it.app(bundle) }
    }
    fun app(bundle: String) = action { it.prepareForApp(); it.app(bundle) }
    fun stremio() = app(config()?.optString("stremio_bundle", "com.stremio.pal") ?: throw ApiError(503, "Apple TV needs setup first."))
    fun url(uri: String) = action { it.prepareForApp(); it.url(uri) }
    // Apple TV's physical TV button is distinct from our Cue TV Home shortcut.
    fun tvButton(hold: Boolean) = action { it.key(7, hold) }
    fun key(androidCode: String, hold: Boolean) {
        val code = mapOf("KEYCODE_DPAD_UP" to 1, "KEYCODE_DPAD_DOWN" to 2, "KEYCODE_DPAD_LEFT" to 3, "KEYCODE_DPAD_RIGHT" to 4,
            "KEYCODE_BACK" to 5, "KEYCODE_DPAD_CENTER" to 6, "KEYCODE_MEDIA_PLAY_PAUSE" to 14).get(androidCode)
            ?: throw ApiError(409, "This control is not available on Apple TV.")
        action { it.key(code, hold) }
    }
}
