package dev.couchtv.ontv

import org.junit.Assert.*
import org.junit.Test

class HomeRestoreTest {
    private val google = "com.google.android.apps.tv.launcherx"
    private val home = "$google.home.HomeActivity"
    private val homes = setOf(home, "com.android.tv.settings.HomeActivity", "com.google.android.tungsten.setupwraith.StartupActivity")
    private fun resumed(component: String) = "topResumedActivity=ActivityRecord{aabcc9e u0 $component t2}"

    @Test fun googleHomeIsRestoredWithEitherComponentNotation() {
        assertTrue(HomeRestore.shouldRestore(resumed("$google/$home"), homes))
        assertTrue(HomeRestore.shouldRestore(resumed("$google/.home.HomeActivity"), homes))
    }

    @Test fun inputsAppsAndSetupStayInFrontEvenIfTheyCanHandleHome() {
        for (component in listOf(
            "com.google.android.tv.inputplayer/.player.PassthroughPlayerActivity",
            "com.stremio.one/com.stremio.tv.MainActivity",
            "com.google.android.youtube.tv/.MainActivity",
            "com.android.tv.settings/.HomeActivity",
            "com.google.android.tungsten.setupwraith/.StartupActivity",
            "$google/.settings.SettingsActivity"
        )) assertFalse(component, HomeRestore.shouldRestore(resumed(component), homes))
    }

    @Test fun unknownOrUnresolvedScreensAreNotReplaced() {
        assertFalse(HomeRestore.shouldRestore("", homes))
        assertFalse(HomeRestore.shouldRestore("mCurrentFocus=null", homes))
        assertFalse(HomeRestore.shouldRestore(resumed("$google/$home"), emptySet()))
    }
}
