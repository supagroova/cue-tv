package dev.couchtv.ontv

/** Selected input, never inferred from a connected HDMI cable or a box's power state. */
internal object PlatformRules {
    enum class Platform(val id: String, val label: String) {
        ANDROID("android", "Android TV"), APPLE("apple", "Apple TV"), OTHER("other", "Other input"), UNKNOWN("unknown", "Checking input")
    }
    data class Source(val platform: Platform, val port: Int? = null)

    /** AOSP's main session token identifies the selected input, including when its box sleeps. */
    fun mainInput(dump: String?): String? {
        if (dump == null) return null
        val token = Regex("(?m)^\\s*mainSessionToken:\\s*(\\S+)").find(dump)?.groupValues?.get(1) ?: return null
        if (token == "null") return null
        val sessions = dump.indexOf("sessionStateMap:").takeIf { it >= 0 } ?: return null
        val start = dump.indexOf("$token:", sessions).takeIf { it >= 0 } ?: return null
        return dump.substring(start).lineSequence().drop(1).takeWhile { it.startsWith("      ") }
            .firstOrNull { it.trimStart().startsWith("inputId:") }?.substringAfter("inputId:")?.trim()?.takeIf { it.isNotEmpty() }
    }

    fun classify(front: String?, hdmi: String?, applePort: Int?, inputDump: String? = null, appleInputs: Set<String> = emptySet()): Source {
        if (front.isNullOrBlank()) return Source(Platform.UNKNOWN)
        val inputPlayer = front in setOf("com.google.android.tv.inputplayer", "com.android.tv", "com.mediatek.tvinput")
        val overlay = front.contains("systemui") || front.contains("settings")
        // Settings and system overlays do not prove that Android is the selected source.
        if (!inputPlayer && !overlay) return Source(Platform.ANDROID)
        val selected = mainInput(inputDump)
        if (selected != null && appleInputs.isNotEmpty()) return if (selected in appleInputs) Source(Platform.APPLE, applePort) else Source(Platform.OTHER)
        if (overlay) return Source(if (inputDump?.contains(Regex("mainSessionToken:\\s*null")) == true) Platform.ANDROID else Platform.UNKNOWN)
        val path = hdmi?.let { Regex("mActiveRoutingPath:\\s*0x([0-9a-fA-F]{4})").find(it)?.groupValues?.get(1)?.toIntOrNull(16) }
        val port = path?.ushr(12)?.takeIf { it in 1..15 }
        return when {
            port == null -> Source(Platform.UNKNOWN)
            port == applePort -> Source(Platform.APPLE, port)
            else -> Source(Platform.OTHER, port)
        }
    }
}
