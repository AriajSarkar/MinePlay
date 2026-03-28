package dev.mineplay.agent

import android.app.Activity
import android.content.Intent
import android.media.projection.MediaProjectionManager
import android.os.Bundle
import android.provider.Settings
import android.widget.Button
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import dev.mineplay.agent.projection.ProjectionService
import dev.mineplay.agent.session.SessionController

class MainActivity : AppCompatActivity() {
    private lateinit var statusView: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        statusView = findViewById(R.id.statusText)
        findViewById<Button>(R.id.startProjectionButton).setOnClickListener {
            requestProjection()
        }
        findViewById<Button>(R.id.openAccessibilityButton).setOnClickListener {
            startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
        }

        statusView.text = SessionController.describeStartupState(this)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != REQUEST_MEDIA_PROJECTION || resultCode != Activity.RESULT_OK || data == null) {
            return
        }

        val serviceIntent = ProjectionService.startIntent(this, resultCode, data)
        startForegroundService(serviceIntent)
        statusView.text = "Projection consent granted. Service started."
    }

    private fun requestProjection() {
        val manager = getSystemService(MediaProjectionManager::class.java)
        startActivityForResult(manager.createScreenCaptureIntent(), REQUEST_MEDIA_PROJECTION)
    }

    private companion object {
        private const val REQUEST_MEDIA_PROJECTION = 1001
    }
}
