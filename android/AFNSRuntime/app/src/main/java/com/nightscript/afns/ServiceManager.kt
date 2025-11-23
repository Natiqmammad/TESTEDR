package com.nightscript.afns

import android.app.Service
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import java.util.concurrent.ConcurrentHashMap

/**
 * ServiceManager - Android Service Management for ApexForge NightScript
 *
 * Handles:
 * - Starting and stopping services
 * - Foreground services (Android 8.0+)
 * - Background services
 * - Service binding and unbinding
 * - Service lifecycle tracking
 * - Service communication
 *
 * Service Types:
 * - Started Service: Runs in background, independent of components
 * - Bound Service: Provides client-server interface for interaction
 * - Foreground Service: User-visible service with notification (Android 8.0+)
 */
class ServiceManager(private val context: Context) {

    companion object {
        private const val TAG = "ServiceManager"
    }

    // Active services tracking
    private val activeServices = ConcurrentHashMap<String, ServiceInfo>()

    // Service connections for bound services
    private val serviceConnections = ConcurrentHashMap<String, ServiceConnectionWrapper>()

    /**
     * Data class for service information
     */
    private data class ServiceInfo(
        val className: String,
        val isRunning: Boolean,
        val isForeground: Boolean,
        val startTime: Long = System.currentTimeMillis()
    )

    /**
     * Wrapper for ServiceConnection with callbacks
     */
    private inner class ServiceConnectionWrapper(
        val onConnected: (ComponentName, IBinder) -> Unit,
        val onDisconnected: (ComponentName) -> Unit
    ) : ServiceConnection {
        override fun onServiceConnected(name: ComponentName, service: IBinder) {
            println("[$TAG] Service connected: ${name.className}")
            onConnected(name, service)
        }

        override fun onServiceDisconnected(name: ComponentName) {
            println("[$TAG] Service disconnected: ${name.className}")
            onDisconnected(name)
        }
    }

    // ========================================
    // Service Starting/Stopping
    // ========================================

    /**
     * Start a service
     * @param serviceClassName Fully qualified service class name
     * @param extras Optional extras to pass to service
     */
    fun startService(serviceClassName: String, extras: Map<String, String> = emptyMap()) {
        println("[$TAG] Starting service: $serviceClassName")

        try {
            val serviceClass = Class.forName(serviceClassName)
            val intent = Intent(context, serviceClass).apply {
                extras.forEach { (key, value) ->
                    putExtra(key, value)
                }
            }

            context.startService(intent)

            // Track service
            activeServices[serviceClassName] = ServiceInfo(
                className = serviceClassName,
                isRunning = true,
                isForeground = false
            )

            println("[$TAG] Service started successfully")
        } catch (e: ClassNotFoundException) {
            println("[$TAG] ERROR: Service class not found: $serviceClassName")
            e.printStackTrace()
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to start service: ${e.message}")
            e.printStackTrace()
        }
    }

    /**
     * Start a foreground service (Android 8.0+)
     * Foreground services must show a notification
     *
     * @param serviceClassName Fully qualified service class name
     * @param extras Optional extras to pass to service
     */
    fun startForegroundService(serviceClassName: String, extras: Map<String, String> = emptyMap()) {
        println("[$TAG] Starting foreground service: $serviceClassName")

        try {
            val serviceClass = Class.forName(serviceClassName)
            val intent = Intent(context, serviceClass).apply {
                extras.forEach { (key, value) ->
                    putExtra(key, value)
                }
                // Add flag to indicate foreground service
                putExtra("IS_FOREGROUND_SERVICE", true)
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }

            // Track service
            activeServices[serviceClassName] = ServiceInfo(
                className = serviceClassName,
                isRunning = true,
                isForeground = true
            )

            println("[$TAG] Foreground service started successfully")
        } catch (e: ClassNotFoundException) {
            println("[$TAG] ERROR: Service class not found: $serviceClassName")
            e.printStackTrace()
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to start foreground service: ${e.message}")
            e.printStackTrace()
        }
    }

    /**
     * Stop a service
     * @param serviceClassName Fully qualified service class name
     */
    fun stopService(serviceClassName: String) {
        println("[$TAG] Stopping service: $serviceClassName")

        try {
            val serviceClass = Class.forName(serviceClassName)
            val intent = Intent(context, serviceClass)

            context.stopService(intent)

            // Remove from tracking
            activeServices.remove(serviceClassName)

            println("[$TAG] Service stopped successfully")
        } catch (e: ClassNotFoundException) {
            println("[$TAG] ERROR: Service class not found: $serviceClassName")
            e.printStackTrace()
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to stop service: ${e.message}")
            e.printStackTrace()
        }
    }

    // ========================================
    // Service Binding
    // ========================================

    /**
     * Bind to a service
     * @param serviceClassName Fully qualified service class name
     * @param onConnected Callback when service is connected
     * @param onDisconnected Callback when service is disconnected
     * @param flags Binding flags (default: BIND_AUTO_CREATE)
     */
    fun bindService(
        serviceClassName: String,
        onConnected: (ComponentName, IBinder) -> Unit,
        onDisconnected: (ComponentName) -> Unit,
        flags: Int = Context.BIND_AUTO_CREATE
    ): Boolean {
        println("[$TAG] Binding to service: $serviceClassName")

        try {
            val serviceClass = Class.forName(serviceClassName)
            val intent = Intent(context, serviceClass)

            val connection = ServiceConnectionWrapper(onConnected, onDisconnected)
            val success = context.bindService(intent, connection, flags)

            if (success) {
                serviceConnections[serviceClassName] = connection
                println("[$TAG] Service bound successfully")
            } else {
                println("[$TAG] WARNING: Failed to bind service")
            }

            return success
        } catch (e: ClassNotFoundException) {
            println("[$TAG] ERROR: Service class not found: $serviceClassName")
            e.printStackTrace()
            return false
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to bind service: ${e.message}")
            e.printStackTrace()
            return false
        }
    }

    /**
     * Unbind from a service
     * @param serviceClassName Fully qualified service class name
     */
    fun unbindService(serviceClassName: String) {
        println("[$TAG] Unbinding from service: $serviceClassName")

        try {
            val connection = serviceConnections.remove(serviceClassName)
            if (connection != null) {
                context.unbindService(connection)
                println("[$TAG] Service unbound successfully")
            } else {
                println("[$TAG] WARNING: No active binding found for service")
            }
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to unbind service: ${e.message}")
            e.printStackTrace()
        }
    }

    // ========================================
    // Service Status
    // ========================================

    /**
     * Check if a service is running
     * @param serviceClassName Fully qualified service class name
     * @return true if service is running
     */
    fun isServiceRunning(serviceClassName: String): Boolean {
        return activeServices[serviceClassName]?.isRunning ?: false
    }

    /**
     * Check if a service is a foreground service
     * @param serviceClassName Fully qualified service class name
     * @return true if service is foreground
     */
    fun isForegroundService(serviceClassName: String): Boolean {
        return activeServices[serviceClassName]?.isForeground ?: false
    }

    /**
     * Get service uptime in milliseconds
     * @param serviceClassName Fully qualified service class name
     * @return Uptime in milliseconds, or -1 if service not running
     */
    fun getServiceUptime(serviceClassName: String): Long {
        val serviceInfo = activeServices[serviceClassName] ?: return -1
        return if (serviceInfo.isRunning) {
            System.currentTimeMillis() - serviceInfo.startTime
        } else {
            -1
        }
    }

    /**
     * Get list of all active services
     * @return List of service class names
     */
    fun getActiveServices(): List<String> {
        return activeServices.keys.toList()
    }

    /**
     * Get number of active services
     * @return Number of active services
     */
    fun getActiveServiceCount(): Int {
        return activeServices.size
    }

    /**
     * Get number of bound services
     * @return Number of bound services
     */
    fun getBoundServiceCount(): Int {
        return serviceConnections.size
    }

    // ========================================
    // Batch Operations
    // ========================================

    /**
     * Stop all active services
     */
    fun stopAllServices() {
        println("[$TAG] Stopping all services (${activeServices.size} services)")

        val servicesToStop = activeServices.keys.toList()
        servicesToStop.forEach { serviceClassName ->
            stopService(serviceClassName)
        }

        println("[$TAG] All services stopped")
    }

    /**
     * Unbind all bound services
     */
    fun unbindAllServices() {
        println("[$TAG] Unbinding all services (${serviceConnections.size} connections)")

        val servicesToUnbind = serviceConnections.keys.toList()
        servicesToUnbind.forEach { serviceClassName ->
            unbindService(serviceClassName)
        }

        println("[$TAG] All services unbound")
    }

    // ========================================
    // Service Communication
    // ========================================

    /**
     * Send broadcast to a service
     * @param action Broadcast action
     * @param extras Optional extras
     */
    fun sendBroadcastToService(action: String, extras: Map<String, String> = emptyMap()) {
        println("[$TAG] Sending broadcast: $action")

        try {
            val intent = Intent(action).apply {
                extras.forEach { (key, value) ->
                    putExtra(key, value)
                }
            }
            context.sendBroadcast(intent)
            println("[$TAG] Broadcast sent successfully")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to send broadcast: ${e.message}")
            e.printStackTrace()
        }
    }

    // ========================================
    // Utility Methods
    // ========================================

    /**
     * Clear service tracking (useful for cleanup)
     */
    fun clearTracking() {
        println("[$TAG] Clearing service tracking")
        activeServices.clear()
    }

    /**
     * Get service information
     * @param serviceClassName Service class name
     * @return ServiceInfo or null if not found
     */
    fun getServiceInfo(serviceClassName: String): Map<String, Any>? {
        val info = activeServices[serviceClassName] ?: return null
        return mapOf(
            "className" to info.className,
            "isRunning" to info.isRunning,
            "isForeground" to info.isForeground,
            "startTime" to info.startTime,
            "uptime" to (System.currentTimeMillis() - info.startTime)
        )
    }

    /**
     * Get all service information
     * @return Map of service class name to service info
     */
    fun getAllServiceInfo(): Map<String, Map<String, Any>> {
        return activeServices.mapValues { (className, _) ->
            getServiceInfo(className) ?: emptyMap()
        }
    }
}
