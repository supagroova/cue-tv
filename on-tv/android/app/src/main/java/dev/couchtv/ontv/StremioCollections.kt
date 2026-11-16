package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.Callable
import java.util.concurrent.ExecutorCompletionService
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/** Catalogue-only requests. Never installs an add-on or changes the account's stream providers. */
internal class StremioCollectionsEngine(private val fetch: (String) -> JSONObject, private val now: () -> Long = System::currentTimeMillis) {
    private data class Cached(val value: JSONObject, val at: Long)
    private val cache = LinkedHashMap<String, Cached>()
    internal data class Shelf(val service: String, val name: String, val kind: String, val label: String, val type: String) {
        val id get() = "$service:$kind:$type"
        fun title() = "$label ${if (type == "series") "TV Shows" else "Films"} on $name"
    }
    internal fun shelves(): List<Shelf> = listOf("nfx" to "Netflix", "prv" to "Prime Video", "atp" to "Apple TV+").flatMap { (service, name) ->
        listOf("tnd" to "Trending", "pop" to "Popular", "new" to "New").flatMap { (kind, label) ->
            listOf("series", "movie").map { Shelf(service, name, kind, label, it) }
        }
    }
    internal fun address(country: String, shelf: Shelf): String {
        require(country.matches(Regex("[A-Z]{2}")))
        require(shelf in shelves())
        return "$BASE/${country}_en_nfx_prv_atp/catalog/${shelf.type}/${country}_en_jw_${shelf.kind}_${shelf.service}.json"
    }
    @Synchronized fun get(country: String): JSONObject {
        if (!country.matches(Regex("[A-Z]{2}"))) throw ApiError(400, "Choose a valid country.")
        val old = cache[country]
        if (old != null && now() - old.at < 30 * 60_000) return JSONObject(old.value.toString())
        val completion = ExecutorCompletionService<Pair<Int, JSONObject?>>(pool)
        val wanted = shelves()
        val futures = wanted.mapIndexed { index, shelf -> completion.submit(Callable {
            index to runCatching {
                val data = fetch(address(country, shelf)).optJSONArray("metas") ?: JSONArray()
                val items = JSONArray(); val seen = HashSet<String>()
                for (i in 0 until minOf(data.length(), 50)) {
                    val meta = data.optJSONObject(i) ?: continue
                    if (meta.optString("type").isBlank()) meta.put("type", shelf.type)
                    val item = Stremio.cleanMeta(meta) ?: continue
                    if (item.optString("type") == shelf.type && seen.add(item.getString("id"))) items.put(item)
                    if (items.length() >= 20) break
                }
                JSONObject().put("id", shelf.id).put("title", shelf.title()).put("service", shelf.name)
                    .put("kind", shelf.kind).put("type", shelf.type).put("country", country)
                    .put("source", "OmniCatalogs").put("items", items)
            }.getOrNull()
        }) }
        val results = LinkedHashMap<Int, JSONObject>()
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(22)
        try {
            repeat(wanted.size) {
                val left = deadline - System.nanoTime()
                if (left > 0) completion.poll(left, TimeUnit.NANOSECONDS)?.get()?.let { (i, value) -> if (value != null) results[i] = value }
            }
        } finally { futures.forEach { if (!it.isDone) it.cancel(true) } }
        // Preserve a previously good individual shelf during an upstream outage. Never fill one
        // category with a different ranking just to make a row appear.
        val stale = old?.value?.optJSONArray("collections") ?: JSONArray()
        val rows = JSONArray()
        wanted.forEachIndexed { index, shelf ->
            val fresh = results[index]
            val previous = (0 until stale.length()).mapNotNull { stale.optJSONObject(it) }.firstOrNull { it.optString("id") == shelf.id }
            (fresh ?: previous?.let { JSONObject(it.toString()).put("stale", true) })?.let(rows::put)
        }
        val result = JSONObject().put("collections", rows).put("country", country).put("source", "OmniCatalogs")
            .put("failed", wanted.size - results.size).put("updated", now())
        if (results.isNotEmpty()) { while (cache.size >= 8) cache.remove(cache.keys.first()); cache[country] = Cached(result, now()) }
        if (rows.length() == 0) throw ApiError(502, "The service lists couldn’t load. Try again.")
        return result
    }
    companion object {
        const val BASE = "https://5cfe2edf73d5-omnicatalogs.baby-beamup.club"
        private val pool = Executors.newFixedThreadPool(4) { runnable -> Thread(runnable, "cue-catalogues").apply { isDaemon = true } }
    }
}

internal object StremioCollections {
    private val engine = StremioCollectionsEngine(StremioResourceHTTP::get)
    fun get() = engine.get(Store.country())
}
