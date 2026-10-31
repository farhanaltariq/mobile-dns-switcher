package com.sicloud.dnsswitcher

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.util.Log

object VpnHelper {
    private const val TAG = "VpnHelper"
    const val VPN_REQUEST_CODE = 2001

    @Volatile
    var currentActivity: Activity? = null

    @Volatile
    var pendingDns: String? = null

    @JvmStatic
    fun startVpn(dnsHost: String): Boolean {
        val act = currentActivity
        if (act == null) {
            Log.e(TAG, "Cannot start VPN: currentActivity is null")
            return false
        }
        pendingDns = dnsHost
        val intent = VpnService.prepare(act)
        return if (intent != null) {
            Log.i(TAG, "Requesting user VPN permission via system dialog...")
            act.startActivityForResult(intent, VPN_REQUEST_CODE)
            false
        } else {
            Log.i(TAG, "VPN permission already granted, starting service...")
            startService(act, dnsHost)
            true
        }
    }

    @JvmStatic
    fun stopVpn(): Boolean {
        val act = currentActivity ?: return false
        val intent = Intent(act, DnsVpnService::class.java).apply {
            action = DnsVpnService.ACTION_STOP
        }
        act.startService(intent)
        return true
    }

    @JvmStatic
    fun isStrictPrivateDns(): Boolean {
        val ctx = currentActivity ?: return false
        return try {
            android.provider.Settings.Global.getString(
                ctx.contentResolver,
                "private_dns_mode"
            ) == "hostname"
        } catch (_: Exception) {
            false
        }
    }

    @JvmStatic
    fun isRunning(): Boolean {
        return DnsVpnService.isRunning
    }

    @JvmStatic
    fun getActiveDns(): String {
        return DnsVpnService.activeDns ?: ""
    }

    fun startService(context: Context, dnsHost: String) {
        val intent = Intent(context, DnsVpnService::class.java).apply {
            action = DnsVpnService.ACTION_START
            putExtra(DnsVpnService.EXTRA_DNS, dnsHost)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(intent)
        } else {
            context.startService(intent)
        }
    }
}
