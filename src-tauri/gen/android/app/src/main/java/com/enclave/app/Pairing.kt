package com.enclave.app

import android.graphics.Bitmap
import android.graphics.Color
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel
import org.json.JSONObject

/**
 * QR pairing payload: the address of the device showing the code. The other
 * device scans it and connects — no typing `host:port` on a phone keyboard.
 * The wire protocol still authenticates with the vault-derived key, so a
 * scanned code from a different vault is refused there, not here.
 */
internal object Pairing {
    private const val KIND = "enclave-pair"
    private const val VERSION = 1

    data class Peer(val host: String, val port: Int)

    fun encode(host: String, port: Int): String = JSONObject()
        .put("t", KIND)
        .put("v", VERSION)
        .put("host", host)
        .put("port", port)
        .toString()

    /** Parses a scanned code; null when it is not an Enclave pairing code. */
    fun decode(contents: String): Peer? = try {
        val json = JSONObject(contents.trim())
        if (json.optString("t") != KIND) null
        else {
            val host = json.optString("host").takeIf { it.isNotBlank() }
            val port = json.optInt("port", 4242)
            host?.let { Peer(it, port) }
        }
    } catch (_: Exception) {
        null
    }

    /** QR bitmap with a proper quiet zone and high error correction. */
    fun qrBitmap(text: String, size: Int = 640): Bitmap {
        val hints = mapOf(
            EncodeHintType.ERROR_CORRECTION to ErrorCorrectionLevel.M,
            EncodeHintType.MARGIN to 2,
        )
        val matrix = QRCodeWriter().encode(text, BarcodeFormat.QR_CODE, size, size, hints)
        val bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888)
        for (x in 0 until size) {
            for (y in 0 until size) {
                bitmap.setPixel(x, y, if (matrix[x, y]) Color.BLACK else Color.WHITE)
            }
        }
        return bitmap
    }
}
