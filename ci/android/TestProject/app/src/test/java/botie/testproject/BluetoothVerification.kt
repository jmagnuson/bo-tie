package botie.testproject

import org.junit.Test
import org.junit.Assert.*

class BluetoothVerification {

  @Test
  fun native_bluetooth_init() {
    Interface().apply { runTests() }
  }
}
