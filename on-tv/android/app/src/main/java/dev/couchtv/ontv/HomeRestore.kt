package dev.couchtv.ontv

/** Only replace an actual Google home at boot, never an input, app, settings or setup screen. */
internal object HomeRestore {
    private val googleHomes = setOf("com.google.android.apps.tv.launcherx", "com.google.android.tvlauncher")
    private val resumed = Regex("ActivityRecord\\{\\S+ u[0-9]+ ([^/\\s]+)/([^\\s}]+)")

    fun component(dump: String): Pair<String, String>? {
        val m = resumed.find(dump) ?: return null
        val pkg = m.groupValues[1]
        val cls = m.groupValues[2].let { if (it.startsWith('.')) pkg + it else it }
        return pkg to cls
    }

    fun shouldRestore(dump: String, resolvedHomes: Set<String>): Boolean {
        val (pkg, cls) = component(dump) ?: return false
        return pkg in googleHomes && cls in resolvedHomes
    }
}
