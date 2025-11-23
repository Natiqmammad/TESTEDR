package com.nightscript.afns

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger

/**
 * PermissionManager - Comprehensive Android Permission Management
 *
 * Handles runtime permission requests for Android 6.0+ (API 23+)
 * Supports:
 * - Single and batch permission requests
 * - Permission status checking
 * - Rationale display
 * - Settings page navigation
 * - Callback-based async results
 *
 * Common Android Permissions:
 * - CAMERA
 * - RECORD_AUDIO
 * - READ_EXTERNAL_STORAGE / WRITE_EXTERNAL_STORAGE
 * - ACCESS_FINE_LOCATION / ACCESS_COARSE_LOCATION
 * - CALL_PHONE / READ_PHONE_STATE
 * - SEND_SMS / RECEIVE_SMS
 * - READ_CONTACTS / WRITE_CONTACTS
 * - BLUETOOTH / BLUETOOTH_ADMIN
 * - INTERNET (not runtime, declared in manifest)
 */
class PermissionManager(private val activity: Activity) {

    companion object {
        private const val TAG = "PermissionManager"
        const val PERMISSION_REQUEST_BASE = 3000

        // Common permission groups
        val CAMERA_PERMISSIONS = arrayOf(
            Manifest.permission.CAMERA
        )

        val STORAGE_PERMISSIONS = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            arrayOf(
                Manifest.permission.READ_MEDIA_IMAGES,
                Manifest.permission.READ_MEDIA_VIDEO,
                Manifest.permission.READ_MEDIA_AUDIO
            )
        } else {
            arrayOf(
                Manifest.permission.READ_EXTERNAL_STORAGE,
                Manifest.permission.WRITE_EXTERNAL_STORAGE
            )
        }

        val LOCATION_PERMISSIONS = arrayOf(
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_COARSE_LOCATION
        )

        val AUDIO_PERMISSIONS = arrayOf(
            Manifest.permission.RECORD_AUDIO
        )

        val PHONE_PERMISSIONS = arrayOf(
            Manifest.permission.CALL_PHONE,
            Manifest.permission.READ_PHONE_STATE
        )

        val SMS_PERMISSIONS = arrayOf(
            Manifest.permission.SEND_SMS,
            Manifest.permission.RECEIVE_SMS,
            Manifest.permission.READ_SMS
        )

        val CONTACTS_PERMISSIONS = arrayOf(
            Manifest.permission.READ_CONTACTS,
            Manifest.permission.WRITE_CONTACTS
        )

        val BLUETOOTH_PERMISSIONS = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            arrayOf(
                Manifest.permission.BLUETOOTH_SCAN,
                Manifest.permission.BLUETOOTH_CONNECT,
                Manifest.permission.BLUETOOTH_ADVERTISE
            )
        } else {
            arrayOf(
                Manifest.permission.BLUETOOTH,
                Manifest.permission.BLUETOOTH_ADMIN
            )
        }

        val CALENDAR_PERMISSIONS = arrayOf(
            Manifest.permission.READ_CALENDAR,
            Manifest.permission.WRITE_CALENDAR
        )

        val SENSORS_PERMISSIONS = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.KITKAT_WATCH) {
            arrayOf(
                Manifest.permission.BODY_SENSORS
            )
        } else {
            emptyArray()
        }
    }

    // Request code generator
    private val requestCodeGenerator = AtomicInteger(PERMISSION_REQUEST_BASE)

    // Pending permission requests
    private val pendingRequests = ConcurrentHashMap<Int, PermissionRequest>()

    // Permission status cache
    private val permissionCache = ConcurrentHashMap<String, Boolean>()

    /**
     * Data class for permission request tracking
     */
    private data class PermissionRequest(
        val permissions: Array<String>,
        val callback: (Map<String, Boolean>) -> Unit,
        val timestamp: Long = System.currentTimeMillis()
    )

    // ========================================
    // Single Permission Operations
    // ========================================

    /**
     * Request a single permission
     * @param permission Permission constant (e.g., Manifest.permission.CAMERA)
     * @param callback Result callback (granted: Boolean)
     */
    fun requestPermission(permission: String, callback: (Boolean) -> Unit) {
        println("[$TAG] Requesting permission: $permission")

        // Check if already granted
        if (isGranted(permission)) {
            println("[$TAG] Permission already granted: $permission")
            callback(true)
            return
        }

        // Check if we should show rationale
        if (shouldShowRationale(permission)) {
            println("[$TAG] Should show rationale for: $permission")
            // For now, proceed with request
            // In a real app, you might show a dialog explaining why you need the permission
        }

        // Request permission
        val requestCode = requestCodeGenerator.incrementAndGet()
        pendingRequests[requestCode] = PermissionRequest(
            permissions = arrayOf(permission),
            callback = { results ->
                callback(results[permission] ?: false)
            }
        )

        ActivityCompat.requestPermissions(activity, arrayOf(permission), requestCode)
    }

    /**
     * Check if a permission is granted
     * @param permission Permission constant
     * @return true if granted, false otherwise
     */
    fun isGranted(permission: String): Boolean {
        // Check cache first
        permissionCache[permission]?.let { return it }

        // Check actual permission
        val granted = ContextCompat.checkSelfPermission(
            activity,
            permission
        ) == PackageManager.PERMISSION_GRANTED

        // Update cache
        permissionCache[permission] = granted

        return granted
    }

    /**
     * Check if we should show permission rationale
     * @param permission Permission constant
     * @return true if rationale should be shown
     */
    fun shouldShowRationale(permission: String): Boolean {
        return ActivityCompat.shouldShowRequestPermissionRationale(activity, permission)
    }

    // ========================================
    // Batch Permission Operations
    // ========================================

    /**
     * Request multiple permissions at once
     * @param permissions Array of permission constants
     * @param callback Result callback (Map<permission, granted>)
     */
    fun requestPermissions(permissions: Array<String>, callback: (Map<String, Boolean>) -> Unit) {
        println("[$TAG] Requesting multiple permissions: ${permissions.joinToString()}")

        val results = mutableMapOf<String, Boolean>()
        val needToRequest = mutableListOf<String>()

        // Check which permissions are already granted
        permissions.forEach { permission ->
            if (isGranted(permission)) {
                results[permission] = true
                println("[$TAG] Permission already granted: $permission")
            } else {
                needToRequest.add(permission)
            }
        }

        // If all are granted, return immediately
        if (needToRequest.isEmpty()) {
            println("[$TAG] All permissions already granted")
            callback(results)
            return
        }

        // Request remaining permissions
        val requestCode = requestCodeGenerator.incrementAndGet()
        pendingRequests[requestCode] = PermissionRequest(
            permissions = needToRequest.toTypedArray(),
            callback = { grantResults ->
                results.putAll(grantResults)
                callback(results)
            }
        )

        ActivityCompat.requestPermissions(
            activity,
            needToRequest.toTypedArray(),
            requestCode
        )
    }

    /**
     * Check if all permissions in array are granted
     * @param permissions Array of permission constants
     * @return true if all are granted, false otherwise
     */
    fun areAllGranted(permissions: Array<String>): Boolean {
        return permissions.all { isGranted(it) }
    }

    /**
     * Check if any permission in array is granted
     * @param permissions Array of permission constants
     * @return true if at least one is granted, false otherwise
     */
    fun isAnyGranted(permissions: Array<String>): Boolean {
        return permissions.any { isGranted(it) }
    }

    /**
     * Get status of multiple permissions
     * @param permissions Array of permission constants
     * @return Map of permission -> granted status
     */
    fun getPermissionStatus(permissions: Array<String>): Map<String, Boolean> {
        return permissions.associateWith { isGranted(it) }
    }

    // ========================================
    // Common Permission Groups
    // ========================================

    /**
     * Request camera permission
     */
    fun requestCameraPermission(callback: (Boolean) -> Unit) {
        requestPermission(Manifest.permission.CAMERA, callback)
    }

    /**
     * Request storage permissions (handles Android 13+ gracefully)
     */
    fun requestStoragePermissions(callback: (Map<String, Boolean>) -> Unit) {
        requestPermissions(STORAGE_PERMISSIONS, callback)
    }

    /**
     * Request location permissions
     */
    fun requestLocationPermissions(callback: (Map<String, Boolean>) -> Unit) {
        requestPermissions(LOCATION_PERMISSIONS, callback)
    }

    /**
     * Request audio recording permission
     */
    fun requestAudioPermission(callback: (Boolean) -> Unit) {
        requestPermission(Manifest.permission.RECORD_AUDIO, callback)
    }

    /**
     * Request phone permissions
     */
    fun requestPhonePermissions(callback: (Map<String, Boolean>) -> Unit) {
        requestPermissions(PHONE_PERMISSIONS, callback)
    }

    /**
     * Request SMS permissions
     */
    fun requestSmsPermissions(callback: (Map<String, Boolean>) -> Unit) {
        requestPermissions(SMS_PERMISSIONS, callback)
    }

    /**
     * Request contacts permissions
     */
    fun requestContactsPermissions(callback: (Map<String, Boolean>) -> Unit) {
        requestPermissions(CONTACTS_PERMISSIONS, callback)
    }

    /**
     * Request bluetooth permissions (handles Android 12+ gracefully)
     */
    fun requestBluetoothPermissions(callback: (Map<String, Boolean>) -> Unit) {
        requestPermissions(BLUETOOTH_PERMISSIONS, callback)
    }

    /**
     * Request calendar permissions
     */
    fun requestCalendarPermissions(callback: (Map<String, Boolean>) -> Unit) {
        requestPermissions(CALENDAR_PERMISSIONS, callback)
    }

    // ========================================
    // Permission Result Handling
    // ========================================

    /**
     * Handle permission request result
     * This should be called from Activity.onRequestPermissionsResult()
     *
     * @param requestCode Request code from onRequestPermissionsResult
     * @param permissions Permissions array from onRequestPermissionsResult
     * @param grantResults Grant results array from onRequestPermissionsResult
     */
    fun handlePermissionResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        println("[$TAG] Handling permission result: requestCode=$requestCode")

        val request = pendingRequests.remove(requestCode)
        if (request == null) {
            println("[$TAG] WARNING: No pending request found for code $requestCode")
            return
        }

        // Build results map
        val results = mutableMapOf<String, Boolean>()
        permissions.forEachIndexed { index, permission ->
            val granted = grantResults.getOrNull(index) == PackageManager.PERMISSION_GRANTED
            results[permission] = granted

            // Update cache
            permissionCache[permission] = granted

            println("[$TAG] Permission result: $permission -> $granted")
        }

        // Invoke callback
        try {
            request.callback(results)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Permission callback failed: ${e.message}")
            e.printStackTrace()
        }
    }

    // ========================================
    // Settings Navigation
    // ========================================

    /**
     * Open app settings page
     * Useful when user needs to manually grant permissions
     */
    fun openSettings() {
        println("[$TAG] Opening app settings")
        try {
            val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
                data = Uri.fromParts("package", activity.packageName, null)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            activity.startActivity(intent)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to open settings: ${e.message}")
            // Fallback: open main settings
            try {
                val intent = Intent(Settings.ACTION_SETTINGS)
                activity.startActivity(intent)
            } catch (fallbackError: Exception) {
                println("[$TAG] ERROR: Fallback settings also failed: ${fallbackError.message}")
            }
        }
    }

    /**
     * Check if permission is permanently denied
     * (user selected "Don't ask again")
     */
    fun isPermanentlyDenied(permission: String): Boolean {
        return !isGranted(permission) && !shouldShowRationale(permission)
    }

    // ========================================
    // Utility Methods
    // ========================================

    /**
     * Clear permission cache
     * Useful for testing or when permissions might have changed externally
     */
    fun clearCache() {
        println("[$TAG] Clearing permission cache")
        permissionCache.clear()
    }

    /**
     * Get all cached permissions
     */
    fun getCachedPermissions(): Map<String, Boolean> {
        return permissionCache.toMap()
    }

    /**
     * Clean up old pending requests (older than 5 minutes)
     */
    fun cleanupOldRequests() {
        val now = System.currentTimeMillis()
        val threshold = 5 * 60 * 1000L // 5 minutes

        val toRemove = pendingRequests.filter { (_, request) ->
            now - request.timestamp > threshold
        }.keys

        toRemove.forEach { requestCode ->
            println("[$TAG] Removing stale request: $requestCode")
            pendingRequests.remove(requestCode)
        }
    }

    /**
     * Get number of pending requests
     */
    fun getPendingRequestCount(): Int = pendingRequests.size

    /**
     * Get Android version info for permission debugging
     */
    fun getAndroidVersionInfo(): String {
        return buildString {
            append("SDK: ${Build.VERSION.SDK_INT}, ")
            append("Release: ${Build.VERSION.RELEASE}, ")
            append("Codename: ${Build.VERSION.CODENAME}")
        }
    }
}
