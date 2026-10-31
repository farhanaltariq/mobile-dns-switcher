package dev.dioxus.main

import android.content.Intent
import com.sicloud.dnsswitcher.VpnHelper

typealias BuildConfig = com.sicloud.dnsswitcher.BuildConfig

class MainActivity : WryActivity() {
    override fun onResume() {
        super.onResume()
        VpnHelper.currentActivity = this
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == VpnHelper.VPN_REQUEST_CODE && resultCode == RESULT_OK) {
            val dns = VpnHelper.pendingDns ?: return
            VpnHelper.startService(this, dns)
        }
    }
}
