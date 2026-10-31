package com.sicloud.dnsswitcher

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import java.io.DataInputStream
import java.io.DataOutputStream
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.Socket
import java.util.concurrent.Executors
import javax.net.ssl.SSLSocket
import javax.net.ssl.SSLSocketFactory

class DnsVpnService : VpnService() {
    companion object {
        private const val TAG = "DnsVpnService"
        const val ACTION_START = "com.sicloud.dnsswitcher.START_VPN"
        const val ACTION_STOP = "com.sicloud.dnsswitcher.STOP_VPN"
        const val EXTRA_DNS = "extra_dns"

        @Volatile
        var isRunning = false
            private set

        @Volatile
        var activeDns: String? = null
            private set

        // Fallback IP mapping for common providers in case system DNS cannot resolve hostname
        private val KNOWN_IPS = mapOf(
            "dns.quad9.net" to "9.9.9.9",
            "dns.google" to "8.8.8.8",
            "cloudflare-dns.com" to "1.1.1.1",
            "one.one.one.one" to "1.1.1.1",
            "dns.adguard.com" to "94.140.14.14",
            "family.cloudflare-dns.com" to "1.1.1.3"
        )
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private var workerThread: Thread? = null
    private val queryPool = Executors.newFixedThreadPool(6)
    @Volatile
    private var running = false

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent == null) return START_NOT_STICKY

        when (intent.action) {
            ACTION_STOP -> {
                stopTunnel()
                stopSelf()
                return START_NOT_STICKY
            }
            ACTION_START -> {
                val dns = intent.getStringExtra(EXTRA_DNS) ?: "dns.quad9.net"
                startTunnel(dns)
                return START_STICKY
            }
        }
        return START_NOT_STICKY
    }

    private fun startTunnel(dnsHost: String) {
        if (running && activeDns == dnsHost) return
        stopTunnel()

        activeDns = dnsHost
        running = true
        isRunning = true

        startForegroundNotification(dnsHost)

        workerThread = Thread {
            try {
                runVpnLoop(dnsHost)
            } catch (e: Exception) {
                Log.e(TAG, "VPN loop error: ${e.message}", e)
            } finally {
                stopTunnel()
            }
        }.apply {
            name = "DnsVpnWorker"
            start()
        }
    }

    private fun startForegroundNotification(dnsHost: String) {
        val channelId = "dns_vpn_channel"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val chan = NotificationChannel(
                channelId,
                "DNS Switcher Service",
                NotificationManager.IMPORTANCE_LOW
            )
            val nm = getSystemService(NotificationManager::class.java)
            nm?.createNotificationChannel(chan)
        }

        val launchIntent = packageManager.getLaunchIntentForPackage(packageName)
        val pendingIntent = if (launchIntent != null) {
            PendingIntent.getActivity(
                this, 0, launchIntent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
        } else null

        val notificationBuilder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, channelId)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }

        val notif = notificationBuilder
            .setContentTitle("DNS Switcher Active")
            .setContentText("Connected to $dnsHost")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(1001, notif, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
        } else {
            startForeground(1001, notif)
        }
    }

    private fun runVpnLoop(dnsHost: String) {
        val builder = Builder()
            .setSession("Private DNS ($dnsHost)")
            .setMtu(1500)
            .addAddress("10.111.222.1", 30)
            .addDnsServer("10.111.222.2")
            .addRoute("10.111.222.2", 32)
            .setBlocking(true)

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            builder.setMetered(false)
        }

        val pfd = builder.establish() ?: run {
            Log.e(TAG, "Failed to establish VPN interface")
            return
        }
        vpnInterface = pfd

        val inStream = FileInputStream(pfd.fileDescriptor)
        val outStream = FileOutputStream(pfd.fileDescriptor)
        val buffer = ByteArray(32767)

        Log.i(TAG, "DNS VPN started successfully for $dnsHost")

        while (running) {
            val length = try {
                inStream.read(buffer)
            } catch (e: IOException) {
                break
            }
            if (length <= 0) continue

            // Parse IPv4 packet: Version=4 and Length >= 28 (IP 20 + UDP 8)
            if ((buffer[0].toInt() and 0xF0) == 0x40 && length >= 28) {
                val proto = buffer[9].toInt() and 0xFF
                if (proto == 17) { // UDP
                    val dstPort = ((buffer[22].toInt() and 0xFF) shl 8) or (buffer[23].toInt() and 0xFF)
                    if (dstPort == 53) {
                        val srcIp = buffer.copyOfRange(12, 16)
                        val dstIp = buffer.copyOfRange(16, 20)
                        val srcPort = ((buffer[20].toInt() and 0xFF) shl 8) or (buffer[21].toInt() and 0xFF)
                        val udpLen = ((buffer[24].toInt() and 0xFF) shl 8) or (buffer[25].toInt() and 0xFF)
                        val payloadLen = udpLen - 8
                        if (payloadLen > 0 && 28 + payloadLen <= length) {
                            val dnsQuery = buffer.copyOfRange(28, 28 + payloadLen)
                            queryPool.execute {
                                handleDnsQuery(dnsHost, srcIp, dstIp, srcPort, dnsQuery, outStream)
                            }
                        }
                    }
                }
            }
        }
    }

    private fun handleDnsQuery(
        dnsHost: String,
        srcIp: ByteArray,
        dstIp: ByteArray,
        clientPort: Int,
        query: ByteArray,
        outStream: FileOutputStream
    ) {
        val response = resolveDns(dnsHost, query) ?: return
        val replyPacket = buildIpv4UdpPacket(
            srcIp = dstIp,      // 10.111.222.2 (virtual DNS server)
            dstIp = srcIp,      // client IP
            srcPort = 53,
            dstPort = clientPort,
            payload = response
        )
        synchronized(outStream) {
            try {
                outStream.write(replyPacket)
            } catch (e: Exception) {
                Log.w(TAG, "Write back to TUN failed: ${e.message}")
            }
        }
    }

    private fun resolveDns(dnsHost: String, query: ByteArray): ByteArray? {
        // 1. Try DoT (port 853 TLS)
        try {
            return queryDoT(dnsHost, query)
        } catch (e: Exception) {
            Log.d(TAG, "DoT query failed for $dnsHost, fallback to UDP: ${e.message}")
        }

        // 2. Fallback to standard UDP (port 53)
        try {
            return queryUdp(dnsHost, query)
        } catch (e: Exception) {
            Log.w(TAG, "UDP DNS query failed for $dnsHost: ${e.message}")
        }
        return null
    }

    private fun queryDoT(dnsHost: String, query: ByteArray): ByteArray? {
        val rawSocket = Socket()
        protect(rawSocket)
        rawSocket.soTimeout = 2500
        rawSocket.connect(InetSocketAddress(dnsHost, 853), 2500)

        val sslSocket = (SSLSocketFactory.getDefault() as SSLSocketFactory)
            .createSocket(rawSocket, dnsHost, 853, true) as SSLSocket
        sslSocket.soTimeout = 2500
        sslSocket.startHandshake()

        val dos = DataOutputStream(sslSocket.getOutputStream())
        dos.writeShort(query.size)
        dos.write(query)
        dos.flush()

        val dis = DataInputStream(sslSocket.getInputStream())
        val respLen = dis.readUnsignedShort()
        val resp = ByteArray(respLen)
        dis.readFully(resp)
        sslSocket.close()
        return resp
    }

    private fun queryUdp(dnsHost: String, query: ByteArray): ByteArray? {
        val targetIp = resolveTargetIp(dnsHost) ?: return null
        val socket = DatagramSocket()
        protect(socket)
        socket.soTimeout = 2500
        val packet = DatagramPacket(query, query.size, targetIp, 53)
        socket.send(packet)

        val buf = ByteArray(4096)
        val respPacket = DatagramPacket(buf, buf.size)
        socket.receive(respPacket)
        socket.close()
        return buf.copyOf(respPacket.length)
    }

    private fun resolveTargetIp(dnsHost: String): InetAddress? {
        return try {
            InetAddress.getByName(dnsHost)
        } catch (e: Exception) {
            KNOWN_IPS[dnsHost]?.let { InetAddress.getByName(it) }
        }
    }

    private fun buildIpv4UdpPacket(
        srcIp: ByteArray,
        dstIp: ByteArray,
        srcPort: Int,
        dstPort: Int,
        payload: ByteArray
    ): ByteArray {
        val totalLen = 20 + 8 + payload.size
        val pkt = ByteArray(totalLen)

        // IPv4 Header (20 bytes)
        pkt[0] = 0x45.toByte() // IPv4, IHL=5
        pkt[1] = 0.toByte()
        pkt[2] = ((totalLen shr 8) and 0xFF).toByte()
        pkt[3] = (totalLen and 0xFF).toByte()
        pkt[4] = 0.toByte()
        pkt[5] = 0.toByte()
        pkt[6] = 0x40.toByte() // Don't Fragment
        pkt[7] = 0.toByte()
        pkt[8] = 64.toByte()   // TTL
        pkt[9] = 17.toByte()   // UDP
        System.arraycopy(srcIp, 0, pkt, 12, 4)
        System.arraycopy(dstIp, 0, pkt, 16, 4)

        val csum = computeChecksum(pkt, 0, 20)
        pkt[10] = ((csum shr 8) and 0xFF).toByte()
        pkt[11] = (csum and 0xFF).toByte()

        // UDP Header (8 bytes)
        val udpLen = 8 + payload.size
        pkt[20] = ((srcPort shr 8) and 0xFF).toByte()
        pkt[21] = (srcPort and 0xFF).toByte()
        pkt[22] = ((dstPort shr 8) and 0xFF).toByte()
        pkt[23] = (dstPort and 0xFF).toByte()
        pkt[24] = ((udpLen shr 8) and 0xFF).toByte()
        pkt[25] = (udpLen and 0xFF).toByte()
        pkt[26] = 0.toByte() // Optional checksum in IPv4
        pkt[27] = 0.toByte()

        // Payload
        System.arraycopy(payload, 0, pkt, 28, payload.size)
        return pkt
    }

    private fun computeChecksum(data: ByteArray, offset: Int, length: Int): Int {
        var sum = 0
        var i = offset
        while (i < offset + length) {
            val b1 = data[i].toInt() and 0xFF
            val b2 = if (i + 1 < offset + length) data[i + 1].toInt() and 0xFF else 0
            sum += (b1 shl 8) or b2
            i += 2
        }
        while (sum shr 16 > 0) {
            sum = (sum and 0xFFFF) + (sum shr 16)
        }
        return sum.inv() and 0xFFFF
    }

    private fun stopTunnel() {
        running = false
        isRunning = false
        activeDns = null
        try {
            vpnInterface?.close()
        } catch (e: Exception) {
            // ignore
        }
        vpnInterface = null
        stopForeground(true)
    }

    override fun onDestroy() {
        stopTunnel()
        queryPool.shutdownNow()
        super.onDestroy()
    }
}
