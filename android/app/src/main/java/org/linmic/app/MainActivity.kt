package org.linmic.app

import android.Manifest
import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.*
import android.text.InputType
import android.view.View
import android.view.WindowManager
import android.widget.*
import kotlin.math.log10

class MainActivity : Activity() {
    override fun attachBaseContext(base: android.content.Context) { super.attachBaseContext(AppLocales.wrap(base)) }
    private lateinit var settings: SettingsRepository
    private lateinit var discovery: DeviceDiscoveryManager
    private val handler=Handler(Looper.getMainLooper())
    private var diagnostics=false
    private val update=object:Runnable { override fun run() { render(); handler.postDelayed(this,100) } }
    private fun <T:View> view(id:Int): T = findViewById(id)
    override fun onCreate(savedInstanceState: Bundle?) {
        settings=SettingsRepository(this)
        val dark=settings.theme==1 || (settings.theme==0 && resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES)
        setTheme(if(dark) R.style.AppThemeDark else R.style.AppTheme)
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        view<View>(R.id.root).setOnApplyWindowInsetsListener { v,insets ->
            @Suppress("DEPRECATION")
            v.setPadding(insets.systemWindowInsetLeft,insets.systemWindowInsetTop,insets.systemWindowInsetRight,insets.systemWindowInsetBottom)
            insets
        }
        view<EditText>(R.id.host).setText(settings.host);view<EditText>(R.id.port).setText(settings.port.toString())
        val profiles=listOf("ultra","balanced","stable")
        view<Spinner>(R.id.profile).apply {
            adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,profiles)
            setSelection(profiles.indexOf(settings.profile).coerceAtLeast(0))
            onItemSelectedListener=selection { settings.profile=profiles[it] }
        }
        refreshInputs()
        view<SeekBar>(R.id.gain).apply {
            progress=(settings.gain+60).toInt()
            setOnSeekBarChangeListener(object:SeekBar.OnSeekBarChangeListener {
                override fun onProgressChanged(bar:SeekBar?,value:Int,fromUser:Boolean) { if(fromUser){settings.gain=value-60f;StreamModel.native.gain(settings.gain)};gainLabel() }
                override fun onStartTrackingTouch(bar:SeekBar?) {}
                override fun onStopTrackingTouch(bar:SeekBar?) {}
            })
        }
        gainLabel()
        view<Button>(R.id.connect).setOnClickListener {
            if(active()) startService(Intent(this,StreamingForegroundService::class.java).setAction("stop")) else connect()
        }
        view<Button>(R.id.mute).setOnClickListener { startService(Intent(this,StreamingForegroundService::class.java).setAction("mute")) }
        view<Button>(R.id.settings).setOnClickListener { settingsDialog() }
        discovery=DeviceDiscoveryManager(this) { computers ->
            view<Spinner>(R.id.computers).apply {
                adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,listOf(getString(R.string.manual))+computers.map{it.serviceName})
                onItemSelectedListener=selection { if(it>0){view<EditText>(R.id.host).setText(computers[it-1].host?.hostAddress ?: "");view<EditText>(R.id.port).setText(computers[it-1].port.toString())} }
            }
        }
    }
    private val inputCallback=object:android.media.AudioDeviceCallback() {
        override fun onAudioDevicesAdded(devices:Array<out AudioDeviceInfo>) { refreshInputs() }
        override fun onAudioDevicesRemoved(devices:Array<out AudioDeviceInfo>) { refreshInputs() }
    }
    private fun refreshInputs() {
        val entries=AudioInputs.list(this)
        view<Spinner>(R.id.microphone).apply {
            adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,entries.map{it.label})
            setSelection(entries.indexOfFirst{it.id==settings.inputDevice}.coerceAtLeast(0))
            onItemSelectedListener=selection { index ->
                settings.inputDevice=entries[index].id
                if(entries[index].bluetooth)Toast.makeText(this@MainActivity,R.string.bluetooth,Toast.LENGTH_LONG).show()
            }
        }
    }
    private fun selection(callback:(Int)->Unit)=object:AdapterView.OnItemSelectedListener {
        override fun onItemSelected(parent:AdapterView<*>?,v:View?,position:Int,id:Long){callback(position)}
        override fun onNothingSelected(parent:AdapterView<*>?){}
    }
    private fun active() = StreamModel.state !is StreamState.Idle && StreamModel.state !is StreamState.Error
    private fun gainLabel(){view<TextView>(R.id.gain_label).text=getString(R.string.gain,settings.gain)}
    private fun connect() {
        if(checkSelfPermission(Manifest.permission.RECORD_AUDIO)!=PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO),1);return }
        val host=view<EditText>(R.id.host).text.toString().trim()
        val port=view<EditText>(R.id.port).text.toString().toIntOrNull()
        if(host.isBlank()||port==null||port !in 1..65535){Toast.makeText(this,R.string.invalid_host,Toast.LENGTH_LONG).show();return}
        settings.host=host;settings.port=port
        if(Build.VERSION.SDK_INT>=33&&checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)!=PackageManager.PERMISSION_GRANTED)requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS),2)
        val pin=settings.fingerprint(host)
        if(pin.isEmpty() || settings.token(host).isEmpty() || (StreamModel.state as? StreamState.Error)?.message?.contains("pair",true)==true) {
            val box=LinearLayout(this).apply { orientation=LinearLayout.VERTICAL;setPadding(32,16,32,16) }
            box.addView(TextView(this).apply{text=getString(R.string.pair_help)})
            val code=EditText(this).apply{hint=getString(R.string.code);inputType=InputType.TYPE_CLASS_NUMBER;minHeight=96}
            box.addView(code)
            val dialog=AlertDialog.Builder(this).setTitle(R.string.pair).setView(box).setNegativeButton(R.string.cancel,null).setPositiveButton(R.string.connect,null).create()
            dialog.setOnShowListener{dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                if(!code.text.toString().matches(Regex("[0-9]{6}"))){code.error=getString(R.string.code);return@setOnClickListener}
                begin(pin,code.text.toString());dialog.dismiss()
            }};dialog.show()
        } else begin(pin,"")
    }
    private fun begin(pin:String,code:String) { startForegroundService(Intent(this,StreamingForegroundService::class.java).putExtra("fingerprint",pin).putExtra("code",code)) }
    override fun onRequestPermissionsResult(requestCode:Int,permissions:Array<out String>,grants:IntArray) {
        super.onRequestPermissionsResult(requestCode,permissions,grants)
        if(requestCode==1) {if(grants.firstOrNull()==PackageManager.PERMISSION_GRANTED)connect() else Toast.makeText(this,R.string.permission_required,Toast.LENGTH_LONG).show()}
    }
    private fun render() {
        val meter=StreamModel.meter;val server=StreamModel.server
        view<TextView>(R.id.status).text=when(val state=StreamModel.state){
            StreamState.Idle->getString(R.string.idle);StreamState.Connecting->getString(R.string.connecting)
            StreamState.Streaming->getString(if(meter[0]<0.001f)R.string.silent else R.string.streaming)
            StreamState.Muted->getString(R.string.muted)
            is StreamState.Reconnecting->getString(R.string.reconnecting,state.attempt)
            is StreamState.Error->getString(R.string.error)+": "+state.message
        }
        view<Button>(R.id.connect).setText(if(active())R.string.disconnect else R.string.connect)
        view<Button>(R.id.mute).apply{isEnabled=active();setText(if(StreamModel.muted)R.string.unmute else R.string.mute)}
        view<Spinner>(R.id.profile).isEnabled=!active();view<Spinner>(R.id.microphone).isEnabled=!active()
        view<TextView>(R.id.stats).text=getString(R.string.stats,server.optDouble("estimated_latency_ms",0.0),server.optDouble("rtt_ms",0.0),server.optDouble("jitter_ms",0.0),server.optDouble("loss_percent",0.0),server.optDouble("jitter_buffer_ms",0.0),20*log10(meter[0].coerceAtLeast(0.000001f)),20*log10(meter[1].coerceAtLeast(0.000001f)))
        view<WaveformView>(R.id.waveform).update(meter)
        view<TextView>(R.id.diagnostics).apply{visibility=if(diagnostics)View.VISIBLE else View.GONE;text=getString(R.string.diagnostic_values,meter[2],meter[3],meter[4],meter[5],meter[8],meter[9],meter[10],meter[11],meter[12])}
    }
    private fun settingsDialog() {
        val labels=intArrayOf(R.string.auto_reconnect,R.string.wifi_latency,R.string.ns,R.string.agc,R.string.advanced_stats,R.string.keep_screen)
        val values=booleanArrayOf(settings.reconnect,settings.wifiLowLatency,settings.systemNs,settings.systemAgc,diagnostics,window.attributes.flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON != 0)
        AlertDialog.Builder(this).setTitle(R.string.settings).setMultiChoiceItems(labels.map{getString(it)}.toTypedArray(),values){_,i,value->when(i){0->settings.reconnect=value;1->settings.wifiLowLatency=value;2->settings.systemNs=value;3->settings.systemAgc=value;4->diagnostics=value;5->if(value)window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}}
            .setPositiveButton(R.string.done,null).setNegativeButton(R.string.forget){_,_->if(!active())settings.forget(settings.host)}
            .setNeutralButton(R.string.appearance){_,_->
                AlertDialog.Builder(this).setItems(arrayOf(getString(R.string.language),getString(R.string.theme),getString(R.string.app_name))){_,choice->
                    when(choice){
                        0->AlertDialog.Builder(this).setTitle(R.string.language).setSingleChoiceItems(AppLocales.names,AppLocales.tags.indexOf(AppLocales.selected(this))){d,i->AppLocales.save(this,AppLocales.tags[i]);d.dismiss();recreate()}.show()
                        1->AlertDialog.Builder(this).setTitle(R.string.theme).setSingleChoiceItems(arrayOf(getString(R.string.system),getString(R.string.dark),getString(R.string.light)),settings.theme){d,i->settings.theme=i;d.dismiss();recreate()}.show()
                        else->AlertDialog.Builder(this).setTitle(R.string.app_name).setMessage(getString(R.string.about)+"\n\n"+resources.openRawResource(R.raw.licenses).bufferedReader().use{it.readText()}).setPositiveButton(R.string.done,null).show()
                    }
                }.show()
            }.show()
    }
    override fun onResume(){super.onResume();getSystemService(AudioManager::class.java).registerAudioDeviceCallback(inputCallback,handler);StreamModel.activityVisible=true;discovery.start();handler.post(update)}
    override fun onPause(){getSystemService(AudioManager::class.java).unregisterAudioDeviceCallback(inputCallback);StreamModel.activityVisible=false;discovery.stop();handler.removeCallbacks(update);super.onPause()}
}
