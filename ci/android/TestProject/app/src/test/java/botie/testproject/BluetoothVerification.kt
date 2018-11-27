package botie.testproject

import org.junit.Test

class BluetoothVerification {

  @Test
  fun native_bluetooth_init() {
    Interface().apply { runTests() }
  }
}
