package com.enclave.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import androidx.glance.appwidget.updateAll
import uniffi.core_api.FfiCore
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Widget data cache — the chosen privacy design: widgets can render while the
 * vault is locked, but only for notes the user explicitly shared, and only
 * through a cache encrypted with an Android Keystore (hardware-backed where
 * available) key. The vault master key never leaves the Rust core.
 *
 * Cache contents are limited to what a widget needs: title, labels, updated
 * time, checklist items and text. It is rebuilt on unlock, on note saves for
 * shared notes, and whenever the per-note widget switch changes.
 */
internal object WidgetStore {
    private const val KEY_ALIAS = "enclave-widget-cache-key"
    private const val CACHE_FILE = "widget-cache.enc"
    private const val IV_LENGTH = 12
    private const val GCM_TAG_BITS = 128

    data class WidgetNote(
        val id: String,
        val title: String,
        val tags: List<String>,
        val updatedAt: String,
        val checklist: List<TaskItem>,
        val text: String,
    )

    // Plaintext state dials — booleans only, no note data.
    private const val STATE_PREFS = "enclave_widget_state"
    private const val KEY_UNLOCKED = "vault_unlocked"
    private const val KEY_HIDE_LOCKED = "hide_when_locked"

    /** Lock state marker so widgets can react while the app is not running. */
    fun setVaultUnlocked(context: Context, unlocked: Boolean) {
        context.getSharedPreferences(STATE_PREFS, Context.MODE_PRIVATE)
            .edit().putBoolean(KEY_UNLOCKED, unlocked).apply()
    }

    fun vaultUnlocked(context: Context): Boolean =
        context.getSharedPreferences(STATE_PREFS, Context.MODE_PRIVATE)
            .getBoolean(KEY_UNLOCKED, false)

    fun setHideWhenLocked(context: Context, hide: Boolean) {
        context.getSharedPreferences(STATE_PREFS, Context.MODE_PRIVATE)
            .edit().putBoolean(KEY_HIDE_LOCKED, hide).apply()
    }

    fun hideWhenLocked(context: Context): Boolean =
        context.getSharedPreferences(STATE_PREFS, Context.MODE_PRIVATE)
            .getBoolean(KEY_HIDE_LOCKED, false)

    /** True when widgets must blank out: the dial is on and the vault is locked. */
    fun shouldHide(context: Context): Boolean = hideWhenLocked(context) && !vaultUnlocked(context)

    private const val PIN_PREFS = "enclave_widget_pin"
    private const val PIN_KEY = "pending_doc_id"

    /**
     * "Pin this note" records the note before asking the launcher to pin a
     * widget; the new instance binds it on first render. Some launchers do not
     * deliver the pin-success callback with a usable appWidgetId, so this
     * handoff is the reliable path (one in-flight pin at a time).
     */
    fun setPendingPin(context: Context, docId: String) {
        context.getSharedPreferences(PIN_PREFS, Context.MODE_PRIVATE)
            .edit().putString(PIN_KEY, docId).apply()
    }

    /** Reads and clears the pending pin (called by a widget's first render). */
    fun consumePendingPin(context: Context): String? {
        val prefs = context.getSharedPreferences(PIN_PREFS, Context.MODE_PRIVATE)
        val value = prefs.getString(PIN_KEY, null) ?: return null
        prefs.edit().remove(PIN_KEY).apply()
        return value
    }

    /** Shared notes, decrypted for a widget render. Empty on any failure. */
    fun notes(context: Context): List<WidgetNote> = try {
        readCache(context)?.let(::parse) ?: emptyList()
    } catch (_: Exception) {
        emptyList()
    }

    /**
     * Rebuild the cache from notes that opted into widgets. Never throws: a
     * lock racing the refresh must keep the previous cache (and the app alive).
     */
    fun refresh(context: Context, core: FfiCore) {
        try {
            refreshInner(context, core)
        } catch (e: Exception) {
            android.util.Log.w("EnclaveWidgets", "cache refresh skipped: " + e.message)
        }
    }

    private fun refreshInner(context: Context, core: FfiCore) {
        val notes = core.listDocuments().mapNotNull { doc ->
            val blocks = core.getBlocks(doc.id)
            val widget = blocks.firstOrNull { it.blockType == "widget" }
            if (!widgetBlockEnabled(widget?.contentJson)) return@mapNotNull null
            val docJson = blocks.firstOrNull { it.blockType == "doc" }?.contentJson
            WidgetNote(
                id = doc.id,
                title = doc.title,
                tags = tagsOf(blocks.firstOrNull { it.blockType == "tags" }?.contentJson),
                updatedAt = doc.updatedAt,
                checklist = docChecklist(docJson),
                text = docBodyText(docJson),
            )
        }
        writeCache(context, serialize(notes))
    }

    /** Serializes refresh+update across rapid toggles/saves. */
    private val updateMutex = kotlinx.coroutines.sync.Mutex()

    /**
     * Re-render every widget instance from the existing cache (no core access).
     * Used when the vault locks — showing the placeholder needs a render, not
     * a vault read.
     */
    suspend fun updateAll(context: Context) {
        updateMutex.withLock { renderAll(context) }
    }

    private suspend fun renderAll(context: Context) {
        NoteListWidget().updateAll(context)
        PinnedNoteWidget().updateAll(context)
        QuickCaptureWidget().updateAll(context)
    }

    /**
     * Rebuild the cache and refresh every widget instance. Serialized: two
     * quick updates must not render out of order (a stale OFF state could
     * otherwise land after the fresh ON state).
     */
    suspend fun refreshAndUpdate(context: Context, core: FfiCore) {
        updateMutex.withLock {
            withContext(Dispatchers.IO) { refresh(context, core) }
            renderAll(context)
        }
    }

    // ── Serialization ───────────────────────────────────────────────────────

    private fun serialize(notes: List<WidgetNote>): String {
        val arr = JSONArray()
        notes.forEach { n ->
            arr.put(
                JSONObject()
                    .put("id", n.id)
                    .put("title", n.title)
                    .put("updated_at", n.updatedAt)
                    .put("tags", JSONArray(n.tags))
                    .put("text", n.text)
                    .put(
                        "checklist",
                        JSONArray().apply {
                            n.checklist.forEach { item ->
                                put(
                                    JSONObject()
                                        .put("text", item.text)
                                        .put("checked", item.checked),
                                )
                            }
                        },
                    ),
            )
        }
        return JSONObject().put("notes", arr).toString()
    }

    private fun parse(raw: String): List<WidgetNote> {
        val arr = JSONObject(raw).optJSONArray("notes") ?: return emptyList()
        return (0 until arr.length()).mapNotNull { i ->
            val n = arr.optJSONObject(i) ?: return@mapNotNull null
            val tags = n.optJSONArray("tags")?.let { t -> (0 until t.length()).map { t.optString(it) } } ?: emptyList()
            val checklist = n.optJSONArray("checklist")?.let { c ->
                (0 until c.length()).mapNotNull { j ->
                    val item = c.optJSONObject(j) ?: return@mapNotNull null
                    TaskItem(item.optString("text"), item.optBoolean("checked", false))
                }
            } ?: emptyList()
            WidgetNote(
                id = n.optString("id"),
                title = n.optString("title"),
                tags = tags,
                updatedAt = n.optString("updated_at"),
                checklist = checklist,
                text = n.optString("text"),
            )
        }
    }

    // ── Keystore-wrapped cache ──────────────────────────────────────────────

    private fun secretKey(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getEntry(KEY_ALIAS, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        gen.init(
            KeyGenParameterSpec.Builder(
                KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build(),
        )
        return gen.generateKey()
    }

    private fun writeCache(context: Context, json: String) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, secretKey())
        val iv = cipher.iv
        val ct = cipher.doFinal(json.toByteArray())
        File(context.filesDir, CACHE_FILE).writeBytes(iv + ct)
    }

    private fun readCache(context: Context): String? {
        val file = File(context.filesDir, CACHE_FILE)
        if (!file.exists()) return null
        val bytes = file.readBytes()
        if (bytes.size <= IV_LENGTH) return null
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            secretKey(),
            GCMParameterSpec(GCM_TAG_BITS, bytes.copyOfRange(0, IV_LENGTH)),
        )
        return String(cipher.doFinal(bytes.copyOfRange(IV_LENGTH, bytes.size)))
    }
}
