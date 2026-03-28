package dev.mineplay.agent.session

import android.content.Context
import dev.mineplay.agent.R

object SessionController {
    fun describeStartupState(context: Context): String {
        return context.getString(R.string.startup_state_text)
    }
}
