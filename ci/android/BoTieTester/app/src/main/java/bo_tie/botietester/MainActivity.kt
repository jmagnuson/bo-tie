package bo_tie.botietester

import android.support.v7.app.AppCompatActivity
import android.os.Bundle
import kotlin.concurrent.thread
import kotlinx.android.synthetic.main.activity_main.*
import java.io.File
import java.io.SequenceInputStream
import java.nio.charset.Charset

class MainActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        textView.text = getString(R.string.text_placeholder)

        thread {
            textView.text = runTests("bo_tie_tests")
        }
    }
}

fun AppCompatActivity.runTests(native: String, vararg args: String = arrayOf("") ): String {
    val fileName = "bo_tie.tests"

    File(filesDir.path + fileName ).apply {
        setExecutable(true)
        setReadable(true)
        setWritable(true)

    }.appendBytes(assets.open( native ).bufferedReader().use { it.readText() }.toByteArray())

    return ProcessBuilder(fileName, *args).start().run {
        SequenceInputStream(inputStream, errorStream).reader(Charset.defaultCharset()).use { it.readText() }
    }
}