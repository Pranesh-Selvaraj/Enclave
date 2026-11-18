package com.enclave.app.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class UpdateCheckTest {
    private val installed = UpdateCandidate(
        packageName = "com.enclave.app",
        versionCode = 3,
        signerFingerprints = setOf("release-key"),
    )

    @Test
    fun acceptsANewerBuildOfTheSameAppAndSigner() {
        assertNull(
            updateRefusal(
                installed,
                installed.copy(versionCode = 4),
            ),
        )
    }

    @Test
    fun rejectsAnotherApplication() {
        assertEquals(
            "This update belongs to a different app.",
            updateRefusal(installed, installed.copy(packageName = "com.example.other", versionCode = 4)),
        )
    }

    @Test
    fun rejectsADowngradeOrTheSameBuild() {
        assertEquals(
            "This update is not newer than the installed build.",
            updateRefusal(installed, installed.copy(versionCode = 3)),
        )
        assertEquals(
            "This update is not newer than the installed build.",
            updateRefusal(installed, installed.copy(versionCode = 2)),
        )
    }

    @Test
    fun rejectsADifferentSigningKey() {
        assertEquals(
            "This update is signed by a different key than the installed app.",
            updateRefusal(installed, installed.copy(versionCode = 4, signerFingerprints = setOf("other-key"))),
        )
    }

    @Test
    fun rejectsMissingSignerOrInstalledIdentity() {
        assertEquals(
            "The downloaded update has no signing certificate.",
            updateRefusal(installed, installed.copy(versionCode = 4, signerFingerprints = emptySet())),
        )
        assertEquals(
            "The installed app identity could not be read.",
            updateRefusal(installed.copy(signerFingerprints = emptySet()), installed),
        )
    }
}
