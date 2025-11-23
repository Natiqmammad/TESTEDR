package com.nightscript.afns

import android.app.Activity
import android.content.Context
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/**
 * NativeBridge - Core JNI Bridge for ApexForge NightScript Runtime
 *
 * This singleton manages the connection between Android Java/Kotlin code
 * and the NightScript Rust runtime via JNI.
 *
 * Responsibilities:
 * - VM initialization and shutdown
 * - Bidirectional message passing (Platform Channels style)
 * - Callback registration and invocation
 * - Memory management for native objects
 * - Thread safety for native calls
 *
 * Architecture:
 * Kotlin → JNI → Rust → NightScript VM
 */
object NativeBridge {

    private const val TAG = "NativeBridge"

    // Native library name
    private const val LIBRARY_NAME = "nightscript_android"

    // Initialization state
    private val isInitialized = AtomicBoolean(false)
    private val isShutdown = AtomicBoolean(false)

    // Activity reference
    @Volatile
    private var activity: Activity? = null

    @Volatile
    private var applicationContext: Context? = null

    // VM state
    private val vmPointer = AtomicLong(0L)
    private val activityPointer = AtomicLong(0L)

    // Platform channel callbacks
    private val channelCallbacks = ConcurrentHashMap<String, (String) -> String>()

    // Message queue for async communication
    private val messageQueue = ConcurrentHashMap<Long, String>()
    private val messageCounter = AtomicLong(0L)

    init {
        try {
            System.loadLibrary(LIBRARY_NAME)
            println("[$TAG] Native library '$LIBRARY_NAME' loaded successfully")
        } catch (e: UnsatisfiedLinkError) {
            println("[$TAG] ERROR: Failed to load native library: ${e.message}")
            e.printStackTrace()
        }
    }

    // ========================================
    // Native Method Declarations (JNI)
    // ========================================

    /**
     * Initialize the NightScript VM
     * @return VM pointer (opaque handle)
     */
    private external fun nativeInitVM(): Long

    /**
     * Shutdown the NightScript VM
     */
    private external fun nativeShutdownVM(vmPtr: Long)

    /**
     * Execute NightScript code
     * @param vmPtr VM pointer
     * @param code NightScript source code
     * @return Execution result as JSON string
     */
    private external fun nativeExecuteCode(vmPtr: Long, code: String): String

    /**
     * Call a NightScript function by name
     * @param vmPtr VM pointer
     * @param functionName Function name to call
     * @param argsJson Arguments as JSON string
     * @return Result as JSON string
     */
    private external fun nativeCallFunction(vmPtr: Long, functionName: String, argsJson: String): String

    /**
     * Send a message to a platform channel
     * @param vmPtr VM pointer
     * @param channel Channel name
     * @param message Message as JSON string
     * @return Response as JSON string
     */
    private external fun nativeSendMessage(vmPtr: Long, channel: String, message: String): String

    /**
     * Register a callback for a platform channel
     * @param vmPtr VM pointer
     * @param channel Channel name
     */
    private external fun nativeRegisterChannel(vmPtr: Long, channel: String)

    /**
     * Unregister a platform channel callback
     * @param vmPtr VM pointer
     * @param channel Channel name
     */
    private external fun nativeUnregisterChannel(vmPtr: Long, channel: String)

    /**
     * Get global environment pointer for native operations
     * @return JNIEnv pointer
     */
    private external fun nativeGetEnvPointer(): Long

    /**
     * Allocate native memory
     * @param size Size in bytes
     * @return Memory pointer
     */
    private external fun nativeAllocate(size: Long): Long

    /**
     * Free native memory
     * @param ptr Memory pointer
     */
    private external fun nativeFree(ptr: Long)

    /**
     * Get VM memory usage statistics
     * @param vmPtr VM pointer
     * @return JSON with memory stats
     */
    private external fun nativeGetMemoryStats(vmPtr: Long): String

    /**
     * Trigger garbage collection
     * @param vmPtr VM pointer
     */
    private external fun nativeTriggerGC(vmPtr: Long)

    /**
     * Get VM version information
     * @return Version string
     */
    private external fun nativeGetVersion(): String

    /**
     * Set log level for native runtime
     * @param level 0=OFF, 1=ERROR, 2=WARN, 3=INFO, 4=DEBUG, 5=TRACE
     */
    private external fun nativeSetLogLevel(level: Int)

    // ========================================
    // Public API
    // ========================================

    /**
     * Initialize the NativeBridge and NightScript VM
     * Must be called before any other operations
     */
    fun initialize(activity: Activity) {
        if (isInitialized.get()) {
            println("[$TAG] Already initialized")
            return
        }

        if (isShutdown.get()) {
            println("[$TAG] Cannot reinitialize after shutdown")
            return
        }

        try {
            println("[$TAG] Initializing NativeBridge...")

            // Store references
            this.activity = activity
            this.applicationContext = activity.applicationContext

            // Initialize VM
            val vmPtr = nativeInitVM()
            if (vmPtr == 0L) {
                throw RuntimeException("Failed to initialize NightScript VM (returned null pointer)")
            }

            vmPointer.set(vmPtr)
            activityPointer.set(System.identityHashCode(activity).toLong())

            isInitialized.set(true)

            println("[$TAG] NativeBridge initialized successfully")
            println("[$TAG] VM Pointer: $vmPtr")
            println("[$TAG] Activity Pointer: ${activityPointer.get()}")
            println("[$TAG] NightScript Version: ${getVersion()}")

        } catch (e: Exception) {
            println("[$TAG] ERROR: Initialization failed: ${e.message}")
            e.printStackTrace()
            throw e
        }
    }

    /**
     * Shutdown the NativeBridge and NightScript VM
     * Should be called when the activity is destroyed
     */
    fun shutdown() {
        if (!isInitialized.get()) {
            println("[$TAG] Not initialized, skipping shutdown")
            return
        }

        if (isShutdown.get()) {
            println("[$TAG] Already shutdown")
            return
        }

        try {
            println("[$TAG] Shutting down NativeBridge...")

            val vmPtr = vmPointer.get()
            if (vmPtr != 0L) {
                nativeShutdownVM(vmPtr)
                vmPointer.set(0L)
            }

            // Clear callbacks
            channelCallbacks.clear()
            messageQueue.clear()

            // Clear references
            activity = null
            applicationContext = null

            isShutdown.set(true)
            isInitialized.set(false)

            println("[$TAG] NativeBridge shutdown complete")

        } catch (e: Exception) {
            println("[$TAG] ERROR: Shutdown failed: ${e.message}")
            e.printStackTrace()
        }
    }

    /**
     * Execute NightScript code
     * @param code NightScript source code
     * @return Execution result as JSON string
     */
    fun executeCode(code: String): String {
        checkInitialized()

        return try {
            val vmPtr = vmPointer.get()
            val result = nativeExecuteCode(vmPtr, code)
            println("[$TAG] Executed code, result: $result")
            result
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to execute code: ${e.message}")
            """{"error": "${e.message}"}"""
        }
    }

    /**
     * Call a NightScript function by name
     * @param functionName Function name
     * @param argsJson Arguments as JSON string
     * @return Result as JSON string
     */
    fun callFunction(functionName: String, argsJson: String = "{}"): String {
        checkInitialized()

        return try {
            val vmPtr = vmPointer.get()
            val result = nativeCallFunction(vmPtr, functionName, argsJson)
            println("[$TAG] Called function '$functionName', result: $result")
            result
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to call function '$functionName': ${e.message}")
            """{"error": "${e.message}"}"""
        }
    }

    /**
     * Send a message to a platform channel
     * @param channel Channel name
     * @param message Message as JSON string
     * @return Response as JSON string
     */
    fun sendMessage(channel: String, message: String): String {
        checkInitialized()

        return try {
            val vmPtr = vmPointer.get()
            val result = nativeSendMessage(vmPtr, channel, message)
            println("[$TAG] Sent message to channel '$channel', response: $result")
            result
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to send message to '$channel': ${e.message}")
            """{"error": "${e.message}"}"""
        }
    }

    /**
     * Register a callback for a platform channel
     * Platform channels allow bidirectional communication between
     * NightScript code and Android code.
     *
     * @param channel Channel name
     * @param callback Callback function (message -> response)
     */
    fun registerCallback(channel: String, callback: (String) -> String) {
        checkInitialized()

        try {
            println("[$TAG] Registering callback for channel: $channel")
            channelCallbacks[channel] = callback

            val vmPtr = vmPointer.get()
            nativeRegisterChannel(vmPtr, channel)

            println("[$TAG] Callback registered for channel: $channel")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to register callback for '$channel': ${e.message}")
        }
    }

    /**
     * Unregister a platform channel callback
     * @param channel Channel name
     */
    fun unregisterCallback(channel: String) {
        checkInitialized()

        try {
            println("[$TAG] Unregistering callback for channel: $channel")
            channelCallbacks.remove(channel)

            val vmPtr = vmPointer.get()
            nativeUnregisterChannel(vmPtr, channel)

            println("[$TAG] Callback unregistered for channel: $channel")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to unregister callback for '$channel': ${e.message}")
        }
    }

    /**
     * Get memory usage statistics
     * @return JSON with memory stats
     */
    fun getMemoryStats(): String {
        checkInitialized()

        return try {
            val vmPtr = vmPointer.get()
            nativeGetMemoryStats(vmPtr)
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to get memory stats: ${e.message}")
            """{"error": "${e.message}"}"""
        }
    }

    /**
     * Trigger garbage collection
     */
    fun triggerGC() {
        checkInitialized()

        try {
            val vmPtr = vmPointer.get()
            nativeTriggerGC(vmPtr)
            println("[$TAG] Garbage collection triggered")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to trigger GC: ${e.message}")
        }
    }

    /**
     * Get NightScript version
     * @return Version string
     */
    fun getVersion(): String {
        return try {
            nativeGetVersion()
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to get version: ${e.message}")
            "unknown"
        }
    }

    /**
     * Set log level for native runtime
     * @param level 0=OFF, 1=ERROR, 2=WARN, 3=INFO, 4=DEBUG, 5=TRACE
     */
    fun setLogLevel(level: Int) {
        try {
            nativeSetLogLevel(level)
            println("[$TAG] Log level set to: $level")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to set log level: ${e.message}")
        }
    }

    /**
     * Get VM pointer (for internal use by Activity)
     */
    fun getVMPointer(): Long = vmPointer.get()

    /**
     * Get activity pointer (for internal use by JNI)
     */
    fun getActivityPointer(): Long = activityPointer.get()

    /**
     * Get JNIEnv pointer (for advanced JNI operations)
     */
    fun getEnvPointer(): Long {
        return try {
            nativeGetEnvPointer()
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to get env pointer: ${e.message}")
            0L
        }
    }

    /**
     * Get current activity
     */
    fun getActivity(): Activity? = activity

    /**
     * Get application context
     */
    fun getApplicationContext(): Context? = applicationContext

    /**
     * Check if initialized
     */
    fun isInitialized(): Boolean = isInitialized.get()

    /**
     * Check if shutdown
     */
    fun isShutdown(): Boolean = isShutdown.get()

    // ========================================
    // Callback from Native Code (JNI)
    // ========================================

    /**
     * Called from native code when a platform channel message is received
     * This is invoked from the Rust side via JNI
     *
     * @param channel Channel name
     * @param message Message as JSON string
     * @return Response as JSON string
     */
    @JvmStatic
    fun onChannelMessage(channel: String, message: String): String {
        println("[$TAG] Received message on channel '$channel': $message")

        return try {
            val callback = channelCallbacks[channel]
            if (callback != null) {
                val response = callback(message)
                println("[$TAG] Channel '$channel' callback returned: $response")
                response
            } else {
                println("[$TAG] WARNING: No callback registered for channel '$channel'")
                """{"error": "No callback registered for channel '$channel'"}"""
            }
        } catch (e: Exception) {
            println("[$TAG] ERROR: Channel callback failed: ${e.message}")
            e.printStackTrace()
            """{"error": "${e.message}"}"""
        }
    }

    /**
     * Called from native code to log messages
     * @param level Log level (0=ERROR, 1=WARN, 2=INFO, 3=DEBUG)
     * @param message Log message
     */
    @JvmStatic
    fun onNativeLog(level: Int, message: String) {
        val levelStr = when (level) {
            0 -> "ERROR"
            1 -> "WARN"
            2 -> "INFO"
            3 -> "DEBUG"
            else -> "TRACE"
        }
        println("[$TAG] [NATIVE-$levelStr] $message")
    }

    /**
     * Called from native code when a panic occurs
     * @param message Panic message
     */
    @JvmStatic
    fun onNativePanic(message: String) {
        println("[$TAG] !!!NATIVE PANIC!!! $message")
        System.err.println("[$TAG] !!!NATIVE PANIC!!! $message")
        // In production, you might want to handle this more gracefully
        // For now, just log it
    }

    // ========================================
    // Private Helpers
    // ========================================

    private fun checkInitialized() {
        if (!isInitialized.get()) {
            throw IllegalStateException("NativeBridge not initialized. Call initialize() first.")
        }
        if (isShutdown.get()) {
            throw IllegalStateException("NativeBridge has been shutdown and cannot be reused.")
        }
    }
}
