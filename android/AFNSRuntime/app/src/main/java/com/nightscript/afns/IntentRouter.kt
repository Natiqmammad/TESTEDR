package com.nightscript.afns

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.MediaStore
import android.provider.Settings
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger

/**
 * IntentRouter - Intent Management for ApexForge NightScript
 *
 * Handles:
 * - Intent creation and sending
 * - Activity launching (by class name or action)
 * - Activity results (startActivityForResult)
 * - Common Android intents (browser, dialer, camera, etc.)
 * - Custom intent actions
 * - Intent extras (data passing)
 *
 * Supports both explicit intents (specific component) and
 * implicit intents (action-based).
 */
class IntentRouter(private val activity: Activity) {

    companion object {
        private const val TAG = "IntentRouter"
        const val REQUEST_CODE_BASE = 4000

        // Common intent actions
        const val ACTION_VIEW = Intent.ACTION_VIEW
        const val ACTION_SEND = Intent.ACTION_SEND
        const val ACTION_DIAL = Intent.ACTION_DIAL
        const val ACTION_CALL = Intent.ACTION_CALL
        const val ACTION_SENDTO = Intent.ACTION_SENDTO
        const val ACTION_PICK = Intent.ACTION_PICK
        const val ACTION_GET_CONTENT = Intent.ACTION_GET_CONTENT

        // Common intent categories
        const val CATEGORY_BROWSABLE = Intent.CATEGORY_BROWSABLE
        const val CATEGORY_DEFAULT = Intent.CATEGORY_DEFAULT
        const val CATEGORY_LAUNCHER = Intent.CATEGORY_LAUNCHER

        // Common MIME types
        const val MIME_TEXT_PLAIN = "text/plain"
        const val MIME_IMAGE_ALL = "image/*"
        const val MIME_VIDEO_ALL = "video/*"
        const val MIME_AUDIO_ALL = "audio/*"
        const val MIME_ANY = "*/*"
    }

    // Request code generator
    private val requestCodeGenerator = AtomicInteger(REQUEST_CODE_BASE)

    // Pending intent result callbacks
    private val resultCallbacks = ConcurrentHashMap<Int, (Int, Intent?) -> Unit>()

    // ========================================
    // Basic Intent Operations
    // ========================================

    /**
     * Send a simple intent with action and extras
     * @param action Intent action (e.g., Intent.ACTION_VIEW)
     * @param extras Map of extra key-value pairs
     */
    fun sendIntent(action: String, extras: Map<String, String>) {
        println("[$TAG] Sending intent: action=$action")

        try {
            val intent = Intent(action).apply {
                extras.forEach { (key, value) ->
                    putExtra(key, value)
                }
            }

            activity.startActivity(intent)
            println("[$TAG] Intent sent successfully")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to send intent: ${e.message}")
            e.printStackTrace()
        }
    }

    /**
     * Start an activity by class name
     * @param className Fully qualified class name (e.g., "com.example.MyActivity")
     * @param extras Map of extra key-value pairs
     */
    fun startActivity(className: String, extras: Map<String, String>) {
        println("[$TAG] Starting activity: $className")

        try {
            val targetClass = Class.forName(className)
            val intent = Intent(activity, targetClass).apply {
                extras.forEach { (key, value) ->
                    putExtra(key, value)
                }
            }

            activity.startActivity(intent)
            println("[$TAG] Activity started successfully")
        } catch (e: ClassNotFoundException) {
            println("[$TAG] ERROR: Class not found: $className")
            e.printStackTrace()
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to start activity: ${e.message}")
            e.printStackTrace()
        }
    }

    /**
     * Start an activity for result
     * @param className Fully qualified class name
     * @param extras Map of extra key-value pairs
     * @param requestCode Request code for result identification
     * @param callback Result callback (resultCode, data)
     */
    fun startActivityForResult(
        className: String,
        extras: Map<String, String>,
        requestCode: Int,
        callback: ((Int, Intent?) -> Unit)? = null
    ) {
        println("[$TAG] Starting activity for result: $className, requestCode=$requestCode")

        try {
            val targetClass = Class.forName(className)
            val intent = Intent(activity, targetClass).apply {
                extras.forEach { (key, value) ->
                    putExtra(key, value)
                }
            }

            // Store callback
            callback?.let { resultCallbacks[requestCode] = it }

            activity.startActivityForResult(intent, requestCode)
            println("[$TAG] Activity started for result")
        } catch (e: ClassNotFoundException) {
            println("[$TAG] ERROR: Class not found: $className")
            callback?.invoke(Activity.RESULT_CANCELED, null)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to start activity for result: ${e.message}")
            e.printStackTrace()
            callback?.invoke(Activity.RESULT_CANCELED, null)
        }
    }

    /**
     * Start an activity with action for result
     * @param action Intent action
     * @param callback Result callback
     */
    fun startActivityForResultWithAction(
        action: String,
        callback: (Int, Intent?) -> Unit
    ): Int {
        val requestCode = requestCodeGenerator.incrementAndGet()
        println("[$TAG] Starting activity for result: action=$action, requestCode=$requestCode")

        try {
            val intent = Intent(action)
            resultCallbacks[requestCode] = callback
            activity.startActivityForResult(intent, requestCode)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to start activity: ${e.message}")
            callback(Activity.RESULT_CANCELED, null)
        }

        return requestCode
    }

    /**
     * Handle activity result
     * This should be called from Activity.onActivityResult()
     */
    fun handleActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        println("[$TAG] Handling activity result: requestCode=$requestCode, resultCode=$resultCode")

        val callback = resultCallbacks.remove(requestCode)
        if (callback != null) {
            try {
                callback(resultCode, data)
                println("[$TAG] Result callback invoked successfully")
            } catch (e: Exception) {
                println("[$TAG] ERROR: Result callback failed: ${e.message}")
                e.printStackTrace()
            }
        } else {
            println("[$TAG] WARNING: No callback found for request code $requestCode")
        }
    }

    // ========================================
    // Common Android Intents
    // ========================================

    /**
     * Open URL in browser
     * @param url URL to open
     */
    fun openUrl(url: String) {
        println("[$TAG] Opening URL: $url")

        try {
            val uri = Uri.parse(url)
            val intent = Intent(Intent.ACTION_VIEW, uri)
            activity.startActivity(intent)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to open URL: ${e.message}")
        }
    }

    /**
     * Open dialer with phone number
     * @param phoneNumber Phone number
     */
    fun openDialer(phoneNumber: String) {
        println("[$TAG] Opening dialer: $phoneNumber")

        try {
            val uri = Uri.parse("tel:$phoneNumber")
            val intent = Intent(Intent.ACTION_DIAL, uri)
            activity.startActivity(intent)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to open dialer: ${e.message}")
        }
    }

    /**
     * Make phone call (requires CALL_PHONE permission)
     * @param phoneNumber Phone number
     */
    fun makePhoneCall(phoneNumber: String) {
        println("[$TAG] Making phone call: $phoneNumber")

        try {
            val uri = Uri.parse("tel:$phoneNumber")
            val intent = Intent(Intent.ACTION_CALL, uri)
            activity.startActivity(intent)
        } catch (e: SecurityException) {
            println("[$TAG] ERROR: Missing CALL_PHONE permission")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to make call: ${e.message}")
        }
    }

    /**
     * Send SMS
     * @param phoneNumber Recipient phone number
     * @param message Message text
     */
    fun sendSms(phoneNumber: String, message: String) {
        println("[$TAG] Sending SMS to: $phoneNumber")

        try {
            val uri = Uri.parse("smsto:$phoneNumber")
            val intent = Intent(Intent.ACTION_SENDTO, uri).apply {
                putExtra("sms_body", message)
            }
            activity.startActivity(intent)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to send SMS: ${e.message}")
        }
    }

    /**
     * Send email
     * @param to Recipient email address
     * @param subject Email subject
     * @param body Email body
     */
    fun sendEmail(to: String, subject: String, body: String) {
        println("[$TAG] Sending email to: $to")

        try {
            val intent = Intent(Intent.ACTION_SENDTO).apply {
                data = Uri.parse("mailto:")
                putExtra(Intent.EXTRA_EMAIL, arrayOf(to))
                putExtra(Intent.EXTRA_SUBJECT, subject)
                putExtra(Intent.EXTRA_TEXT, body)
            }
            activity.startActivity(Intent.createChooser(intent, "Send email"))
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to send email: ${e.message}")
        }
    }

    /**
     * Share text
     * @param text Text to share
     * @param title Chooser title
     */
    fun shareText(text: String, title: String = "Share") {
        println("[$TAG] Sharing text")

        try {
            val intent = Intent(Intent.ACTION_SEND).apply {
                type = MIME_TEXT_PLAIN
                putExtra(Intent.EXTRA_TEXT, text)
            }
            activity.startActivity(Intent.createChooser(intent, title))
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to share text: ${e.message}")
        }
    }

    /**
     * Share file
     * @param fileUri File URI
     * @param mimeType MIME type
     * @param title Chooser title
     */
    fun shareFile(fileUri: Uri, mimeType: String, title: String = "Share") {
        println("[$TAG] Sharing file: $fileUri")

        try {
            val intent = Intent(Intent.ACTION_SEND).apply {
                type = mimeType
                putExtra(Intent.EXTRA_STREAM, fileUri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            activity.startActivity(Intent.createChooser(intent, title))
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to share file: ${e.message}")
        }
    }

    /**
     * Open camera for photo capture
     * @param callback Result callback with image URI
     */
    fun openCamera(callback: (Int, Intent?) -> Unit): Int {
        println("[$TAG] Opening camera")

        val requestCode = requestCodeGenerator.incrementAndGet()
        try {
            val intent = Intent(MediaStore.ACTION_IMAGE_CAPTURE)
            resultCallbacks[requestCode] = callback
            activity.startActivityForResult(intent, requestCode)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to open camera: ${e.message}")
            callback(Activity.RESULT_CANCELED, null)
        }

        return requestCode
    }

    /**
     * Open gallery for image selection
     * @param callback Result callback with selected image URI
     */
    fun openGallery(callback: (Int, Intent?) -> Unit): Int {
        println("[$TAG] Opening gallery")

        val requestCode = requestCodeGenerator.incrementAndGet()
        try {
            val intent = Intent(Intent.ACTION_PICK, MediaStore.Images.Media.EXTERNAL_CONTENT_URI)
            resultCallbacks[requestCode] = callback
            activity.startActivityForResult(intent, requestCode)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to open gallery: ${e.message}")
            callback(Activity.RESULT_CANCELED, null)
        }

        return requestCode
    }

    /**
     * Pick file with specific MIME type
     * @param mimeType MIME type (e.g., "image/*", "application/pdf")
     * @param callback Result callback with file URI
     */
    fun pickFile(mimeType: String = MIME_ANY, callback: (Int, Intent?) -> Unit): Int {
        println("[$TAG] Picking file: mimeType=$mimeType")

        val requestCode = requestCodeGenerator.incrementAndGet()
        try {
            val intent = Intent(Intent.ACTION_GET_CONTENT).apply {
                type = mimeType
                addCategory(Intent.CATEGORY_OPENABLE)
            }
            resultCallbacks[requestCode] = callback
            activity.startActivityForResult(intent, requestCode)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to pick file: ${e.message}")
            callback(Activity.RESULT_CANCELED, null)
        }

        return requestCode
    }

    /**
     * Open location in maps
     * @param latitude Latitude
     * @param longitude Longitude
     * @param label Location label
     */
    fun openMaps(latitude: Double, longitude: Double, label: String = "Location") {
        println("[$TAG] Opening maps: lat=$latitude, lon=$longitude")

        try {
            val uri = Uri.parse("geo:$latitude,$longitude?q=$latitude,$longitude($label)")
            val intent = Intent(Intent.ACTION_VIEW, uri)
            activity.startActivity(intent)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to open maps: ${e.message}")
        }
    }

    /**
     * Open app settings
     */
    fun openAppSettings() {
        println("[$TAG] Opening app settings")

        try {
            val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                data = Uri.fromParts("package", activity.packageName, null)
            }
            activity.startActivity(intent)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to open settings: ${e.message}")
        }
    }

    /**
     * Open Google Play store page for this app
     */
    fun openPlayStore() {
        println("[$TAG] Opening Play Store")

        try {
            val uri = Uri.parse("market://details?id=${activity.packageName}")
            val intent = Intent(Intent.ACTION_VIEW, uri)
            activity.startActivity(intent)
        } catch (e: Exception) {
            // Fallback to web browser
            try {
                val uri = Uri.parse("https://play.google.com/store/apps/details?id=${activity.packageName}")
                val intent = Intent(Intent.ACTION_VIEW, uri)
                activity.startActivity(intent)
            } catch (fallbackError: Exception) {
                println("[$TAG] ERROR: Failed to open Play Store: ${fallbackError.message}")
            }
        }
    }

    // ========================================
    // Advanced Intent Operations
    // ========================================

    /**
     * Create custom intent with builder pattern
     */
    fun buildIntent(builder: IntentBuilder.() -> Unit): Intent {
        val intentBuilder = IntentBuilder()
        intentBuilder.builder()
        return intentBuilder.build()
    }

    /**
     * IntentBuilder for complex intent construction
     */
    inner class IntentBuilder {
        private var action: String? = null
        private var data: Uri? = null
        private var type: String? = null
        private var categories = mutableListOf<String>()
        private var extras = Bundle()
        private var flags = 0

        fun action(action: String) {
            this.action = action
        }

        fun data(uri: Uri) {
            this.data = uri
        }

        fun data(uriString: String) {
            this.data = Uri.parse(uriString)
        }

        fun type(mimeType: String) {
            this.type = mimeType
        }

        fun category(category: String) {
            categories.add(category)
        }

        fun extra(key: String, value: Any) {
            when (value) {
                is String -> extras.putString(key, value)
                is Int -> extras.putInt(key, value)
                is Long -> extras.putLong(key, value)
                is Boolean -> extras.putBoolean(key, value)
                is Float -> extras.putFloat(key, value)
                is Double -> extras.putDouble(key, value)
                else -> println("[$TAG] WARNING: Unsupported extra type: ${value::class.java}")
            }
        }

        fun flags(flags: Int) {
            this.flags = flags
        }

        fun build(): Intent {
            val intent = Intent().apply {
                action?.let { setAction(it) }
                data?.let { setData(it) }
                type?.let { setType(it) }
                categories.forEach { addCategory(it) }
                putExtras(extras)
                setFlags(flags)
            }
            return intent
        }
    }

    // ========================================
    // Utility Methods
    // ========================================

    /**
     * Check if intent can be handled
     */
    fun canHandleIntent(intent: Intent): Boolean {
        val packageManager = activity.packageManager
        return intent.resolveActivity(packageManager) != null
    }

    /**
     * Get number of pending result callbacks
     */
    fun getPendingCallbackCount(): Int = resultCallbacks.size

    /**
     * Clear all pending callbacks
     */
    fun clearPendingCallbacks() {
        println("[$TAG] Clearing ${resultCallbacks.size} pending callbacks")
        resultCallbacks.clear()
    }

    /**
     * Clean up old callbacks (older than 10 minutes)
     */
    fun cleanupOldCallbacks() {
        // In a real implementation, you'd track timestamps
        // For now, just clear all if too many
        if (resultCallbacks.size > 50) {
            println("[$TAG] Too many pending callbacks (${resultCallbacks.size}), clearing old ones")
            val toKeep = resultCallbacks.entries.takeLast(20)
            resultCallbacks.clear()
            toKeep.forEach { resultCallbacks[it.key] = it.value }
        }
    }
}
