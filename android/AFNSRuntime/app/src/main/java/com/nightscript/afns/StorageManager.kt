package com.nightscript.afns

import android.content.Context
import android.os.Build
import android.os.Environment
import android.os.StatFs
import java.io.File
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.*

/**
 * StorageManager - File System and Storage Management for ApexForge NightScript
 *
 * Handles:
 * - Internal storage access
 * - External storage access (SD card, USB)
 * - Cache directory management
 * - Temporary file creation
 * - Storage space queries
 * - File system utilities
 * - Storage permissions checking
 *
 * Storage Types:
 * - Internal Storage: Private to app, always available
 * - External Storage: Shared storage, may be removable
 * - Cache: Temporary storage, can be cleared by system
 * - Files: Persistent app data
 *
 * Android Storage Scopes (Android 10+):
 * - Scoped Storage enforced on Android 10+
 * - App-specific directories don't require permissions
 * - Shared storage requires READ_EXTERNAL_STORAGE or WRITE_EXTERNAL_STORAGE
 */
class StorageManager(private val context: Context) {

    companion object {
        private const val TAG = "StorageManager"

        // Storage types
        const val STORAGE_INTERNAL = "internal"
        const val STORAGE_EXTERNAL = "external"
        const val STORAGE_CACHE = "cache"
        const val STORAGE_FILES = "files"

        // Directory names
        const val DIR_DOCUMENTS = "Documents"
        const val DIR_DOWNLOADS = "Downloads"
        const val DIR_PICTURES = "Pictures"
        const val DIR_MOVIES = "Movies"
        const val DIR_MUSIC = "Music"
        const val DIR_DCIM = "DCIM"

        // Size units
        const val BYTES_PER_KB = 1024L
        const val BYTES_PER_MB = 1024L * 1024L
        const val BYTES_PER_GB = 1024L * 1024L * 1024L
    }

    // ========================================
    // Path Access
    // ========================================

    /**
     * Get internal storage path (private to app)
     * No permissions required, always available
     * Example: /data/user/0/com.nightscript.app/files
     */
    fun getInternalStoragePath(): String {
        return context.filesDir.absolutePath
    }

    /**
     * Get external storage path (app-specific, Android 10+ compatible)
     * No permissions required for app-specific external storage
     * Example: /storage/emulated/0/Android/data/com.nightscript.app/files
     */
    fun getExternalStoragePath(): String {
        return context.getExternalFilesDir(null)?.absolutePath
            ?: getInternalStoragePath()
    }

    /**
     * Get cache directory path
     * Temporary storage, can be cleared by system when low on space
     * Example: /data/user/0/com.nightscript.app/cache
     */
    fun getCacheDir(): String {
        return context.cacheDir.absolutePath
    }

    /**
     * Get external cache directory path
     * Example: /storage/emulated/0/Android/data/com.nightscript.app/cache
     */
    fun getExternalCacheDir(): String {
        return context.externalCacheDir?.absolutePath
            ?: getCacheDir()
    }

    /**
     * Get files directory path (same as internal storage)
     */
    fun getFilesDir(): String {
        return context.filesDir.absolutePath
    }

    /**
     * Get code cache directory path (for optimized code)
     * Example: /data/user/0/com.nightscript.app/code_cache
     */
    fun getCodeCacheDir(): String {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            context.codeCacheDir.absolutePath
        } else {
            File(getCacheDir(), "code_cache").apply { mkdirs() }.absolutePath
        }
    }

    /**
     * Get no-backup files directory
     * Files here won't be backed up by auto-backup
     * Example: /data/user/0/com.nightscript.app/no_backup
     */
    fun getNoBackupFilesDir(): String {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            context.noBackupFilesDir.absolutePath
        } else {
            File(getInternalStoragePath(), "no_backup").apply { mkdirs() }.absolutePath
        }
    }

    /**
     * Get public external storage directory (requires permissions on Android 10+)
     * @param type Directory type (e.g., DIR_DOCUMENTS, DIR_PICTURES)
     */
    fun getPublicExternalStorageDir(type: String): String? {
        return if (hasStoragePermission()) {
            Environment.getExternalStoragePublicDirectory(type)?.absolutePath
        } else {
            null
        }
    }

    // ========================================
    // Temporary Files
    // ========================================

    /**
     * Create a temporary file
     * @param prefix File name prefix
     * @param suffix File name suffix (e.g., ".tmp", ".txt")
     * @return File object
     */
    fun createTempFile(prefix: String, suffix: String): File {
        println("[$TAG] Creating temp file: prefix=$prefix, suffix=$suffix")

        return try {
            val file = File.createTempFile(prefix, suffix, context.cacheDir)
            println("[$TAG] Temp file created: ${file.absolutePath}")
            file
        } catch (e: IOException) {
            println("[$TAG] ERROR: Failed to create temp file: ${e.message}")
            throw e
        }
    }

    /**
     * Create a temporary file with timestamp
     * @param prefix File name prefix
     * @param suffix File name suffix
     * @return File object
     */
    fun createTempFileWithTimestamp(prefix: String, suffix: String): File {
        val timestamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.US).format(Date())
        val fileName = "${prefix}_${timestamp}"
        return createTempFile(fileName, suffix)
    }

    /**
     * Create a temporary directory
     * @param prefix Directory name prefix
     * @return File object
     */
    fun createTempDir(prefix: String): File {
        println("[$TAG] Creating temp directory: prefix=$prefix")

        val timestamp = System.currentTimeMillis()
        val dir = File(context.cacheDir, "${prefix}_${timestamp}")

        if (dir.mkdirs() || dir.exists()) {
            println("[$TAG] Temp directory created: ${dir.absolutePath}")
            return dir
        } else {
            throw IOException("Failed to create temp directory: ${dir.absolutePath}")
        }
    }

    // ========================================
    // Storage Space Queries
    // ========================================

    /**
     * Get total internal storage space in bytes
     */
    fun getTotalInternalSpace(): Long {
        return try {
            val stat = StatFs(context.filesDir.absolutePath)
            val blockSize = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.blockSizeLong
            } else {
                stat.blockSize.toLong()
            }
            val totalBlocks = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.blockCountLong
            } else {
                stat.blockCount.toLong()
            }
            blockSize * totalBlocks
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to get total internal space: ${e.message}")
            -1L
        }
    }

    /**
     * Get available internal storage space in bytes
     */
    fun getAvailableInternalSpace(): Long {
        return try {
            val stat = StatFs(context.filesDir.absolutePath)
            val blockSize = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.blockSizeLong
            } else {
                stat.blockSize.toLong()
            }
            val availableBlocks = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.availableBlocksLong
            } else {
                stat.availableBlocks.toLong()
            }
            blockSize * availableBlocks
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to get available internal space: ${e.message}")
            -1L
        }
    }

    /**
     * Get total external storage space in bytes
     */
    fun getTotalExternalSpace(): Long {
        return try {
            val externalDir = context.getExternalFilesDir(null) ?: return -1L
            val stat = StatFs(externalDir.absolutePath)
            val blockSize = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.blockSizeLong
            } else {
                stat.blockSize.toLong()
            }
            val totalBlocks = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.blockCountLong
            } else {
                stat.blockCount.toLong()
            }
            blockSize * totalBlocks
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to get total external space: ${e.message}")
            -1L
        }
    }

    /**
     * Get available external storage space in bytes
     */
    fun getAvailableExternalSpace(): Long {
        return try {
            val externalDir = context.getExternalFilesDir(null) ?: return -1L
            val stat = StatFs(externalDir.absolutePath)
            val blockSize = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.blockSizeLong
            } else {
                stat.blockSize.toLong()
            }
            val availableBlocks = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.JELLY_BEAN_MR2) {
                stat.availableBlocksLong
            } else {
                stat.availableBlocks.toLong()
            }
            blockSize * availableBlocks
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to get available external space: ${e.message}")
            -1L
        }
    }

    /**
     * Get cache size in bytes
     */
    fun getCacheSize(): Long {
        return calculateDirectorySize(context.cacheDir)
    }

    /**
     * Get total app storage usage in bytes (files + cache)
     */
    fun getTotalAppStorageUsage(): Long {
        val filesSize = calculateDirectorySize(context.filesDir)
        val cacheSize = calculateDirectorySize(context.cacheDir)
        return filesSize + cacheSize
    }

    // ========================================
    // Storage Status
    // ========================================

    /**
     * Check if external storage is available
     */
    fun isExternalStorageAvailable(): Boolean {
        val state = Environment.getExternalStorageState()
        return state == Environment.MEDIA_MOUNTED
    }

    /**
     * Check if external storage is read-only
     */
    fun isExternalStorageReadOnly(): Boolean {
        val state = Environment.getExternalStorageState()
        return state == Environment.MEDIA_MOUNTED_READ_ONLY
    }

    /**
     * Check if external storage is writable
     */
    fun isExternalStorageWritable(): Boolean {
        val state = Environment.getExternalStorageState()
        return state == Environment.MEDIA_MOUNTED
    }

    /**
     * Check if has storage permission
     * Note: Not needed for app-specific directories on Android 10+
     */
    fun hasStoragePermission(): Boolean {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            Environment.isExternalStorageManager()
        } else {
            true // Assume granted for older versions
        }
    }

    /**
     * Check if low on storage space
     * @param thresholdMB Threshold in megabytes
     */
    fun isLowOnSpace(thresholdMB: Long = 100): Boolean {
        val availableSpace = getAvailableInternalSpace()
        return availableSpace < thresholdMB * BYTES_PER_MB
    }

    // ========================================
    // Cache Management
    // ========================================

    /**
     * Clear all cache files
     * @return true if successful
     */
    fun clearCache(): Boolean {
        println("[$TAG] Clearing cache")

        return try {
            val cacheDir = context.cacheDir
            val externalCacheDir = context.externalCacheDir

            var success = deleteDirectory(cacheDir)
            externalCacheDir?.let {
                success = success && deleteDirectory(it)
            }

            // Recreate directories
            cacheDir.mkdirs()
            externalCacheDir?.mkdirs()

            println("[$TAG] Cache cleared: success=$success")
            success
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to clear cache: ${e.message}")
            false
        }
    }

    /**
     * Clear old cache files (older than specified days)
     * @param daysOld Files older than this will be deleted
     * @return Number of files deleted
     */
    fun clearOldCache(daysOld: Int = 7): Int {
        println("[$TAG] Clearing cache older than $daysOld days")

        val threshold = System.currentTimeMillis() - (daysOld * 24 * 60 * 60 * 1000L)
        var deletedCount = 0

        try {
            context.cacheDir.listFiles()?.forEach { file ->
                if (file.lastModified() < threshold) {
                    if (deleteRecursive(file)) {
                        deletedCount++
                    }
                }
            }

            context.externalCacheDir?.listFiles()?.forEach { file ->
                if (file.lastModified() < threshold) {
                    if (deleteRecursive(file)) {
                        deletedCount++
                    }
                }
            }

            println("[$TAG] Deleted $deletedCount old cache files")
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to clear old cache: ${e.message}")
        }

        return deletedCount
    }

    // ========================================
    // Utility Methods
    // ========================================

    /**
     * Calculate directory size recursively
     */
    private fun calculateDirectorySize(directory: File): Long {
        var size = 0L

        if (!directory.exists()) {
            return 0L
        }

        try {
            if (directory.isFile) {
                return directory.length()
            }

            directory.listFiles()?.forEach { file ->
                size += if (file.isDirectory) {
                    calculateDirectorySize(file)
                } else {
                    file.length()
                }
            }
        } catch (e: Exception) {
            println("[$TAG] ERROR: Failed to calculate directory size: ${e.message}")
        }

        return size
    }

    /**
     * Delete directory and all contents
     */
    private fun deleteDirectory(directory: File): Boolean {
        if (!directory.exists()) {
            return true
        }

        return deleteRecursive(directory)
    }

    /**
     * Delete file or directory recursively
     */
    private fun deleteRecursive(file: File): Boolean {
        if (file.isDirectory) {
            file.listFiles()?.forEach { child ->
                deleteRecursive(child)
            }
        }
        return file.delete()
    }

    /**
     * Format bytes to human-readable string
     */
    fun formatBytes(bytes: Long): String {
        return when {
            bytes < 0 -> "Unknown"
            bytes < BYTES_PER_KB -> "$bytes B"
            bytes < BYTES_PER_MB -> String.format("%.2f KB", bytes.toDouble() / BYTES_PER_KB)
            bytes < BYTES_PER_GB -> String.format("%.2f MB", bytes.toDouble() / BYTES_PER_MB)
            else -> String.format("%.2f GB", bytes.toDouble() / BYTES_PER_GB)
        }
    }

    /**
     * Get storage information as map
     */
    fun getStorageInfo(): Map<String, Any> {
        return mapOf(
            "internalPath" to getInternalStoragePath(),
            "externalPath" to getExternalStoragePath(),
            "cachePath" to getCacheDir(),
            "totalInternal" to getTotalInternalSpace(),
            "availableInternal" to getAvailableInternalSpace(),
            "totalExternal" to getTotalExternalSpace(),
            "availableExternal" to getAvailableExternalSpace(),
            "cacheSize" to getCacheSize(),
            "totalAppUsage" to getTotalAppStorageUsage(),
            "externalAvailable" to isExternalStorageAvailable(),
            "externalWritable" to isExternalStorageWritable(),
            "hasPermission" to hasStoragePermission(),
            "lowOnSpace" to isLowOnSpace()
        )
    }

    /**
     * Get storage information as formatted string
     */
    fun getStorageInfoString(): String {
        return buildString {
            appendLine("Storage Information:")
            appendLine("  Internal Path: ${getInternalStoragePath()}")
            appendLine("  External Path: ${getExternalStoragePath()}")
            appendLine("  Cache Path: ${getCacheDir()}")
            appendLine("  Internal: ${formatBytes(getAvailableInternalSpace())} / ${formatBytes(getTotalInternalSpace())} available")
            appendLine("  External: ${formatBytes(getAvailableExternalSpace())} / ${formatBytes(getTotalExternalSpace())} available")
            appendLine("  Cache Size: ${formatBytes(getCacheSize())}")
            appendLine("  Total App Usage: ${formatBytes(getTotalAppStorageUsage())}")
            appendLine("  External Available: ${isExternalStorageAvailable()}")
            appendLine("  External Writable: ${isExternalStorageWritable()}")
            appendLine("  Has Permission: ${hasStoragePermission()}")
            appendLine("  Low on Space: ${isLowOnSpace()}")
        }
    }
}
