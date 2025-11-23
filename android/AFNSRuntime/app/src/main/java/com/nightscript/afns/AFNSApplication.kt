package com.nightscript.afns

import android.app.Application
import android.content.Context
import android.util.Log
import java.lang.ref.WeakReference

/**
 * AFNSApplication - Application class for ApexForge NightScript Android Runtime
 *
 * This is the main application entry point that initializes:
 * - Native library loading
 * - Global context management
 * - Crash handling
 * - Memory management
 * - Lifecycle observation
 *
 * Architecture:
 * Application (onCreate) → NativeBridge.initialize() → Rust VM Init → NightScript Runtime
 */
class AFNSApplication : Application() {

    companion object {
        private const val TAG = "AFNSApplication"

        @Volatile
        private var instance: AFNSApplication? = null

        /**
         * Get singleton application instance
         */
        fun getInstance(): AFNSApplication? = instance

        /**
         * Get application context (safe, returns null if not available)
         */
        fun getAppContext(): Context? = instance?.applicationContext

        /**
         * Check if application is initialized
         */
        fun isInitialized(): Boolean = instance != null
    }

    // Lifecycle state
    private var isAppInitialized = false
    private var activityCount = 0
    private var isInForeground = false

    // Application start time for uptime tracking
    private var startTime: Long = 0

    // Weak reference to current activity
    private var currentActivity: WeakReference<AFNSActivity>? = null

    // ========================================
    // Application Lifecycle
    // ========================================

    override fun onCreate() {
        super.onCreate()
        startTime = System.currentTimeMillis()

        Log.i(TAG, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
        Log.i(TAG, "  ApexForge NightScript (AFNS) Android Runtime")
        Log.i(TAG, "  Version: 1.0.0-alpha")
        Log.i(TAG, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")

        // Set singleton instance
        instance = this

        // Initialize native library
        initializeNativeLibrary()

        // Setup crash handler
        setupCrashHandler()

        // Setup memory management
        setupMemoryManagement()

        // Register activity lifecycle callbacks
        registerActivityLifecycleCallbacks(object : ActivityLifecycleCallbacks {
            override fun onActivityCreated(activity: android.app.Activity, savedInstanceState: android.os.Bundle?) {
                if (activity is AFNSActivity) {
                    currentActivity = WeakReference(activity)
                    Log.d(TAG, "Activity created: ${activity.javaClass.simpleName}")
                }
            }

            override fun onActivityStarted(activity: android.app.Activity) {
                activityCount++
                if (activityCount == 1) {
                    isInForeground = true
                    onEnterForeground()
                }
                Log.d(TAG, "Activity started: ${activity.javaClass.simpleName}, count=$activityCount")
            }

            override fun onActivityResumed(activity: android.app.Activity) {
                if (activity is AFNSActivity) {
                    currentActivity = WeakReference(activity)
                }
                Log.d(TAG, "Activity resumed: ${activity.javaClass.simpleName}")
            }

            override fun onActivityPaused(activity: android.app.Activity) {
                Log.d(TAG, "Activity paused: ${activity.javaClass.simpleName}")
            }

            override fun onActivityStopped(activity: android.app.Activity) {
                activityCount--
                if (activityCount == 0) {
                    isInForeground = false
                    onEnterBackground()
                }
                Log.d(TAG, "Activity stopped: ${activity.javaClass.simpleName}, count=$activityCount")
            }

            override fun onActivitySaveInstanceState(activity: android.app.Activity, outState: android.os.Bundle) {
                Log.d(TAG, "Activity save state: ${activity.javaClass.simpleName}")
            }

            override fun onActivityDestroyed(activity: android.app.Activity) {
                if (activity is AFNSActivity && currentActivity?.get() == activity) {
                    currentActivity = null
                }
                Log.d(TAG, "Activity destroyed: ${activity.javaClass.simpleName}")
            }
        })

        isAppInitialized = true
        Log.i(TAG, "Application initialized successfully")
    }

    override fun onTerminate() {
        super.onTerminate()
        Log.i(TAG, "Application terminating...")

        // Clear singleton
        instance = null
        currentActivity = null

        Log.i(TAG, "Application terminated")
    }

    // ========================================
    // Initialization
    // ========================================

    private fun initializeNativeLibrary() {
        Log.i(TAG, "Initializing native library...")

        try {
            // Native library is loaded in NativeBridge companion object
            // Just verify it's loaded
            if (NativeBridge.isInitialized()) {
                Log.i(TAG, "✓ Native library already initialized")
            } else {
                Log.i(TAG, "Native library loaded, waiting for activity initialization")
            }
        } catch (e: Exception) {
            Log.e(TAG, "✗ Failed to initialize native library", e)
            // Don't crash, let the activity handle initialization
        }
    }

    private fun setupCrashHandler() {
        Log.d(TAG, "Setting up crash handler...")

        val defaultHandler = Thread.getDefaultUncaughtExceptionHandler()

        Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
            Log.e(TAG, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
            Log.e(TAG, "UNCAUGHT EXCEPTION in thread: ${thread.name}")
            Log.e(TAG, "Exception: ${throwable.message}")
            Log.e(TAG, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
            throwable.printStackTrace()

            // Try to shutdown NativeBridge gracefully
            try {
                if (NativeBridge.isInitialized()) {
                    Log.w(TAG, "Attempting graceful native bridge shutdown...")
                    NativeBridge.shutdown()
                }
            } catch (e: Exception) {
                Log.e(TAG, "Failed to shutdown native bridge", e)
            }

            // Call original handler
            defaultHandler?.uncaughtException(thread, throwable)
        }

        Log.d(TAG, "✓ Crash handler setup complete")
    }

    private fun setupMemoryManagement() {
        Log.d(TAG, "Setting up memory management...")

        // Register low memory callback
        registerComponentCallbacks(object : android.content.ComponentCallbacks2 {
            override fun onConfigurationChanged(newConfig: android.content.res.Configuration) {
                Log.d(TAG, "Configuration changed")
            }

            override fun onLowMemory() {
                Log.w(TAG, "⚠️  LOW MEMORY WARNING")
                handleLowMemory()
            }

            override fun onTrimMemory(level: Int) {
                val levelStr = when (level) {
                    android.content.ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN -> "UI_HIDDEN"
                    android.content.ComponentCallbacks2.TRIM_MEMORY_RUNNING_MODERATE -> "RUNNING_MODERATE"
                    android.content.ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW -> "RUNNING_LOW"
                    android.content.ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL -> "RUNNING_CRITICAL"
                    android.content.ComponentCallbacks2.TRIM_MEMORY_BACKGROUND -> "BACKGROUND"
                    android.content.ComponentCallbacks2.TRIM_MEMORY_MODERATE -> "MODERATE"
                    android.content.ComponentCallbacks2.TRIM_MEMORY_COMPLETE -> "COMPLETE"
                    else -> "UNKNOWN($level)"
                }

                Log.w(TAG, "⚠️  TRIM MEMORY: $levelStr")
                handleTrimMemory(level)
            }
        })

        Log.d(TAG, "✓ Memory management setup complete")
    }

    // ========================================
    // Foreground/Background Handling
    // ========================================

    private fun onEnterForeground() {
        Log.i(TAG, "📱 App entered FOREGROUND")
        // Notify native layer if needed
        try {
            if (NativeBridge.isInitialized()) {
                // Could send message to native side about foreground state
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to notify native layer", e)
        }
    }

    private fun onEnterBackground() {
        Log.i(TAG, "📱 App entered BACKGROUND")
        // Notify native layer if needed
        try {
            if (NativeBridge.isInitialized()) {
                // Could trigger GC or save state
                NativeBridge.triggerGC()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to notify native layer", e)
        }
    }

    // ========================================
    // Memory Management
    // ========================================

    private fun handleLowMemory() {
        Log.w(TAG, "Handling low memory situation...")

        try {
            // Trigger garbage collection
            System.gc()

            // Trigger native GC if available
            if (NativeBridge.isInitialized()) {
                NativeBridge.triggerGC()
            }

            // Clear any caches
            clearCaches()

            Log.i(TAG, "✓ Low memory handling complete")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to handle low memory", e)
        }
    }

    private fun handleTrimMemory(level: Int) {
        try {
            when (level) {
                android.content.ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL,
                android.content.ComponentCallbacks2.TRIM_MEMORY_COMPLETE -> {
                    // Critical: aggressive cleanup
                    Log.w(TAG, "Critical memory pressure - aggressive cleanup")
                    System.gc()
                    if (NativeBridge.isInitialized()) {
                        NativeBridge.triggerGC()
                    }
                    clearCaches()
                }

                android.content.ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW,
                android.content.ComponentCallbacks2.TRIM_MEMORY_MODERATE -> {
                    // Moderate: normal cleanup
                    Log.w(TAG, "Moderate memory pressure - normal cleanup")
                    if (NativeBridge.isInitialized()) {
                        NativeBridge.triggerGC()
                    }
                }

                android.content.ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN -> {
                    // UI hidden: background cleanup
                    Log.d(TAG, "UI hidden - background cleanup")
                    clearCaches()
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to trim memory", e)
        }
    }

    private fun clearCaches() {
        Log.d(TAG, "Clearing application caches...")
        // Override this to clear any app-specific caches
    }

    // ========================================
    // Public API
    // ========================================

    /**
     * Get current activity (may be null)
     */
    fun getCurrentActivity(): AFNSActivity? = currentActivity?.get()

    /**
     * Check if app is in foreground
     */
    fun isInForeground(): Boolean = isInForeground

    /**
     * Check if app is initialized
     */
    fun isInitialized(): Boolean = isAppInitialized

    /**
     * Get app uptime in milliseconds
     */
    fun getUptime(): Long = System.currentTimeMillis() - startTime

    /**
     * Get active activity count
     */
    fun getActivityCount(): Int = activityCount

    /**
     * Get application information as map
     */
    fun getApplicationInfo(): Map<String, Any> {
        return mapOf(
            "packageName" to packageName,
            "versionName" to packageManager.getPackageInfo(packageName, 0).versionName,
            "versionCode" to packageManager.getPackageInfo(packageName, 0).longVersionCode,
            "isInitialized" to isAppInitialized,
            "isInForeground" to isInForeground,
            "activityCount" to activityCount,
            "uptime" to getUptime(),
            "nativeBridgeInitialized" to NativeBridge.isInitialized()
        )
    }

    /**
     * Print application status to log
     */
    fun printStatus() {
        Log.i(TAG, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
        Log.i(TAG, "Application Status:")
        Log.i(TAG, "  Initialized: $isAppInitialized")
        Log.i(TAG, "  In Foreground: $isInForeground")
        Log.i(TAG, "  Activity Count: $activityCount")
        Log.i(TAG, "  Uptime: ${getUptime() / 1000}s")
        Log.i(TAG, "  Native Bridge: ${if (NativeBridge.isInitialized()) "Connected" else "Disconnected"}")
        Log.i(TAG, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
    }
}
