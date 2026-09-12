package org.linmic.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.util.AttributeSet
import android.view.View

class WaveformView(context: Context, attrs: AttributeSet?) : View(context, attrs) {
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { color=Color.rgb(0,155,126); strokeWidth=3f; strokeCap=Paint.Cap.ROUND }
    private var data = FloatArray(144)
    fun update(values: FloatArray) { data=values; invalidate() }
    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val center = height/2f
        for (i in 0 until 128) { val x=(i+0.5f)*width/128f; val amplitude=data.getOrElse(i+16){0f}.coerceIn(0f,1f)*height*0.48f; canvas.drawLine(x,center-amplitude-1,x,center+amplitude+1,paint) }
        val peak=data.getOrElse(1){0f}
        paint.color=if(peak>=0.89f) Color.rgb(203,69,69) else Color.rgb(0,155,126)
    }
}
