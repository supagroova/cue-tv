package dev.couchtv.ontv

import org.junit.Assert.*
import org.junit.Test

class PlatformRulesTest {
    private val appleInput = "com.mediatek.tvinput/.hdmi.HDMIInputService/HDMI300004"
    private fun session(id: String) = """
  sessionStateMap: ITvInputSession -> SessionState
    android.os.Binder@b0e3c24: SessionState
      inputId: $id
      sessionId: fixture
  mainSessionToken: android.os.Binder@b0e3c24
"""
    private fun source(front: String?, path: String?, port: Int? = 3) = PlatformRules.classify(front, path?.let { "mActiveRoutingPath: $it" }, port)
    @Test fun nativeYouTubeWinsOverStaleAppleCEC() {
        assertEquals(PlatformRules.Platform.ANDROID, source("com.google.android.youtube.tv", "0x3000").platform)
    }
    @Test fun nativeHomeDoesNotUseConnectedDeviceAsSelectedInput() {
        assertEquals(PlatformRules.Platform.ANDROID, source("dev.couchtv.ontv", "0x3000").platform)
    }
    @Test fun selectedApplePort() {
        val actual = source("com.google.android.tv.inputplayer", "0x3000")
        assertEquals(PlatformRules.Platform.APPLE, actual.platform); assertEquals(3, actual.port)
    }
    @Test fun anotherPortDoesNotOpenAppleOrAndroidApps() {
        assertEquals(PlatformRules.Platform.OTHER, source("com.google.android.tv.inputplayer", "0x2000").platform)
    }
    @Test fun missingOrInternalRoutingIsUnknown() {
        for (path in listOf(null, "0x0000", "invalid")) assertEquals(PlatformRules.Platform.UNKNOWN, source("com.google.android.tv.inputplayer", path).platform)
    }
    @Test fun settingsOverlayDoesNotSwitchPlatforms() {
        assertEquals(PlatformRules.Platform.UNKNOWN, source("com.android.tv.settings", "0x3000").platform)
    }
    @Test fun missingForegroundDoesNotGuess() {
        assertEquals(PlatformRules.Platform.UNKNOWN, source(null, "0x3000").platform)
    }
    @Test fun unconfiguredPortDoesNotGuess() {
        assertEquals(PlatformRules.Platform.OTHER, source("com.google.android.tv.inputplayer", "0x3000", null).platform)
    }
    @Test fun realMainSessionIdentifiesAppleEvenWhenCECIsInactive() {
        assertEquals(appleInput, PlatformRules.mainInput(session(appleInput)))
        assertEquals(PlatformRules.Platform.APPLE, PlatformRules.classify("com.google.android.tv.inputplayer", "mActiveRoutingPath: 0x0000", 3, session(appleInput), setOf(appleInput)).platform)
    }
    @Test fun anotherMainSessionWinsOverStaleAppleRouting() {
        assertEquals(PlatformRules.Platform.OTHER, PlatformRules.classify("com.google.android.tv.inputplayer", "mActiveRoutingPath: 0x3000", 3, session("another-input"), setOf(appleInput)).platform)
    }
    @Test fun nativeVideoWinsOverRetainedHDMISession() {
        assertEquals(PlatformRules.Platform.ANDROID, PlatformRules.classify("com.google.android.youtube.tv", null, 3, session(appleInput), setOf(appleInput)).platform)
    }
    @Test fun settingsOverlayKeepsUnderlyingInput() {
        assertEquals(PlatformRules.Platform.APPLE, PlatformRules.classify("com.android.tv.settings", null, 3, session(appleInput), setOf(appleInput)).platform)
        assertEquals(PlatformRules.Platform.ANDROID, PlatformRules.classify("com.android.tv.settings", null, 3, "mainSessionToken: null", setOf(appleInput)).platform)
    }
}
