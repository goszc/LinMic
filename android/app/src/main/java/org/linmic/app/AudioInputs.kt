package org.linmic.app

import android.content.Context
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build

object AudioInputs {
    data class Entry(val id: Int,val label: String,val bluetooth: Boolean)
    fun list(context: Context): List<Entry> {
        val devices=context.getSystemService(AudioManager::class.java).getDevices(AudioManager.GET_DEVICES_INPUTS)
        val names=devices.map { device ->
            val type=when(device.type){
                AudioDeviceInfo.TYPE_BUILTIN_MIC -> context.getString(R.string.input_phone)
                AudioDeviceInfo.TYPE_WIRED_HEADSET -> context.getString(R.string.input_wired)
                AudioDeviceInfo.TYPE_USB_DEVICE,AudioDeviceInfo.TYPE_USB_ACCESSORY,AudioDeviceInfo.TYPE_USB_HEADSET -> context.getString(R.string.input_usb)
                AudioDeviceInfo.TYPE_BLUETOOTH_SCO -> context.getString(R.string.input_bt)
                AudioDeviceInfo.TYPE_BLE_HEADSET -> context.getString(R.string.input_ble)
                AudioDeviceInfo.TYPE_TELEPHONY -> context.getString(R.string.input_call)
                AudioDeviceInfo.TYPE_LINE_ANALOG -> context.getString(R.string.input_analog)
                AudioDeviceInfo.TYPE_LINE_DIGITAL -> context.getString(R.string.input_digital)
                AudioDeviceInfo.TYPE_REMOTE_SUBMIX -> context.getString(R.string.input_system)
                else -> context.getString(R.string.input_unknown)
            }
            val product=device.productName?.toString()?.trim().orEmpty()
            val generic=product.isBlank()||product.equals("Android",true)||product.equals("default",true)
            val name=if(generic)type else "$type — $product"
            Entry(device.id,name,device.type==AudioDeviceInfo.TYPE_BLUETOOTH_SCO||device.type==AudioDeviceInfo.TYPE_BLE_HEADSET)
        }
        // Android does not reliably expose individual bottom/top capsules. Never invent positions.
        val counts=names.groupingBy { it.label }.eachCount()
        return listOf(Entry(0,context.getString(R.string.input_auto),false))+names.map {
            if(counts[it.label]!!>1)it.copy(label="${it.label} · #${it.id}")else it
        }
    }
}
