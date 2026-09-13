package org.linmic.app

import android.Manifest
import android.app.Activity
import android.app.AlertDialog
import android.content.*
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.net.Uri
import android.os.*
import android.provider.Settings
import android.text.InputFilter
import android.text.InputType
import android.view.View
import android.view.WindowManager
import android.widget.*
import kotlin.math.log10

class MainActivity : Activity() {
    override fun attachBaseContext(base: Context) { super.attachBaseContext(AppLocales.wrap(base)) }
    private lateinit var settings: SettingsRepository
    private lateinit var discovery: DeviceDiscoveryManager
    private val handler=Handler(Looper.getMainLooper())
    private var selectedTab=0
    private var inputs=emptyList<AudioInputs.Entry>()
    private var batteryExempt=false
    private var notificationAsked=false
    private var textUpdated=0L
    private var renderedState: StreamState?=null
    private var renderedMute=false
    private val profiles=listOf("ultra","balanced","stable")
    private val profileHints=intArrayOf(R.string.profile_ultra_help,R.string.profile_balanced_help,R.string.profile_stable_help)
    private val update=object:Runnable { override fun run() { render(); handler.postDelayed(this,125) } }
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
        notificationAsked=getSharedPreferences("settings",MODE_PRIVATE).getBoolean("notificationAsked",false)
        view<EditText>(R.id.host).setText(settings.host);view<EditText>(R.id.port).setText(settings.port.toString())
        listOf(R.id.tab_mic,R.id.tab_connection,R.id.tab_health).forEachIndexed { i,id -> view<Button>(id).setOnClickListener { tab(i) } }
        view<Button>(R.id.manual_toggle).setOnClickListener { val fields=view<View>(R.id.manual_fields);fields.visibility=if(fields.visibility==View.VISIBLE)View.GONE else View.VISIBLE }
        view<Spinner>(R.id.profile).apply {
            adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,listOf(getString(R.string.profile_ultra),getString(R.string.profile_balanced),getString(R.string.profile_stable)))
            setSelection(profiles.indexOf(settings.profile).coerceAtLeast(0))
            onItemSelectedListener=selection { if(!active())settings.profile=profiles[it];view<TextView>(R.id.profile_help).setText(profileHints[it]) }
        }
        view<Switch>(R.id.compatible).apply { isChecked=settings.compatible;setOnCheckedChangeListener { _,enabled -> settings.compatible=enabled } }
        refreshInputs()
        view<SeekBar>(R.id.gain).apply {
            progress=(settings.gain+60).toInt()
            setOnSeekBarChangeListener(object:SeekBar.OnSeekBarChangeListener {
                override fun onProgressChanged(bar:SeekBar?,value:Int,fromUser:Boolean) { if(fromUser)settings.gain=value-60f;gainLabel() }
                override fun onStartTrackingTouch(bar:SeekBar?) {}
                override fun onStopTrackingTouch(bar:SeekBar?) {}
            })
        }
        listOf(R.id.gain_reset to 0f,R.id.gain_six to 6f,R.id.gain_twelve to 12f).forEach { (id,value) -> view<Button>(id).setOnClickListener { settings.gain=value;view<SeekBar>(R.id.gain).progress=(value+60).toInt();gainLabel() } }
        gainLabel()
        view<Button>(R.id.connect).setOnClickListener {
            if(active()) serviceAction("stop") else connect()
        }
        view<Button>(R.id.mute).setOnClickListener { if(active())serviceAction("mute") }
        view<Button>(R.id.repair).setOnClickListener { if(active())serviceAction("repair") else connect() }
        view<Button>(R.id.settings).setOnClickListener { settingsDialog() }
        view<Button>(R.id.show_report).setOnClickListener { showReport() }
        view<Button>(R.id.battery_settings).setOnClickListener {
            val intent=Intent(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS)
            runCatching { startActivity(intent) }.onFailure { startActivity(Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:$packageName"))) }
        }
        discovery=DeviceDiscoveryManager(this) { computers ->
            if(!active())view<Spinner>(R.id.computers).apply {
                onItemSelectedListener=null
                adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,listOf(getString(R.string.manual))+computers.map{it.serviceName})
                val address=view<EditText>(R.id.host).text.toString()
                setSelection((computers.indexOfFirst{it.host?.hostAddress==address}+1).coerceAtLeast(0))
                onItemSelectedListener=selection { if(it>0){view<EditText>(R.id.host).setText(computers[it-1].host?.hostAddress ?: "");view<EditText>(R.id.port).setText(computers[it-1].port.toString())} }
            }
        }
        view<Spinner>(R.id.computers).adapter=ArrayAdapter(this,android.R.layout.simple_spinner_dropdown_item,listOf(settings.host.ifEmpty{getString(R.string.manual)}))
        tab(savedInstanceState?.getInt("tab") ?: if(settings.host.isEmpty())1 else 0)
    }
    override fun onSaveInstanceState(outState:Bundle){outState.putInt("tab",selectedTab);super.onSaveInstanceState(outState)}
    private fun tab(index:Int) {
        selectedTab=index
        textUpdated=0
        listOf(R.id.panel_mic,R.id.panel_connection,R.id.panel_health).forEachIndexed { i,id -> view<View>(id).visibility=if(i==index)View.VISIBLE else View.GONE }
        listOf(R.id.tab_mic,R.id.tab_connection,R.id.tab_health).forEachIndexed { i,id -> view<Button>(id).apply { isSelected=i==index;setBackgroundResource(if(i==index)R.drawable.soft_card else android.R.color.transparent);alpha=if(i==index)1f else 0.65f;setTypeface(null,if(i==index)android.graphics.Typeface.BOLD else android.graphics.Typeface.NORMAL) } }
        view<ScrollView>(R.id.scroll).scrollTo(0,0)
    }
    private val inputCallback=object:android.media.AudioDeviceCallback() {
        override fun onAudioDevicesAdded(devices:Array<out AudioDeviceInfo>) { refreshInputs() }
        override fun onAudioDevicesRemoved(devices:Array<out AudioDeviceInfo>) { refreshInputs() }
    }
    private fun refreshInputs() {
        inputs=AudioInputs.list(this)
        view<Spinner>(R.id.microphone).apply {
            onItemSelectedListener=null
            adapter=ArrayAdapter(this@MainActivity,android.R.layout.simple_spinner_dropdown_item,inputs.map{it.label})
            setSelection(inputs.indexOfFirst{it.id==settings.inputDevice}.coerceAtLeast(0))
            onItemSelectedListener=selection { index ->
                if(!active())settings.inputDevice=inputs[index].id
                if(inputs[index].bluetooth)Toast.makeText(this@MainActivity,R.string.bluetooth,Toast.LENGTH_LONG).show()
            }
        }
    }
    private fun selection(callback:(Int)->Unit)=object:AdapterView.OnItemSelectedListener {
        override fun onItemSelected(parent:AdapterView<*>?,v:View?,position:Int,id:Long){callback(position)}
        override fun onNothingSelected(parent:AdapterView<*>?){}
    }
    private fun active() = StreamModel.state !is StreamState.Idle && StreamModel.state !is StreamState.Error
    private fun serviceAction(action:String){startService(Intent(this,StreamingForegroundService::class.java).setAction(action))}
    private fun gainLabel(){view<TextView>(R.id.gain_label).text=getString(R.string.gain,settings.gain)}
    private fun connect() {
        if(checkSelfPermission(Manifest.permission.RECORD_AUDIO)!=PackageManager.PERMISSION_GRANTED) { requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO),1);return }
        val host=view<EditText>(R.id.host).text.toString().trim()
        val port=view<EditText>(R.id.port).text.toString().toIntOrNull()
        if(host.isBlank()||port==null||port !in 1..65535){tab(1);view<View>(R.id.manual_fields).visibility=View.VISIBLE;view<EditText>(R.id.host).error=getString(R.string.invalid_host);return}
        settings.host=host;settings.port=port
        if(Build.VERSION.SDK_INT>=33&&!notificationAsked&&checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)!=PackageManager.PERMISSION_GRANTED){
            notificationAsked=true;getSharedPreferences("settings",MODE_PRIVATE).edit().putBoolean("notificationAsked",true).apply()
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS),2);return
        }
        val pin=settings.fingerprint(host)
        if(pin.isEmpty() || settings.token(host).isEmpty() || (StreamModel.state as? StreamState.Error)?.message?.contains("pair",true)==true) {
            val box=LinearLayout(this).apply { orientation=LinearLayout.VERTICAL;val pad=(24*resources.displayMetrics.density).toInt();setPadding(pad,pad,pad,pad) }
            box.addView(TextView(this).apply{text=getString(R.string.pair_help);textSize=16f})
            val code=EditText(this).apply{hint=getString(R.string.code);inputType=InputType.TYPE_CLASS_NUMBER;filters=arrayOf(InputFilter.LengthFilter(6));textSize=30f;gravity=android.view.Gravity.CENTER;setSelectAllOnFocus(true)}
            box.addView(code)
            val dialog=AlertDialog.Builder(this).setTitle(R.string.pair).setView(box).setNegativeButton(R.string.cancel,null).setPositiveButton(R.string.connect,null).create()
            dialog.setOnShowListener{dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                if(!code.text.toString().matches(Regex("[0-9]{6}"))){code.error=getString(R.string.code);return@setOnClickListener}
                begin(pin,code.text.toString());dialog.dismiss()
            }};dialog.show()
        } else begin(pin,"")
    }
    private fun begin(pin:String,code:String) {
        tab(0)
        startForegroundService(Intent(this,StreamingForegroundService::class.java).putExtra("fingerprint",pin).putExtra("code",code))
    }
    override fun onRequestPermissionsResult(requestCode:Int,permissions:Array<out String>,grants:IntArray) {
        super.onRequestPermissionsResult(requestCode,permissions,grants)
        if(requestCode==1) {if(grants.firstOrNull()==PackageManager.PERMISSION_GRANTED)connect() else Toast.makeText(this,R.string.permission_required,Toast.LENGTH_LONG).show()}
        if(requestCode==2)connect()
    }
    private fun render() {
        if(selectedTab==0)view<WaveformView>(R.id.waveform).update(StreamModel.meter)
        val now=SystemClock.elapsedRealtime()
        if(now-textUpdated<1000 && renderedState==StreamModel.state && renderedMute==StreamModel.muted)return
        textUpdated=now;renderedState=StreamModel.state;renderedMute=StreamModel.muted
        val meter=StreamModel.meter;val server=StreamModel.server;val health=StreamModel.health;val running=active()
        val problem=health in listOf(CaptureMonitor.Health.SYSTEM_BLOCKED,CaptureMonitor.Health.CAPTURE_STALLED,CaptureMonitor.Health.SEND_STALLED,CaptureMonitor.Health.DELIVERY_STALLED,CaptureMonitor.Health.DIGITAL_SILENCE)
        val status=when(val state=StreamModel.state){
            StreamState.Idle->getString(R.string.idle);StreamState.Connecting->getString(R.string.connecting)
            StreamState.Streaming->getString(when(health){CaptureMonitor.Health.SYSTEM_BLOCKED->R.string.system_blocked;CaptureMonitor.Health.DIGITAL_SILENCE->R.string.digital_silence;CaptureMonitor.Health.CAPTURE_STALLED,CaptureMonitor.Health.SEND_STALLED,CaptureMonitor.Health.DELIVERY_STALLED->R.string.audio_stalled;CaptureMonitor.Health.QUIET->R.string.streaming;else->R.string.streaming})
            StreamState.Muted->getString(R.string.muted)
            is StreamState.Reconnecting->getString(R.string.reconnecting,state.attempt)
            is StreamState.Error->getString(R.string.error)+": "+state.message
        }
        view<TextView>(R.id.status).apply{if(text.toString()!=status)text=status}
        val elapsed=if(running)(SystemClock.elapsedRealtime()-StreamModel.startedAt).coerceAtLeast(0)/1000 else 0
        view<TextView>(R.id.session_route).text=if(settings.host.isEmpty())getString(R.string.no_pc) else getString(R.string.session_route,settings.host,String.format(java.util.Locale.ROOT,"%02d:%02d",elapsed/60,elapsed%60))
        view<Button>(R.id.connect).setText(if(running)R.string.disconnect else R.string.connect)
        view<Button>(R.id.mute).apply{isEnabled=StreamModel.state is StreamState.Streaming||StreamModel.state is StreamState.Muted;alpha=if(isEnabled)1f else 0.4f;setText(if(StreamModel.muted)R.string.unmute else R.string.mute)}
        listOf(R.id.profile,R.id.microphone,R.id.computers,R.id.host,R.id.port,R.id.compatible).forEach{view<View>(it).isEnabled=!running}
        val peak=20*log10(meter[1].coerceAtLeast(0.000001f))
        view<TextView>(R.id.level).text=if(running&&!problem)getString(R.string.level_reading,peak) else "—"
        view<TextView>(R.id.level_hint).setText(when{!running||StreamModel.muted->R.string.level_ready;peak >= -2->R.string.level_high;peak < -35->R.string.level_low;else->R.string.level_good})
        view<TextView>(R.id.quick_stats).visibility=if(running)View.VISIBLE else View.GONE
        view<TextView>(R.id.quick_stats).text=if(running)getString(R.string.quick_stats,server.optDouble("estimated_latency_ms",0.0),server.optDouble("loss_percent",0.0)) else getString(R.string.intro)
        view<View>(R.id.recovery_card).visibility=if(running&&(problem||StreamModel.state is StreamState.Reconnecting)||StreamModel.state is StreamState.Error)View.VISIBLE else View.GONE
        view<TextView>(R.id.recovery_help).setText(if(health==CaptureMonitor.Health.SYSTEM_BLOCKED)R.string.blocked_help else R.string.recovery_help)
        view<TextView>(R.id.current_input).text=getString(R.string.input_active,inputs.firstOrNull{it.id==meter[11].toInt()&&it.id!=0}?.label ?: inputs.firstOrNull{it.id==settings.inputDevice}?.label ?: getString(R.string.input_auto))
        view<TextView>(R.id.background_status).setText(if(batteryExempt)R.string.battery_ok else R.string.battery_restricted)
        if(selectedTab==2){
            view<TextView>(R.id.stats).text=getString(R.string.stats,server.optDouble("estimated_latency_ms",0.0),server.optDouble("rtt_ms",0.0),server.optDouble("jitter_ms",0.0),server.optDouble("loss_percent",0.0),server.optDouble("jitter_buffer_ms",0.0),20*log10(meter[0].coerceAtLeast(0.000001f)),peak)
            view<TextView>(R.id.diagnostics).text=getString(R.string.diagnostic_values,meter[2],meter[3],meter[4],meter[5],meter[8],meter[9],meter[10],meter[11],meter[12])
        }
        if(selectedTab==0)view<WaveformView>(R.id.waveform).update(meter)
    }
    private fun showReport() {
        val report=SessionDiagnostics(this).report()
        val text=TextView(this).apply{text=getString(R.string.report_help)+"\n\n"+report;setTextIsSelectable(true);setPadding(32,24,32,24);textSize=13f}
        val scroll=ScrollView(this).apply{addView(text)}
        AlertDialog.Builder(this).setTitle(R.string.show_report).setView(scroll).setPositiveButton(R.string.copy_report){_,_->
            getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("LinMic",report));Toast.makeText(this,R.string.copied,Toast.LENGTH_SHORT).show()
        }.setNegativeButton(R.string.done,null).show()
    }
    private fun settingsDialog() {
        val labels=arrayOf(getString(R.string.settings),getString(R.string.language),getString(R.string.theme),getString(R.string.app_name),getString(R.string.forget))
        AlertDialog.Builder(this).setTitle(R.string.settings).setItems(labels){_,choice->when(choice){
            0->{val ids=intArrayOf(R.string.auto_reconnect,R.string.wifi_latency,R.string.ns,R.string.agc,R.string.keep_screen)
                val values=booleanArrayOf(settings.reconnect,settings.wifiLowLatency,settings.systemNs,settings.systemAgc,window.attributes.flags and WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON!=0)
                AlertDialog.Builder(this).setTitle(R.string.settings).setMultiChoiceItems(ids.map{getString(it)}.toTypedArray(),values){_,i,v->when(i){0->settings.reconnect=v;1->settings.wifiLowLatency=v;2->settings.systemNs=v;3->settings.systemAgc=v;4->if(v)window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)else window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}}.setPositiveButton(R.string.done,null).show()}
            1->AlertDialog.Builder(this).setTitle(R.string.language).setSingleChoiceItems(arrayOf(getString(R.string.system))+AppLocales.names.drop(1),AppLocales.tags.indexOf(AppLocales.selected(this))){d,i->AppLocales.save(this,AppLocales.tags[i]);d.dismiss();recreate()}.show()
            2->AlertDialog.Builder(this).setTitle(R.string.theme).setSingleChoiceItems(arrayOf(getString(R.string.system),getString(R.string.dark),getString(R.string.light)),settings.theme){d,i->settings.theme=i;d.dismiss();recreate()}.show()
            3->AlertDialog.Builder(this).setTitle(R.string.app_name).setMessage(getString(R.string.about)+"\n\n"+resources.openRawResource(R.raw.licenses).bufferedReader().use{it.readText()}).setPositiveButton(R.string.done,null).show()
            4->if(!active())AlertDialog.Builder(this).setTitle(R.string.forget).setMessage(settings.host).setPositiveButton(R.string.forget){_,_->settings.forget(settings.host)}.setNegativeButton(R.string.cancel,null).show() else Toast.makeText(this,R.string.disconnect,Toast.LENGTH_SHORT).show()
        }}.show()
    }
    override fun onResume(){super.onResume();getSystemService(AudioManager::class.java).registerAudioDeviceCallback(inputCallback,handler);StreamModel.activityVisible=true;batteryExempt=getSystemService(PowerManager::class.java).isIgnoringBatteryOptimizations(packageName);discovery.start();handler.post(update)}
    override fun onPause(){getSystemService(AudioManager::class.java).unregisterAudioDeviceCallback(inputCallback);StreamModel.activityVisible=false;discovery.stop();handler.removeCallbacks(update);super.onPause()}
}
