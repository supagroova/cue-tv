package dev.couchtv.ontv

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import org.bouncycastle.util.encoders.Base64
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** The phone sees public connection state and list display data, never Google credentials. */
object YouTubeAccount {
    private val engine by lazy { YouTubeAccounts(AndroidYouTubeVault(), GoogleYouTubeTransport()) }
    private fun <T> safely(block: () -> T): T = try { block() } catch (e: ApiError) { throw e }
        catch (_: Exception) { throw ApiError(502, "Couldn’t read the YouTube connection. Try again or reconnect this phone.") }
    fun configure(body: JSONObject) = safely { engine.configure(body) }
    fun status(id: String) = safely { engine.status(id) }
    fun begin(id: String) = safely { engine.begin(id) }
    fun poll(id: String) = safely { engine.poll(id) }
    fun cancel(id: String) = safely { engine.cancel(id) }
    fun disconnect(id: String) = safely { engine.disconnect(id) }
    fun library(id: String, kind: String, item: String, page: String) = safely { engine.library(id, kind, item, page) }
}

internal class AndroidYouTubeVault : YouTubeVault {
    private val alias = "couchtv.youtube.v1"
    private val box = YouTubeSecretBox { key() }
    @Synchronized private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(alias, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).setKeySize(256).build())
        }.generateKey()
    }
    override fun get(key: String): String? {
        val raw = Store.str("youtube-v1-$key") ?: return null
        return box.open(key, raw)
    }
    override fun put(key: String, value: String?) {
        if (value == null) { Store.putSecret("youtube-v1-$key", null); return }
        Store.putSecret("youtube-v1-$key", box.seal(key, value))
    }
}

internal class YouTubeSecretBox(private val key: () -> SecretKey) {
    fun seal(record: String, value: String): String {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding"); cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(record.toByteArray(Charsets.UTF_8))
        val data = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        return JSONObject().put("v", 1).put("iv", Base64.toBase64String(cipher.iv)).put("data", Base64.toBase64String(data)).toString()
    }
    fun open(record: String, raw: String): String {
        val envelope = JSONObject(raw)
        require(envelope.getInt("v") == 1)
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, Base64.decode(envelope.getString("iv"))))
        cipher.updateAAD(record.toByteArray(Charsets.UTF_8))
        return String(cipher.doFinal(Base64.decode(envelope.getString("data"))), Charsets.UTF_8)
    }
}

internal class GoogleYouTubeTransport : YouTubeTransport {
    override fun post(endpoint: String, fields: Map<String, String>): YouTubeReply {
        require(endpoint in setOf("device/code", "token"))
        return request("https://oauth2.googleapis.com/$endpoint", fields.entries.joinToString("&") { encode(it.key) + "=" + encode(it.value) }, null)
    }
    override fun get(resource: String, query: Map<String, String>, token: String): YouTubeReply {
        require(resource in setOf("channels", "subscriptions", "playlists", "playlistItems", "videos"))
        return request("https://www.googleapis.com/youtube/v3/$resource?" + query.entries.joinToString("&") { encode(it.key) + "=" + encode(it.value) }, null, token)
    }
    private fun encode(raw: String) = URLEncoder.encode(raw, "UTF-8")
    private fun request(address: String, body: String?, token: String?): YouTubeReply {
        val connection = URL(address).openConnection() as HttpURLConnection
        try {
            connection.connectTimeout = 8000; connection.readTimeout = 12000; connection.instanceFollowRedirects = false
            connection.setRequestProperty("Accept", "application/json")
            if (token != null) connection.setRequestProperty("Authorization", "Bearer $token")
            if (body != null) {
                connection.requestMethod = "POST"; connection.doOutput = true
                connection.setRequestProperty("Content-Type", "application/x-www-form-urlencoded")
                connection.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            }
            val status = connection.responseCode
            val stream = if (status in 200..299) connection.inputStream else connection.errorStream
            val data = stream?.use {
                val out = java.io.ByteArrayOutputStream(); val buffer = ByteArray(8192)
                while (true) {
                    val n = it.read(buffer); if (n < 0) break
                    if (out.size() + n > 2 * 1024 * 1024) throw ApiError(502, "YouTube returned too much data. Try a smaller list.")
                    out.write(buffer, 0, n)
                }
                out.toString("UTF-8")
            }.orEmpty()
            return YouTubeReply(status, if (data.isEmpty()) JSONObject() else JSONObject(data))
        } catch (e: ApiError) { throw e }
        catch (_: Exception) { throw ApiError(502, "Couldn’t reach Google. Check the TV’s internet connection and try again.") }
        finally { connection.disconnect() }
    }
}
