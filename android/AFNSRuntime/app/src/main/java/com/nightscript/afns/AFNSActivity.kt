package com.nightscript.afns

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import java.util.concurrent.ConcurrentHashMap

/**
 * AFNSActivity - Main Activity for ApexForge NightScript Android Runtime
 *
 * This activity serves as the bridge between Android and NightScript runtime.
 * It handles:
 * - Activity lifecycle → JNI callbacks
 * - Permission requests → Native permission manager
 * - Intent routing → Native intent handler
 * - Service management → Native service manager
 * - Storage access → Native storage manager
 *
 * Architecture:
 * Android (Kotlin/Java) ↔ JNI Bridge (C) ↔ Rust Runtime ↔ NightScript VM
 */
class AFNSActivity : AppCompatActivity() {

    companion object {
        const val TAG = "AFNSActivity"
        const val PERMISSION_REQUEST_CODE = 1001
        const val INTENT_REQUEST_CODE = 2001

        // Singleton instance for global access
        @Volatile
        private var instance: AFNSActivity? = null

        fun getInstance(): AFNSActivity? = instance

        init {
            // Load native library
            try {
                System.loadLibrary("nightscript_android")
                println("[$TAG] Native library loaded successfully")
            } catch (e: UnsatisfiedLinkError) {
                println("[$TAG] Failed to load native library: ${e.message}")
            }
        }
    }

    // Managers
    private lateinit var permissionManager: PermissionManager
    private lateinit var intentRouter: IntentRouter
    private lateinit var serviceManager: ServiceManager
    private lateinit var storageManager: StorageManager

    // Lifecycle state
    private var isInitialized = false
    private var isPaused = false
    private var isStopped = false

    // Permission callback storage
    private val permissionCallbacks = ConcurrentHashMap<String, (Boolean) -> Unit>()

    // Intent result callback storage
    private val intentCallbacks = ConcurrentHashMap<Int, (Int, Intent?) -> Unit>()

    // Native method declarations (JNI bridge)
    private external fun onNativeCreate(activityPtr: Long)
    private external fun onNativeStart()
    private external fun onNativeResume()
    private external fun onNativePause()
    private external fun onNativeStop()
    private external fun onNativeDestroy()
    private external fun onNativePermissionResult(permission: String, granted: Boolean)
    private external fun onNativeIntentReceived(action: String, extras: String)
    private external fun onNativeIntentResult(requestCode: Int, resultCode: Int, data: String)

    // ========================================
    // Activity Lifecycle Methods
    // ========================================

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        println("[$TAG] onCreate called")

        // Set singleton instance
        instance = this

        // Initialize managers
        permissionManager = PermissionManager(this)
        intentRouter = IntentRouter(this)
        serviceManager = ServiceManager(this)
        storageManager = StorageManager(this)

        // Initialize NativeBridge
        NativeBridge.initialize(this)

        // Notify native layer
        try {
            onNativeCreate(NativeBridge.getActivityPointer())
            isInitialized = true
            println("[$TAG] Native layer initialized")
        } catch (e: Exception) {
            println("[$TAG] Failed to initialize native layer: ${e.message}")
        }

        // Handle incoming intent
        intent?.let { handleIntent(it) }
    }

    override fun onStart() {
        super.onStart()
        println("[$TAG] onStart called")
        isStopped = false

        if (isInitialized) {
            try {
                onNativeStart()
            } catch (e: Exception) {
                println("[$TAG] Native onStart failed: ${e.message}")
            }
        }
    }

    override fun onResume() {
        super.onResume()
        println("[$TAG] onResume called")
        isPaused = false

        if (isInitialized) {
            try {
                onNativeResume()
            } catch (e: Exception) {
                println("[$TAG] Native onResume failed: ${e.message}")
            }
        }
    }

    override fun onPause() {
        super.onPause()
        println("[$TAG] onPause called")
        isPaused = true

        if (isInitialized) {
            try {
                onNativePause()
            } catch (e: Exception) {
                println("[$TAG] Native onPause failed: ${e.message}")
            }
        }
    }

    override fun onStop() {
        super.onStop()
        println("[$TAG] onStop called")
        isStopped = true

        if (isInitialized) {
            try {
                onNativeStop()
            } catch (e: Exception) {
                println("[$TAG] Native onStop failed: ${e.message}")
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        println("[$TAG] onDestroy called")

        if (isInitialized) {
            try {
                onNativeDestroy()
            } catch (e: Exception) {
                println("[$TAG] Native onDestroy failed: ${e.message}")
            }
        }

        // Clear singleton
        instance = null

        // Shutdown native bridge
        NativeBridge.shutdown()
    }

    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        println("[$TAG] onNewIntent called")
        intent?.let { handleIntent(it) }
    }

    // ========================================
    // Intent Handling
    // ========================================

    private fun handleIntent(intent: Intent) {
        val action = intent.action ?: "NONE"
        val extras = intent.extras?.let { bundle ->
            bundle.keySet().joinToString(", ") { key ->
                "$key=${bundle.get(key)}"
            }
        } ?: ""

        println("[$TAG] Handling intent: action=$action, extras=$extras")

        if (isInitialized) {
            try {
                onNativeIntentReceived(action, extras)
            } catch (e: Exception) {
                println("[$TAG] Failed to notify native layer of intent: ${e.message}")
            }
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        println("[$TAG] onActivityResult: requestCode=$requestCode, resultCode=$resultCode")

        // Handle intent result callback
        intentCallbacks[requestCode]?.let { callback ->
            callback(resultCode, data)
            intentCallbacks.remove(requestCode)
        }

        // Notify native layer
        val dataString = data?.extras?.let { bundle ->
            bundle.keySet().joinToString(", ") { key ->
                "$key=${bundle.get(key)}"
            }
        } ?: ""

        if (isInitialized) {
            try {
                onNativeIntentResult(requestCode, resultCode, dataString)
            } catch (e: Exception) {
                println("[$TAG] Failed to notify native layer of result: ${e.message}")
            }
        }
    }

    // ========================================
    // Permission Handling
    // ========================================

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        println("[$TAG] onRequestPermissionsResult: requestCode=$requestCode")

        if (requestCode == PERMISSION_REQUEST_CODE) {
            permissions.forEachIndexed { index, permission ->
                val granted = grantResults.getOrNull(index) == PackageManager.PERMISSION_GRANTED
                println("[$TAG] Permission result: $permission -> $granted")

                // Trigger callback
                permissionCallbacks[permission]?.let { callback ->
                    callback(granted)
                    permissionCallbacks.remove(permission)
                }

                // Notify native layer
                if (isInitialized) {
                    try {
                        onNativePermissionResult(permission, granted)
                    } catch (e: Exception) {
                        println("[$TAG] Failed to notify native layer: ${e.message}")
                    }
                }
            }
        }
    }

    // ========================================
    // Public API (called from JNI)
    // ========================================

    /**
     * Show a toast message (called from native code via JNI)
     */
    fun showToast(message: String, duration: Int = Toast.LENGTH_SHORT) {
        runOnUiThread {
            Toast.makeText(this, message, duration).show()
            println("[$TAG] Toast shown: $message")
        }
    }

    /**
     * Request a single permission (called from native code via JNI)
     */
    fun requestPermission(permission: String, callback: ((Boolean) -> Unit)? = null): Boolean {
        println("[$TAG] Requesting permission: $permission")

        // Check if already granted
        if (ContextCompat.checkSelfPermission(this, permission) == PackageManager.PERMISSION_GRANTED) {
            println("[$TAG] Permission already granted: $permission")
            callback?.invoke(true)
            return true
        }

        // Store callback
        callback?.let { permissionCallbacks[permission] = it }

        // Request permission
        ActivityCompat.requestPermissions(this, arrayOf(permission), PERMISSION_REQUEST_CODE)
        return false
    }

    /**
     * Request multiple permissions (called from native code via JNI)
     */
    fun requestPermissions(permissions: Array<String>, callback: ((Map<String, Boolean>) -> Unit)? = null) {
        println("[$TAG] Requesting multiple permissions: ${permissions.joinToString()}")

        val results = mutableMapOf<String, Boolean>()
        val needToRequest = mutableListOf<String>()

        permissions.forEach { permission ->
            if (ContextCompat.checkSelfPermission(this, permission) == PackageManager.PERMISSION_GRANTED) {
                results[permission] = true
            } else {
                needToRequest.add(permission)
            }
        }

        if (needToRequest.isEmpty()) {
            // All already granted
            callback?.invoke(results)
            return
        }

        // Request permissions that aren't granted
        ActivityCompat.requestPermissions(
            this,
            needToRequest.toTypedArray(),
            PERMISSION_REQUEST_CODE
        )
    }

    /**
     * Check if permission is granted (called from native code via JNI)
     */
    fun isPermissionGranted(permission: String): Boolean {
        val granted = ContextCompat.checkSelfPermission(this, permission) == PackageManager.PERMISSION_GRANTED
        println("[$TAG] Permission check: $permission -> $granted")
        return granted
    }

    /**
     * Send an intent (called from native code via JNI)
     */
    fun sendIntent(action: String, extras: Map<String, String>) {
        println("[$TAG] Sending intent: action=$action, extras=$extras")
        intentRouter.sendIntent(action, extras)
    }

    /**
     * Start an activity (called from native code via JNI)
     */
    fun startActivityByName(className: String, extras: Map<String, String>) {
        println("[$TAG] Starting activity: $className")
        intentRouter.startActivity(className, extras)
    }

    /**
     * Start an activity for result (called from native code via JNI)
     */
    fun startActivityForResult(
        className: String,
        extras: Map<String, String>,
        callback: ((Int, Intent?) -> Unit)? = null
    ) {
        println("[$TAG] Starting activity for result: $className")
        val requestCode = INTENT_REQUEST_CODE + intentCallbacks.size
        callback?.let { intentCallbacks[requestCode] = it }
        intentRouter.startActivityForResult(className, extras, requestCode)
    }

    /**
     * Start a foreground service (called from native code via JNI)
     */
    fun startForegroundService(serviceClassName: String, extras: Map<String, String>) {
        println("[$TAG] Starting foreground service: $serviceClassName")
        serviceManager.startForegroundService(serviceClassName, extras)
    }

    /**
     * Stop a service (called from native code via JNI)
     */
    fun stopService(serviceClassName: String) {
        println("[$TAG] Stopping service: $serviceClassName")
        serviceManager.stopService(serviceClassName)
    }

    /**
     * Get internal storage path (called from native code via JNI)
     */
    fun getInternalStoragePath(): String {
        val path = storageManager.getInternalStoragePath()
        println("[$TAG] Internal storage path: $path")
        return path
    }

    /**
     * Get external storage path (called from native code via JNI)
     */
    fun getExternalStoragePath(): String {
        val path = storageManager.getExternalStoragePath()
        println("[$TAG] External storage path: $path")
        return path
    }

    /**
     * Get cache directory path (called from native code via JNI)
     */
    fun getCachePath(): String {
        val path = storageManager.getCacheDir()
        println("[$TAG] Cache path: $path")
        return path
    }

    /**
     * Get files directory path (called from native code via JNI)
     */
    fun getFilesPath(): String {
        val path = storageManager.getFilesDir()
        println("[$TAG] Files path: $path")
        return path
    }

    // ========================================
    // Getters
    // ========================================

    fun getPermissionManager(): PermissionManager = permissionManager
    fun getIntentRouter(): IntentRouter = intentRouter
    fun getServiceManager(): ServiceManager = serviceManager
    fun getStorageManager(): StorageManager = storageManager

    fun isActivityPaused(): Boolean = isPaused
    fun isActivityStopped(): Boolean = isStopped
    fun isActivityInitialized(): Boolean = isInitialized
}
