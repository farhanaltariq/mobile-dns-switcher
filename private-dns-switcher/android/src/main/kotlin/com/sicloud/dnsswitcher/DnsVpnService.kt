package com.sicloud.dnsswitcher

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor

class DnsVpnService : VpnService() {
    companion object {
        const val ACTION_START = "com.sicloud.dnsswitcher.START_VPN"
        const val ACTION_STOP = "com.sicloud.dnsswitcher.STOP_VPN"
        const val EXTRA_DNS = "extra_dns"

        @Volatile
        var isRunning = false
            private set

        @Volatile
        var activeDns: String? = null
            private set

        init {
            try {
                System.loadLibrary("private_dns_switcher")
            } catch (_: UnsatisfiedLinkError) {}
        }
    }

    private var vpnInterface: ParcelFileDescriptor? = null

    private external fun nativeRunTunnel(tunFd: Int, dnsHost: String)
    private external fun nativeStopTunnel()

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val prefs = getSharedPreferences("dns_vpn_prefs", Context.MODE_PRIVATE)
        if (intent == null) {
            val savedDns = prefs.getString("last_dns", null)
            if (savedDns != null) {
                startTunnel(savedDns)
                return START_STICKY
            }
            return START_NOT_STICKY
        }
        when (intent.action) {
            ACTION_STOP -> stopTunnel()
            ACTION_START -> {
                val dns = intent.getStringExtra(EXTRA_DNS) ?: "dns.quad9.net"
                startTunnel(dns)
                return START_STICKY
            }
        }
        return START_NOT_STICKY
    }

    override fun onTaskRemoved(rootIntent: Intent?) {
        // Keep VPN tunnel running even if user swipes away UI from Recents
    }

    private fun startTunnel(dnsHost: String) {
        if (isRunning && activeDns == dnsHost) return
        closeCurrentTunnel()

        activeDns = dnsHost
        isRunning = true
        getSharedPreferences("dns_vpn_prefs", Context.MODE_PRIVATE)
            .edit()
            .putString("last_dns", dnsHost)
            .apply()
        startNotification(dnsHost)

        val pfd = Builder()
            .setSession("DNS Switcher ($dnsHost)")
            .setMtu(1500)
            .addAddress("10.111.222.1", 30)
            .addDnsServer("10.111.222.2")
            .addRoute("10.111.222.2", 32)  // split routing: only DNS traffic goes through the tunnel
            .setBlocking(true)
            .establish() ?: run {
                stopTunnel()
                return
            }

        vpnInterface = pfd
        val fd = pfd.fd

        Thread({
            nativeRunTunnel(fd, dnsHost)
            if (isRunning && activeDns == dnsHost) {
                stopTunnel()
            }
        }, "RustTunnel").start()
    }

    private fun startNotification(dnsHost: String) {
        val cid = "dns_vpn_channel"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val chan = NotificationChannel(cid, "DNS Switcher", NotificationManager.IMPORTANCE_LOW)
            getSystemService(NotificationManager::class.java)?.createNotificationChannel(chan)
        }
        val pi = packageManager.getLaunchIntentForPackage(packageName)?.let {
            PendingIntent.getActivity(this, 0, it, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        }
        val nb = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            android.app.Notification.Builder(this, cid)
        } else {
            @Suppress("DEPRECATION") android.app.Notification.Builder(this)
        }
        val notif = nb.setContentTitle("DNS Switcher Active")
            .setContentText("Connected to $dnsHost")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentIntent(pi)
            .setOngoing(true)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(1001, notif, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(1001, notif)
        }
    }

    private fun closeCurrentTunnel() {
        nativeStopTunnel()
        try { vpnInterface?.close() } catch (_: Exception) {}
        vpnInterface = null
    }

    private fun stopTunnel() {
        closeCurrentTunnel()
        isRunning = false
        activeDns = null
        getSharedPreferences("dns_vpn_prefs", Context.MODE_PRIVATE)
            .edit()
            .remove("last_dns")
            .apply()
        stopForeground(true)
        stopSelf()
    }

    override fun onDestroy() {
        stopTunnel()
        super.onDestroy()
    }
}
