package com.enclave.app.ui

/** The identity facts Android needs before it will replace an installed app. */
internal data class UpdateCandidate(
    val packageName: String,
    val versionCode: Long,
    val signerFingerprints: Set<String>,
)

/**
 * Refuse an APK that the package installer would reject, before handing it
 * over. The release signature in the core authenticates the published asset;
 * this check catches the neighbouring problem: a valid APK for another app,
 * a downgrade, or a different signing key. Returns a user-facing reason, or
 * null when the update is acceptable.
 */
internal fun updateRefusal(
    installed: UpdateCandidate,
    candidate: UpdateCandidate,
): String? {
    if (installed.packageName.isBlank() || installed.signerFingerprints.isEmpty()) {
        return "The installed app identity could not be read."
    }
    if (candidate.packageName != installed.packageName) {
        return "This update belongs to a different app."
    }
    if (candidate.signerFingerprints.isEmpty()) {
        return "The downloaded update has no signing certificate."
    }
    if (candidate.signerFingerprints != installed.signerFingerprints) {
        return "This update is signed by a different key than the installed app."
    }
    if (candidate.versionCode <= installed.versionCode) {
        return "This update is not newer than the installed build."
    }
    return null
}
