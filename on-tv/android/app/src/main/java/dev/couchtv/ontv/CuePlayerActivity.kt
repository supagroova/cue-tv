package dev.couchtv.ontv

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.KeyEvent
import android.view.View
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.TextView
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.session.MediaSession
import androidx.media3.ui.PlayerView
import org.json.JSONObject
import java.net.URI
import java.util.UUID
import java.util.concurrent.Executors

/** Foreground-only video playback. The streaming address is never an exported intent or log. */
@androidx.annotation.OptIn(markerClass = [UnstableApi::class])
class CuePlayerActivity : Activity() {
    private var player: ExoPlayer? = null
    private var session: MediaSession? = null
    private var media: JSONObject? = null
    private val handler = Handler(Looper.getMainLooper())
    private var played = false
    private var released = false
    private val progress = object : Runnable {
        override fun run() { save(); if (!released) handler.postDelayed(this, 5000) }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val payload = synchronized(pending) { pending.remove(intent.getStringExtra("cue-playback")) }
        val uri = payload?.optString("url")?.let { runCatching { URI(it) }.getOrNull() }
        if (payload == null || uri?.scheme != "https" || uri.host != "torrentio.strem.fun" || uri.userInfo != null ||
            !(uri.path.startsWith("/realdebrid/") || uri.path.startsWith("/resolve/realdebrid/"))) { finish(); return }
        media = payload
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        @Suppress("DEPRECATION")
        window.decorView.systemUiVisibility = View.SYSTEM_UI_FLAG_FULLSCREEN or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
        androidx.media3.common.util.Log.setLogLevel(androidx.media3.common.util.Log.LOG_LEVEL_OFF)
        val root = FrameLayout(this).apply { setBackgroundColor(0xff000000.toInt()) }
        val transport = DefaultHttpDataSource.Factory().setAllowCrossProtocolRedirects(false)
        val video = ExoPlayer.Builder(this).setMediaSourceFactory(DefaultMediaSourceFactory(transport)).build()
        player = video
        val controls = PlayerView(this).apply {
            player = video; useController = true; controllerShowTimeoutMs = 5000
            setShowSubtitleButton(true)
            setShowBuffering(PlayerView.SHOW_BUFFERING_ALWAYS)
        }
        root.addView(controls, FrameLayout.LayoutParams(-1, -1))
        val error = TextView(this).apply { textSize = 23f; setTextColor(0xffffffff.toInt()); setPadding(40, 30, 40, 30); setBackgroundColor(0xdd101010.toInt()); visibility = View.GONE }
        root.addView(error, FrameLayout.LayoutParams(-1, -2, Gravity.TOP))
        setContentView(root); controls.requestFocus()
        video.setAudioAttributes(AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MOVIE).build(), true)
        video.addListener(object : Player.Listener {
            override fun onIsPlayingChanged(isPlaying: Boolean) { if (isPlaying) played = true; save() }
            override fun onPlaybackStateChanged(state: Int) { if (state == Player.STATE_ENDED) save("ended") }
            override fun onPlayerError(exception: PlaybackException) { error.text = "This source couldn’t play. Press Back and choose another source."; error.visibility = View.VISIBLE }
        })
        val metadata = MediaMetadata.Builder().setTitle(payload.optString("title")).setSubtitle(payload.optString("episodeTitle")).build()
        video.setMediaItem(MediaItem.Builder().setUri(uri.toString()).setMediaMetadata(metadata).build(), payload.optLong("resume", 0))
        session = MediaSession.Builder(this, video).build()
        video.prepare(); video.play(); handler.postDelayed(progress, 5000)
    }
    private fun save(state: String? = null) {
        val value = media ?: return; val video = player ?: return
        if (!played) return
        val body = JSONObject().put("playback", value.getString("playback")).put("position", video.currentPosition.coerceAtLeast(0))
            .put("duration", video.duration.coerceAtLeast(0)).put("state", state ?: if (video.isPlaying) "playing" else "paused")
        writer.execute { runCatching { CuePlayback.progress(body) } }
    }
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        // PlayerView consumes Back to hide its controls. Cue TV's Back always returns to browsing.
        if (event.keyCode == KeyEvent.KEYCODE_BACK) {
            if (event.action == KeyEvent.ACTION_UP && !event.isCanceled) finish()
            return true
        }
        return super.dispatchKeyEvent(event)
    }
    override fun onPause() { save(); player?.pause(); super.onPause() }
    override fun onStop() { releasePlayer(); super.onStop(); if (!isFinishing) finish() }
    override fun onDestroy() { releasePlayer(); super.onDestroy() }
    private fun releasePlayer() {
        if (released) return
        save("stopped"); released = true; handler.removeCallbacks(progress)
        session?.release(); session = null; player?.release(); player = null
        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
    companion object {
        private val pending = LinkedHashMap<String, JSONObject>()
        private val writer = Executors.newSingleThreadExecutor { r -> Thread(r, "cue-progress").apply { isDaemon = true } }
        fun open(media: JSONObject) {
            val id = UUID.randomUUID().toString()
            synchronized(pending) { pending.clear(); pending[id] = JSONObject(media.toString()) }
            Tv.start(Intent(Store.context(), CuePlayerActivity::class.java).putExtra("cue-playback", id).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }
}
